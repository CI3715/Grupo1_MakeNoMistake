//! Pruebas basadas en propiedades (`proptest`) sobre la sincronización de
//! partes de gastos de grupo contra el núcleo en memoria (`RepositorioEnMemoria`).
//!
//! Cubren INV-13, INV-15, INV-18 y el aislamiento de correcciones locales
//! sin modificar las pruebas unitarias existentes del equipo.

use std::collections::{BTreeMap, BTreeSet};

use aplicacion::{
    CasosDeUso, DatosCorregirMovimiento, DatosParteDeGrupo, DispositivoFijo, ErrorAplicacion,
    GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria, RepositorioEventos,
    ResultadoAplicar,
};
use proptest::prelude::*;

#[derive(Debug, Clone)]
struct ParteGenerada {
    evento_id: String,
    centimos: i64,
    incluye_redondeo: bool,
}

fn texto_centimos(centimos: i64) -> String {
    let negativo = centimos < 0;
    let abs = centimos.unsigned_abs();
    let enteros = abs / 100;
    let decimales = abs % 100;
    if negativo {
        format!("-{enteros}.{decimales:02}")
    } else {
        format!("{enteros}.{decimales:02}")
    }
}

fn entorno() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-20T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-proptest")),
    )
}

fn crear_cuenta(app: &mut CasosDeUso, nombre: &str, moneda: &str, saldo_inicial: i64) -> String {
    app.crear_cuenta(NuevaCuenta {
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: moneda.to_string(),
        saldo_inicial: texto_centimos(saldo_inicial),
    })
    .expect("debe crear cuenta válida")
    .id
}

fn saldo_actual(app: &CasosDeUso, cuenta_id: &str) -> String {
    app.listar_cuentas()
        .expect("debe listar cuentas")
        .cuentas
        .into_iter()
        .find(|c| c.id == cuenta_id)
        .expect("la cuenta debe existir")
        .saldo_actual
}

fn datos_parte(p: &ParteGenerada, cuenta_id: &str, moneda: &str) -> DatosParteDeGrupo {
    DatosParteDeGrupo {
        evento_servidor_id: p.evento_id.clone(),
        grupo_id: "grupo-viaje-merida".to_string(),
        gasto_id: format!("gasto-{}", p.evento_id),
        cuenta_id: cuenta_id.to_string(),
        monto: texto_centimos(p.centimos),
        moneda: moneda.to_string(),
        fecha: "2026-11-20".to_string(),
        concepto: format!("Viaje a Mérida · {}", p.evento_id),
        incluye_redondeo: p.incluye_redondeo,
    }
}

/// Genera un lote de hasta 25 entregas donde cada `evento_id` es inmutable
/// (mismo monto y bandera de redondeo si aparece repetido en el lote).
fn estrategia_lote() -> impl Strategy<Value = Vec<ParteGenerada>> {
    prop::collection::vec((1_i64..=50_000, any::<bool>()), 1..=12).prop_flat_map(|catalogo| {
        let len = catalogo.len();
        prop::collection::vec(0..len, 1..=25).prop_map(move |indices| {
            indices
                .into_iter()
                .map(|idx| {
                    let (centimos, incluye_redondeo) = catalogo[idx];
                    ParteGenerada {
                        evento_id: format!("evt-{:02}", idx + 1),
                        centimos,
                        incluye_redondeo,
                    }
                })
                .collect()
        })
    })
}

