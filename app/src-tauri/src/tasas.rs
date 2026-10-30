//! Comandos de tasas de cambio, moneda base, monedas propias y patrimonio (Entrega 4).
//!
//! Cada comando bloquea los casos de uso, los ejecuta y convierte el resultado a los
//! datos acordados (`dto.rs`). Aquí no hay reglas de negocio: todas las validaciones,
//! la selección de la tasa vigente, la inversa y el redondeo viven en el dominio y en
//! la capa de aplicación.

use std::sync::{Mutex, MutexGuard};

use aplicacion::{CasosDeUso, NuevaMonedaPropia, NuevaTasa, PatrimonioVista, TasaListada};
use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::{
    CambioMonedaBaseDto, ConsultaPatrimonioDto, ErroresDto, HistorialParDto, MonedaBaseDto,
    MonedaDto, NuevaMonedaDto, NuevaTasaDto, ParFaltanteDto, PatrimonioDto, SaldoCuentaDto,
    SubtotalMonedaDto, TasaDto, TasaUsadaDto,
};

// ---------- Conversiones entre los datos de la pantalla y los casos de uso ----------

impl From<NuevaTasaDto> for NuevaTasa {
    fn from(d: NuevaTasaDto) -> Self {
        NuevaTasa {
            moneda_origen: d.moneda_origen,
            moneda_destino: d.moneda_destino,
            valor: d.valor,
            fecha: d.fecha,
            fuente: d.fuente,
        }
    }
}

impl From<TasaListada> for TasaDto {
    fn from(t: TasaListada) -> Self {
        TasaDto {
            id: t.id,
            moneda_origen: t.moneda_origen,
            moneda_destino: t.moneda_destino,
            valor: t.valor,
            fecha: t.fecha,
            fuente: t.fuente,
        }
    }
}

impl From<PatrimonioVista> for PatrimonioDto {
    fn from(v: PatrimonioVista) -> Self {
        PatrimonioDto {
            moneda_base: v.moneda_base,
            fecha_consulta: v.fecha_consulta,
            cuentas: v
                .cuentas
                .into_iter()
                .map(|c| SaldoCuentaDto {
                    id: c.id,
                    nombre: c.nombre,
                    moneda: c.moneda,
                    saldo: c.saldo,
                    en_moneda_base: c.en_moneda_base,
                })
                .collect(),
            subtotales: v
                .subtotales
                .into_iter()
                .map(|s| SubtotalMonedaDto {
                    moneda: s.moneda,
                    total: s.total,
                })
                .collect(),
            total: v.total,
            tasas_usadas: v
                .tasas_usadas
                .into_iter()
                .map(|t| TasaUsadaDto {
                    moneda_origen: t.origen,
                    moneda_destino: t.destino,
                    valor: t.valor,
                    fecha: t.fecha,
                    fuente: t.fuente,
                    inversa: t.es_inversa,
                })
                .collect(),
            faltantes: v
                .faltantes
                .into_iter()
                .map(|f| ParFaltanteDto {
                    moneda_origen: f.origen,
                    moneda_destino: f.destino,
                })
                .collect(),
        }
    }
}

impl From<NuevaMonedaDto> for NuevaMonedaPropia {
    fn from(d: NuevaMonedaDto) -> Self {
        NuevaMonedaPropia {
            codigo: d.codigo,
            nombre: d.nombre,
            simbolo: d.simbolo,
            // Los decimales viajan como texto; el núcleo los lee y valida (de 0 a 8).
            decimales: d.decimales,
        }
    }
}

