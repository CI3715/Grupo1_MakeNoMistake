//! Comandos de categorías, movimientos y detalle de cuenta (Entrega 3).
//!
//! Esta capa solo traduce: convierte los datos que llegan de la pantalla a los de
//! los casos de uso y devuelve el resultado como texto. No suma, no resta ni valida
//! reglas de negocio: todo eso vive en el dominio y en la capa de aplicación.
//! Los errores llegan por campo y en español (`ErroresDto`), sin trazas técnicas.

use std::sync::Mutex;

use aplicacion::CasosDeUso;
use tauri::State;

use crate::EstadoCasosDeUso;
use crate::cuentas::bloquear;
use crate::dto::{
    CategoriaDto, CorreccionDto, DetalleCuentaDto, ErroresDto, MovimientoDto, NuevaCategoriaDto,
    NuevoMovimientoDto, PrevisualizacionDto,
};

fn registrar(
    casos: &Mutex<CasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<MovimientoDto, ErroresDto> {
    bloquear(casos)
        .registrar_movimiento(datos.into())
        .map(MovimientoDto::from)
        .map_err(ErroresDto::from)
}

fn corregir(casos: &Mutex<CasosDeUso>, datos: CorreccionDto) -> Result<MovimientoDto, ErroresDto> {
    bloquear(casos)
        .corregir_movimiento(datos.into())
        .map(MovimientoDto::from)
        .map_err(ErroresDto::from)
}

fn previsualizar(
    casos: &Mutex<CasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<PrevisualizacionDto, ErroresDto> {
    bloquear(casos)
        .previsualizar_movimiento(datos.into())
        .map(PrevisualizacionDto::from)
        .map_err(ErroresDto::from)
}

fn categorias(casos: &Mutex<CasosDeUso>) -> Result<Vec<CategoriaDto>, ErroresDto> {
    bloquear(casos)
        .listar_categorias()
        .map(|lista| lista.into_iter().map(CategoriaDto::from).collect())
        .map_err(ErroresDto::from)
}

fn crear_cat(
    casos: &Mutex<CasosDeUso>,
    datos: NuevaCategoriaDto,
) -> Result<CategoriaDto, ErroresDto> {
    bloquear(casos)
        .crear_categoria(datos.into())
        .map(|c| CategoriaDto {
            id: c.id,
            nombre: c.nombre,
            tipo: c.tipo,
            de_fabrica: c.de_fabrica,
        })
        .map_err(ErroresDto::from)
}

fn detalle(casos: &Mutex<CasosDeUso>, cuenta_id: &str) -> Result<DetalleCuentaDto, ErroresDto> {
    bloquear(casos)
        .detalle_cuenta(cuenta_id)
        .map(DetalleCuentaDto::from)
        .map_err(ErroresDto::from)
}

// ---------- Comandos ----------

/// Registra un gasto o un ingreso en una cuenta y devuelve el movimiento.
#[tauri::command]
pub fn registrar_movimiento(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<MovimientoDto, ErroresDto> {
    registrar(&estado, datos)
}

/// Corrige un movimiento registrando un ajuste por la diferencia; devuelve el ajuste.
#[tauri::command]
pub fn corregir_movimiento(
    estado: State<'_, EstadoCasosDeUso>,
    datos: CorreccionDto,
) -> Result<MovimientoDto, ErroresDto> {
    corregir(&estado, datos)
}

/// Calcula el saldo que quedaría tras un movimiento, sin guardar nada.
#[tauri::command]
pub fn previsualizar_movimiento(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<PrevisualizacionDto, ErroresDto> {
    previsualizar(&estado, datos)
}

/// Lista las categorías de fábrica y las creadas por la persona.
#[tauri::command]
pub fn listar_categorias(
    estado: State<'_, EstadoCasosDeUso>,
) -> Result<Vec<CategoriaDto>, ErroresDto> {
    categorias(&estado)
}

/// Crea una categoría propia.
#[tauri::command]
pub fn crear_categoria(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaCategoriaDto,
) -> Result<CategoriaDto, ErroresDto> {
    crear_cat(&estado, datos)
}

/// Detalle de una cuenta: saldo, composición y movimientos. Se invoca con
/// `invoke('detalle_cuenta', { cuenta_id })`, en snake_case como el resto.
#[tauri::command(rename_all = "snake_case")]
pub fn detalle_cuenta(
    estado: State<'_, EstadoCasosDeUso>,
    cuenta_id: String,
) -> Result<DetalleCuentaDto, ErroresDto> {
    detalle(&estado, &cuenta_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aplicacion::NuevaCuenta;

    fn casos() -> Mutex<CasosDeUso> {
        Mutex::new(crate::crear_casos_de_uso())
    }

    /// `ErroresDto` no implementa `Debug`, así que `unwrap()` no sirve con él.
    fn bien<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => panic!("debía salir bien, pero falló: {:?}", mensajes(&e)),
        }
    }

    fn mal<T>(r: Result<T, ErroresDto>) -> ErroresDto {
        match r {
            Ok(_) => panic!("debía fallar"),
            Err(e) => e,
        }
    }

    fn mensajes(e: &ErroresDto) -> Vec<String> {
        e.errores
            .iter()
            .map(|x| format!("{:?}: {}", x.campo, x.mensaje))
            .collect()
    }

    fn campos(e: &ErroresDto) -> Vec<Option<&str>> {
        e.errores.iter().map(|x| x.campo.as_deref()).collect()
    }

    /// Crea «Billetera dólares» (USD) con saldo inicial 1000.00 y devuelve su id.
    fn billetera(c: &Mutex<CasosDeUso>) -> String {
        bloquear(c)
            .crear_cuenta(NuevaCuenta {
                nombre: "Billetera dólares".into(),
                tipo: "banco".into(),
                moneda: "USD".into(),
                saldo_inicial: "1000.00".into(),
            })
            .map(|cuenta| cuenta.id)
            .unwrap_or_else(|_| panic!("la cuenta debía crearse"))
    }

    fn mov(
        cuenta: &str,
        tipo: &str,
        monto: &str,
        fecha: &str,
        categoria: &str,
    ) -> NuevoMovimientoDto {
        NuevoMovimientoDto {
            cuenta_id: cuenta.into(),
            tipo: tipo.into(),
            monto: monto.into(),
            fecha: fecha.into(),
            categoria_id: categoria.into(),
            nota: String::new(),
        }
    }

    fn gasto(cuenta: &str, monto: &str) -> NuevoMovimientoDto {
        mov(cuenta, "gasto", monto, "2026-10-19", "gasto:alimentacion")
    }

    fn saldo(c: &Mutex<CasosDeUso>, cuenta: &str) -> String {
        bien(detalle(c, cuenta)).cuenta.saldo_actual
    }

    #[test]
    fn hay_siete_categorias_de_gasto_y_cuatro_de_ingreso() {
        let lista = bien(categorias(&casos()));
        assert_eq!(lista.iter().filter(|c| c.tipo == "gasto").count(), 7);
        assert_eq!(lista.iter().filter(|c| c.tipo == "ingreso").count(), 4);
        assert!(lista.iter().all(|c| c.de_fabrica));
    }

    #[test]
    fn una_categoria_propia_aparece_en_la_lista_y_no_se_repite() {
        let c = casos();
        let nueva = |nombre: &str| NuevaCategoriaDto {
            nombre: nombre.into(),
            tipo: "gasto".into(),
        };
        let creada = bien(crear_cat(&c, nueva("Mascotas")));
        assert!(!creada.de_fabrica);
        assert_eq!(bien(categorias(&c)).len(), 12);
        let e = mal(crear_cat(&c, nueva("mascotas")));
        assert_eq!(campos(&e), vec![Some("nombre")]);
    }

    #[test]
    fn el_ejemplo_2_2_se_reproduce_tal_cual() {
        let c = casos();
        let cuenta = billetera(&c);
        assert_eq!(saldo(&c, &cuenta), "1000.00");

        bien(registrar(
            &c,
            mov(
                &cuenta,
                "ingreso",
                "800.00",
                "2026-10-19",
                "ingreso:salario",
            ),
        ));
        assert_eq!(saldo(&c, &cuenta), "1800.00");

        let gasto_mal = bien(registrar(&c, gasto(&cuenta, "500.00")));
        assert_eq!(saldo(&c, &cuenta), "1300.00");

        let ajuste = bien(corregir(
            &c,
            CorreccionDto {
                movimiento_id: gasto_mal.id.clone(),
                monto_correcto: "50.00".into(),
            },
        ));
        assert_eq!(ajuste.estado, "ajuste");
        assert_eq!(ajuste.monto_con_signo, "450.00");
        assert_eq!(ajuste.corrige_a.as_deref(), Some(gasto_mal.id.as_str()));
        assert_eq!(saldo(&c, &cuenta), "1750.00");

        let d = bien(detalle(&c, &cuenta));
        assert_eq!(d.composicion.saldo_inicial, "1000.00");
        assert_eq!(d.composicion.total_ingresos, "800.00");
        assert_eq!(d.composicion.total_gastos, "500.00");
        assert_eq!(
            d.composicion.total_ajustes.trim_start_matches('+'),
            "450.00"
        );
        assert_eq!(d.movimientos.len(), 3);
        let original = d.movimientos.iter().find(|m| m.id == gasto_mal.id);
        match original {
            Some(m) => {
                assert_eq!(m.estado, "corregido");
                assert_eq!(m.ajustes, vec![ajuste.id.clone()]);
            }
            None => panic!("el original debe seguir visible"),
        }
    }

    #[test]
    fn un_ingreso_sube_y_un_gasto_baja_el_saldo_exactamente() {
        let c = casos();
        let cuenta = billetera(&c);
        let ing = bien(registrar(
            &c,
            mov(&cuenta, "ingreso", "10.25", "2026-10-19", "ingreso:ventas"),
        ));
        assert_eq!(ing.monto_con_signo, "10.25");
        assert_eq!(saldo(&c, &cuenta), "1010.25");
        let gas = bien(registrar(&c, gasto(&cuenta, "0.25")));
        assert_eq!(gas.monto_con_signo, "-0.25");
        assert_eq!(saldo(&c, &cuenta), "1010.00");
    }

    #[test]
    fn monto_cero_negativo_o_con_decimales_de_mas_se_rechaza_por_monto() {
        let c = casos();
        let cuenta = billetera(&c);
        for monto in ["0", "-5", "10.555"] {
            let e = mal(registrar(&c, gasto(&cuenta, monto)));
            assert_eq!(campos(&e), vec![Some("monto")], "monto {monto}");
        }
        assert_eq!(saldo(&c, &cuenta), "1000.00");
    }

    #[test]
    fn categoria_del_tipo_equivocado_vacia_o_inexistente_se_rechaza_por_categoria() {
        let c = casos();
        let cuenta = billetera(&c);
        for categoria in ["ingreso:salario", "", "no-existe"] {
            let e = mal(registrar(
                &c,
                mov(&cuenta, "gasto", "5.00", "2026-10-19", categoria),
            ));
            assert_eq!(
                campos(&e),
                vec![Some("categoria")],
                "categoria {categoria:?}"
            );
        }
    }

    #[test]
    fn una_fecha_futura_se_rechaza_por_fecha() {
        let c = casos();
        let cuenta = billetera(&c);
        let e = mal(registrar(
            &c,
            mov(&cuenta, "gasto", "5.00", "2999-01-01", "gasto:salud"),
        ));
        assert_eq!(campos(&e), vec![Some("fecha")]);
    }

    #[test]
    fn previsualizar_advierte_saldo_negativo_y_no_guarda_nada() {
        let c = casos();
        let cuenta = billetera(&c);
        let p = bien(previsualizar(&c, gasto(&cuenta, "1500.00")));
        assert!(p.deja_saldo_negativo);
        assert_eq!(p.saldo_resultante, "-500.00");
        let p = bien(previsualizar(&c, gasto(&cuenta, "100.00")));
        assert!(!p.deja_saldo_negativo);
        assert_eq!(saldo(&c, &cuenta), "1000.00");
        assert!(bien(detalle(&c, &cuenta)).movimientos.is_empty());
    }

    #[test]
    fn corregir_a_cero_anula_y_un_ajuste_no_se_corrige() {
        let c = casos();
        let cuenta = billetera(&c);
        let original = bien(registrar(&c, gasto(&cuenta, "20.00")));
        let ajuste = bien(corregir(
            &c,
            CorreccionDto {
                movimiento_id: original.id.clone(),
                monto_correcto: "0".into(),
            },
        ));
        assert_eq!(saldo(&c, &cuenta), "1000.00");
        let de_nuevo = |id: &str| CorreccionDto {
            movimiento_id: id.into(),
            monto_correcto: "5.00".into(),
        };
        assert!(corregir(&c, de_nuevo(&ajuste.id)).is_err());
        assert!(corregir(&c, de_nuevo("no-existe")).is_err());
    }

    #[test]
    fn corregir_con_el_mismo_valor_vigente_o_negativo_se_rechaza_por_monto() {
        let c = casos();
        let cuenta = billetera(&c);
        let original = bien(registrar(&c, gasto(&cuenta, "20.00")));
        for monto in ["20.00", "-1"] {
            let e = mal(corregir(
                &c,
                CorreccionDto {
                    movimiento_id: original.id.clone(),
                    monto_correcto: monto.into(),
                },
            ));
            assert_eq!(campos(&e), vec![Some("monto")], "monto {monto}");
        }
    }
}