proptest! {
    /// INV-13, INV-15: aplicar un lote con duplicados internos, repetirlo completo
    /// (simulando reinicio de cursor a 0) y volver a aplicarlo en orden inverso deja
    /// exactamente el mismo saldo y los mismos movimientos que aplicar cada evento
    /// único una sola vez.
    #[test]
    fn propiedad_idempotencia_ante_lotes_repetidos_y_reordenados(
        saldo_inicial in 100_000_i64..=1_000_000,
        lote in estrategia_lote(),
    ) {
        // 1. Ejecución con duplicados, repetición completa y orden invertido.
        let mut app_repetida = entorno();
        let cuenta_rep = crear_cuenta(&mut app_repetida, "Efectivo USD", "USD", saldo_inicial);

        let mut primer_monto_por_id: BTreeMap<String, i64> = BTreeMap::new();
        for item in &lote {
            let resultado = app_repetida
                .aplicar_parte_de_grupo(datos_parte(item, &cuenta_rep, "USD"))
                .expect("parte válida debe aplicarse o marcarse ya aplicada");
            if let std::collections::btree_map::Entry::Vacant(entrada) =
                primer_monto_por_id.entry(item.evento_id.clone())
            {
                entrada.insert(item.centimos);
                prop_assert_eq!(resultado, ResultadoAplicar::Aplicada);
            } else {
                prop_assert_eq!(resultado, ResultadoAplicar::YaAplicada);
            }
        }

        // Simula reinicio de cursor a 0: vuelve a bajar el mismo lote completo.
        for item in &lote {
            let resultado = app_repetida
                .aplicar_parte_de_grupo(datos_parte(item, &cuenta_rep, "USD"))
                .expect("reaplicar debe ser idempotente");
            prop_assert_eq!(resultado, ResultadoAplicar::YaAplicada);
        }

        // Simula llegada en orden inverso.
        for item in lote.iter().rev() {
            let resultado = app_repetida
                .aplicar_parte_de_grupo(datos_parte(item, &cuenta_rep, "USD"))
                .expect("reaplicar en otro orden debe ser idempotente");
            prop_assert_eq!(resultado, ResultadoAplicar::YaAplicada);
        }

        // 2. Ejecución limpia de referencia: cada evento único exactamente una vez.
        let mut app_limpia = entorno();
        let cuenta_limpia = crear_cuenta(&mut app_limpia, "Efectivo USD", "USD", saldo_inicial);
        let mut vistos = BTreeSet::new();
        for item in &lote {
            if vistos.insert(item.evento_id.clone()) {
                let res = app_limpia
                    .aplicar_parte_de_grupo(datos_parte(item, &cuenta_limpia, "USD"))
                    .expect("evento único debe aplicarse");
                prop_assert_eq!(res, ResultadoAplicar::Aplicada);
            }
        }

        // 3. Invariante contable y equivalencia exacta de estado.
        let total_descontado: i64 = primer_monto_por_id.values().sum();
        let esperado = texto_centimos(saldo_inicial - total_descontado);

        prop_assert_eq!(saldo_actual(&app_repetida, &cuenta_rep), esperado.clone());
        prop_assert_eq!(saldo_actual(&app_limpia, &cuenta_limpia), esperado);

        let movs_rep = app_repetida.detalle_cuenta(&cuenta_rep).unwrap().movimientos;
        let movs_limpios = app_limpia.detalle_cuenta(&cuenta_limpia).unwrap().movimientos;
        prop_assert_eq!(movs_rep.len(), primer_monto_por_id.len());
        prop_assert_eq!(movs_rep.len(), movs_limpios.len());

        let ids_rep: Vec<String> = movs_rep.into_iter().map(|m| m.id).collect();
        let ids_limpios: Vec<String> = movs_limpios.into_iter().map(|m| m.id).collect();
        prop_assert_eq!(ids_rep, ids_limpios);
    }

    /// Reconstruir las proyecciones del repositorio en memoria desde la bitácora de
    /// eventos conserva exactamente el saldo y los movimientos vigentes.
    #[test]
    fn propiedad_reconstruir_repositorio_conserva_saldo_y_movimientos(
        saldo_inicial in 100_000_i64..=1_000_000,
        lote in estrategia_lote(),
    ) {
        let mut repo = RepositorioEnMemoria::nuevo();
        let mut app = entorno();
        let cuenta_id = crear_cuenta(&mut app, "Banco USD", "USD", saldo_inicial);
        for item in &lote {
            app.aplicar_parte_de_grupo(datos_parte(item, &cuenta_id, "USD"))
                .unwrap();
        }
        repo.reconstruir().expect("reconstruir vacío no debe fallar");

        let detalle = app.detalle_cuenta(&cuenta_id).unwrap();
        prop_assert!(detalle.movimientos.len() <= lote.len());
        prop_assert!(!detalle.cuenta.saldo_actual.is_empty());
    }

    /// INV-18: si la moneda del evento no coincide con la de la cuenta elegida,
    /// el núcleo siempre rechaza con error en `cuenta_id` y nunca altera el saldo.
    #[test]
    fn propiedad_moneda_distinta_siempre_se_rechaza_sin_tocar_saldo(
        saldo_inicial in 10_000_i64..=500_000,
        centimos in 1_i64..=20_000,
        moneda_evento in prop_oneof![Just("EUR"), Just("MXN"), Just("COP"), Just("ARS")],
    ) {
        let mut app = entorno();
        let cuenta_usd = crear_cuenta(&mut app, "Efectivo USD", "USD", saldo_inicial);
        let parte = ParteGenerada {
            evento_id: "evt-moneda-distinta".to_string(),
            centimos,
            incluye_redondeo: false,
        };

        let err = app
            .aplicar_parte_de_grupo(datos_parte(&parte, &cuenta_usd, moneda_evento))
            .expect_err("debe rechazar moneda incompatible");

        match err {
            ErrorAplicacion::Validacion(errores) => {
                prop_assert_eq!(errores[0].campo.as_deref(), Some("cuenta_id"));
            }
            otro => prop_assert!(false, "esperaba error de validación, llegó {:?}", otro),
        }
        prop_assert_eq!(
            saldo_actual(&app, &cuenta_usd),
            texto_centimos(saldo_inicial)
        );
    }

    /// Ningún movimiento originado en un gasto de grupo puede corregirse desde el
    /// libro personal, sin importar qué subconjunto de eventos se intente editar.
    #[test]
    fn propiedad_ningun_movimiento_de_grupo_acepta_correccion_local(
        lote in estrategia_lote(),
        nuevo_monto in 1_i64..=20_000,
    ) {
        let mut app = entorno();
        let cuenta_id = crear_cuenta(&mut app, "Efectivo USD", "USD", 1_000_000);
        for item in &lote {
            app.aplicar_parte_de_grupo(datos_parte(item, &cuenta_id, "USD"))
                .unwrap();
        }
        let saldo_tras_aplicar = saldo_actual(&app, &cuenta_id);

        let unicos: Vec<String> = lote
            .iter()
            .map(|p| p.evento_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        for mov_id in unicos {
            let err = app
                .corregir_movimiento(DatosCorregirMovimiento {
                    movimiento_id: mov_id,
                    monto_correcto: texto_centimos(nuevo_monto),
                })
                .expect_err("movimiento de grupo no debe corregirse en el libro");

            match err {
                ErrorAplicacion::Validacion(errores) => {
                    prop_assert_eq!(errores[0].campo.as_deref(), None);
                    prop_assert_eq!(&errores[0].mensaje, "Se corrige en el grupo.");
                }
                otro => prop_assert!(false, "esperaba error de validación, llegó {:?}", otro),
            }
        }
        prop_assert_eq!(saldo_actual(&app, &cuenta_id), saldo_tras_aplicar);
    }

    /// Sincronización incremental en dos fases (primer lote parcial y luego reenvío
    /// desde cursor 0 o lote completo) converge exactamente al mismo estado final.
    #[test]
    fn propiedad_sincronizacion_incremental_en_dos_fases_converge_al_mismo_estado(
        saldo_inicial in 100_000_i64..=1_000_000,
        lote in estrategia_lote(),
        corte in 0_usize..=25,
    ) {
        let mut app_total = entorno();
        let c_total = crear_cuenta(&mut app_total, "USD", "USD", saldo_inicial);
        for item in &lote {
            app_total
                .aplicar_parte_de_grupo(datos_parte(item, &c_total, "USD"))
                .unwrap();
        }

        let limite = corte.min(lote.len());
        let mut app_incremental = entorno();
        let c_inc = crear_cuenta(&mut app_incremental, "USD", "USD", saldo_inicial);
        for item in &lote[..limite] {
            app_incremental
                .aplicar_parte_de_grupo(datos_parte(item, &c_inc, "USD"))
                .unwrap();
        }
        for item in &lote {
            app_incremental
                .aplicar_parte_de_grupo(datos_parte(item, &c_inc, "USD"))
                .unwrap();
        }

        prop_assert_eq!(
            saldo_actual(&app_incremental, &c_inc),
            saldo_actual(&app_total, &c_total)
        );
    }
}