/// Agrupa las tasas por par de monedas y ordena cada historial de la más reciente a
/// la más antigua por fecha. En la misma fecha se conserva el orden que entrega el
/// núcleo.
fn agrupar_por_par(tasas: Vec<TasaDto>) -> Vec<HistorialParDto> {
    let mut grupos: Vec<HistorialParDto> = Vec::new();
    for t in tasas {
        match grupos
            .iter_mut()
            .find(|g| g.moneda_origen == t.moneda_origen && g.moneda_destino == t.moneda_destino)
        {
            Some(g) => g.tasas.push(t),
            None => grupos.push(HistorialParDto {
                moneda_origen: t.moneda_origen.clone(),
                moneda_destino: t.moneda_destino.clone(),
                tasas: vec![t],
            }),
        }
    }
    for g in &mut grupos {
        g.tasas.sort_by(|a, b| b.fecha.cmp(&a.fecha));
    }
    grupos
}

// ---------- Lógica de cada comando (se prueba sin Tauri) ----------

/// Acceso exclusivo a los casos de uso. Si un hilo anterior falló con el candado
/// tomado, se recupera el estado: los datos siguen siendo válidos.
fn bloquear(casos: &Mutex<CasosDeUso>) -> MutexGuard<'_, CasosDeUso> {
    casos.lock().unwrap_or_else(|e| e.into_inner())
}

fn registrar(casos: &Mutex<CasosDeUso>, datos: NuevaTasaDto) -> Result<TasaDto, ErroresDto> {
    bloquear(casos)
        .registrar_tasa(datos.into())
        .map(TasaDto::from)
        .map_err(ErroresDto::from)
}

fn eliminar(casos: &Mutex<CasosDeUso>, tasa_id: &str) -> Result<(), ErroresDto> {
    bloquear(casos)
        .eliminar_tasa(tasa_id.trim())
        .map_err(ErroresDto::from)
}

fn historial(casos: &Mutex<CasosDeUso>) -> Result<Vec<HistorialParDto>, ErroresDto> {
    let tasas = bloquear(casos).listar_tasas().map_err(ErroresDto::from)?;
    Ok(agrupar_por_par(
        tasas.into_iter().map(TasaDto::from).collect(),
    ))
}

fn cambiar_base(
    casos: &Mutex<CasosDeUso>,
    datos: CambioMonedaBaseDto,
) -> Result<MonedaBaseDto, ErroresDto> {
    bloquear(casos)
        .cambiar_moneda_base(datos.moneda.trim())
        .map(|moneda| MonedaBaseDto { moneda })
        .map_err(ErroresDto::from)
}

fn nueva_moneda(casos: &Mutex<CasosDeUso>, datos: NuevaMonedaDto) -> Result<MonedaDto, ErroresDto> {
    bloquear(casos)
        .crear_moneda(datos.into())
        .map(MonedaDto::from)
        .map_err(ErroresDto::from)
}

fn patrimonio(
    casos: &Mutex<CasosDeUso>,
    datos: ConsultaPatrimonioDto,
) -> Result<PatrimonioDto, ErroresDto> {
    let fecha = datos.fecha.trim();
    let fecha = if fecha.is_empty() {
        None
    } else {
        Some(fecha.to_string())
    };
    bloquear(casos)
        .consultar_patrimonio(fecha)
        .map(PatrimonioDto::from)
        .map_err(ErroresDto::from)
}

// ---------- Comandos ----------

/// Registra una tasa de cambio (1 origen = valor destino) y devuelve la tasa guardada.
#[tauri::command]
pub fn registrar_tasa(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaTasaDto,
) -> Result<TasaDto, ErroresDto> {
    registrar(&estado, datos)
}

/// Elimina una tasa registrada por error (en el libro es un evento nuevo, no un
/// borrado). Se invoca con `invoke('eliminar_tasa', { tasa_id })`.
#[tauri::command(rename_all = "snake_case")]
pub fn eliminar_tasa(
    estado: State<'_, EstadoCasosDeUso>,
    tasa_id: String,
) -> Result<(), ErroresDto> {
    eliminar(&estado, &tasa_id)
}

/// Historial de cada par de monedas, con valor, fecha y fuente.
#[tauri::command]
pub fn listar_tasas(
    estado: State<'_, EstadoCasosDeUso>,
) -> Result<Vec<HistorialParDto>, ErroresDto> {
    historial(&estado)
}

/// Cambia la moneda base en la que se presenta el total consolidado.
#[tauri::command]
pub fn cambiar_moneda_base(
    estado: State<'_, EstadoCasosDeUso>,
    datos: CambioMonedaBaseDto,
) -> Result<MonedaBaseDto, ErroresDto> {
    cambiar_base(&estado, datos)
}

/// Crea una moneda propia (código de 3 a 5 mayúsculas, nombre, símbolo y decimales).
#[tauri::command]
pub fn crear_moneda(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaMonedaDto,
) -> Result<MonedaDto, ErroresDto> {
    nueva_moneda(&estado, datos)
}

/// Patrimonio a una fecha ("" = hoy): saldos, subtotales por moneda y total
/// consolidado en la moneda base, o los pares de tasas que faltan.
#[tauri::command]
pub fn consultar_patrimonio(
    estado: State<'_, EstadoCasosDeUso>,
    datos: ConsultaPatrimonioDto,
) -> Result<PatrimonioDto, ErroresDto> {
    patrimonio(&estado, datos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aplicacion::NuevaCuenta;

    /// `ErroresDto` no implementa `Debug`, así que `unwrap()` no sirve con él.
    fn bien<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => {
                let mensajes: Vec<&str> = e.errores.iter().map(|x| x.mensaje.as_str()).collect();
                panic!("se esperaba Ok y hubo errores: {mensajes:?}")
            }
        }
    }

    fn fallo<T>(r: Result<T, ErroresDto>) -> ErroresDto {
        match r {
            Ok(_) => panic!("se esperaba un error y salió bien"),
            Err(e) => e,
        }
    }

    fn campos(e: &ErroresDto) -> Vec<Option<&str>> {
        e.errores.iter().map(|x| x.campo.as_deref()).collect()
    }

    fn casos() -> Mutex<CasosDeUso> {
        Mutex::new(crate::crear_casos_de_uso())
    }

    fn cuenta(c: &Mutex<CasosDeUso>, nombre: &str, tipo: &str, moneda: &str, saldo: &str) {
        bloquear(c)
            .crear_cuenta(NuevaCuenta {
                nombre: nombre.into(),
                tipo: tipo.into(),
                moneda: moneda.into(),
                saldo_inicial: saldo.into(),
            })
            .expect("crear la cuenta de prueba");
    }

    fn tasa(origen: &str, destino: &str, valor: &str, fecha: &str, fuente: &str) -> NuevaTasaDto {
        NuevaTasaDto {
            moneda_origen: origen.into(),
            moneda_destino: destino.into(),
            valor: valor.into(),
            fecha: fecha.into(),
            fuente: fuente.into(),
        }
    }

    fn consulta() -> ConsultaPatrimonioDto {
        ConsultaPatrimonioDto {
            fecha: String::new(),
        }
    }

    fn base(c: &Mutex<CasosDeUso>, moneda: &str) {
        bien(cambiar_base(
            c,
            CambioMonedaBaseDto {
                moneda: moneda.into(),
            },
        ));
    }

    /// Cuentas y tasas del ejemplo 2.3 del enunciado.
    fn cargar_ejemplo(c: &Mutex<CasosDeUso>) {
        cuenta(c, "Efectivo bolívares", "efectivo", "VES", "88700");
        cuenta(c, "Banco en dólares", "banco", "USD", "200");
        cuenta(c, "Billetera USDT", "billetera_digital", "USDT", "50");
        bien(registrar(c, tasa("USD", "VES", "887", "2026-10-20", "BCV")));
        bien(registrar(
            c,
            tasa("USDT", "VES", "1000", "2026-10-21", "casa de cambio"),
        ));
    }

    #[test]
    fn el_ejemplo_2_3_se_reproduce_de_punta_a_punta() {
        let c = casos();
        cargar_ejemplo(&c);

        // Base USD (por defecto): falta la tasa USDT -> USD, así que no hay total.
        let p = bien(patrimonio(&c, consulta()));
        assert_eq!(p.moneda_base, "USD");
        assert_eq!(p.total, None);
        assert_eq!(p.cuentas.len(), 3);
        assert_eq!(p.subtotales.len(), 3);
        assert!(
            p.faltantes
                .iter()
                .any(|f| f.moneda_origen == "USDT" && f.moneda_destino == "USD"),
            "faltantes: {:?}",
            p.faltantes
                .iter()
                .map(|f| (&f.moneda_origen, &f.moneda_destino))
                .collect::<Vec<_>>()
        );

        // Al registrar 1 USDT = 1,00 USD el total es 350,00 USD.
        bien(registrar(
            &c,
            tasa("USDT", "USD", "1", "2026-10-23", "estimación propia"),
        ));
        let p = bien(patrimonio(&c, consulta()));
        assert_eq!(p.total.as_deref(), Some("350.00"));
        assert!(p.faltantes.is_empty());
        assert!(p.tasas_usadas.iter().any(|t| t.inversa));

        // Con base VES el total es 316.100,00.
        base(&c, "VES");
        let p = bien(patrimonio(&c, consulta()));
        assert_eq!(p.moneda_base, "VES");
        assert_eq!(p.total.as_deref(), Some("316100.00"));
        assert!(p.faltantes.is_empty());
        assert!(p.tasas_usadas.iter().all(|t| !t.inversa));
        assert!(p.cuentas.iter().all(|c| c.en_moneda_base.is_some()));
    }

    #[test]
    fn eliminar_una_tasa_necesaria_deja_de_consolidar_y_nombra_el_par() {
        let c = casos();
        cargar_ejemplo(&c);
        base(&c, "VES");
        assert!(bien(patrimonio(&c, consulta())).total.is_some());

        let id = bien(historial(&c))
            .into_iter()
            .find(|h| h.moneda_origen == "USDT" && h.moneda_destino == "VES")
            .and_then(|h| h.tasas.into_iter().next())
            .map(|t| t.id)
            .expect("tasa USDT a VES");
        bien(eliminar(&c, &id));

        let p = bien(patrimonio(&c, consulta()));
        assert_eq!(p.total, None);
        assert!(
            p.faltantes
                .iter()
                .any(|f| f.moneda_origen == "USDT" && f.moneda_destino == "VES")
        );
        // Los saldos de las cuentas no cambian por eliminar una tasa.
        assert!(p.cuentas.iter().any(|x| x.saldo == "88700.00"));

        // Eliminarla otra vez, o sin indicar cuál, se rechaza.
        assert!(eliminar(&c, &id).is_err());
        assert!(eliminar(&c, "  ").is_err());
    }

    #[test]
    fn cambiar_la_base_no_cambia_los_saldos() {
        let c = casos();
        cargar_ejemplo(&c);
        let saldos = |c: &Mutex<CasosDeUso>| -> Vec<String> {
            bien(patrimonio(c, consulta()))
                .cuentas
                .into_iter()
                .map(|x| x.saldo)
                .collect()
        };
        let antes = saldos(&c);
        base(&c, "VES");
        base(&c, "EUR");
        assert_eq!(antes, saldos(&c));
    }

    #[test]
    fn una_moneda_base_inexistente_se_rechaza() {
        let c = casos();
        let e = fallo(cambiar_base(
            &c,
            CambioMonedaBaseDto {
                moneda: "QQQ".into(),
            },
        ));
        assert!(!e.errores.is_empty());
    }

    #[test]
    fn varias_tasas_del_mismo_par_se_agrupan_de_la_mas_reciente_a_la_mas_antigua() {
        let c = casos();
        cargar_ejemplo(&c);
        bien(registrar(
            &c,
            tasa("USD", "VES", "900", "2026-10-25", "BCV"),
        ));

        let h = bien(historial(&c));
        assert_eq!(h.len(), 2, "pares: USD a VES y USDT a VES");
        let par = h
            .iter()
            .find(|g| g.moneda_origen == "USD" && g.moneda_destino == "VES")
            .expect("par USD a VES");
        assert_eq!(par.tasas.len(), 2);
        assert_eq!(par.tasas[0].fecha, "2026-10-25");
        assert_eq!(par.tasas[1].fecha, "2026-10-20");
    }

    #[test]
    fn una_tasa_invalida_se_rechaza_con_el_campo_que_falla() {
        let c = casos();
        let con_campo = |t: NuevaTasaDto, campo: &str| {
            let e = fallo(registrar(&c, t));
            assert!(
                campos(&e).contains(&Some(campo)),
                "se esperaba el campo {campo}, llegó {:?}",
                campos(&e)
            );
        };
        con_campo(tasa("USD", "VES", "0", "2026-10-20", "BCV"), "valor");
        con_campo(tasa("USD", "VES", "-5", "2026-10-20", "BCV"), "valor");
        con_campo(
            tasa("USD", "VES", "1.123456789", "2026-10-20", "BCV"),
            "valor",
        );
        con_campo(tasa("USD", "VES", "887", "2999-01-01", "BCV"), "fecha");
        con_campo(tasa("USD", "VES", "887", "", "BCV"), "fecha");
        con_campo(tasa("USD", "VES", "887", "2026-10-20", "  "), "fuente");
        // Monedas iguales: el campo exacto lo decide el dominio; basta con que falle.
        assert!(registrar(&c, tasa("USD", "USD", "1", "2026-10-20", "BCV")).is_err());
        // Hasta 8 decimales es válido.
        bien(registrar(
            &c,
            tasa("USD", "EUR", "0.12345678", "2026-10-20", "BCV"),
        ));
    }

    #[test]
    fn una_moneda_propia_se_crea_y_se_usa_en_una_cuenta_y_en_una_tasa() {
        let c = casos();
        let m = bien(nueva_moneda(
            &c,
            NuevaMonedaDto {
                codigo: "BTC".into(),
                nombre: "Bitcoin".into(),
                simbolo: "₿".into(),
                decimales: "8".into(),
            },
        ));
        assert_eq!(m.codigo, "BTC");
        assert_eq!(m.decimales, 8);
        assert!(!m.de_fabrica);

        cuenta(&c, "Cripto", "billetera_digital", "BTC", "1");
        bien(registrar(
            &c,
            tasa("BTC", "USD", "65000", "2026-10-20", "estimación propia"),
        ));
        base(&c, "BTC");
    }

    #[test]
    fn una_moneda_invalida_o_repetida_se_rechaza_por_campo() {
        let c = casos();
        let datos = |codigo: &str, decimales: &str| NuevaMonedaDto {
            codigo: codigo.into(),
            nombre: "Prueba".into(),
            simbolo: "P".into(),
            decimales: decimales.into(),
        };
        // Los decimales vacíos o que no son un entero se rechazan por campo.
        for decimales in ["", "dos", "-1"] {
            let e = fallo(nueva_moneda(&c, datos("ZZZ", decimales)));
            assert!(
                campos(&e).contains(&Some("decimales")),
                "decimales {decimales:?}: llegó {:?}",
                campos(&e)
            );
        }
        // El código repetido, con formato inválido o con decimales fuera de rango los
        // decide el dominio.
        assert!(nueva_moneda(&c, datos("USD", "2")).is_err());
        assert!(nueva_moneda(&c, datos("usd1", "2")).is_err());
        assert!(nueva_moneda(&c, datos("ZZZ", "9")).is_err());
    }

    #[test]
    fn la_fecha_de_consulta_futura_se_rechaza_con_el_campo_fecha() {
        let c = casos();
        let e = fallo(patrimonio(
            &c,
            ConsultaPatrimonioDto {
                fecha: "2999-01-01".into(),
            },
        ));
        assert!(campos(&e).contains(&Some("fecha")));
    }
}
