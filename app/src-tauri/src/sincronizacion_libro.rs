//! Convierte los cambios recibidos del servidor en un lote para el libro (Entrega 7).
//!
//! Sin red: recibe la página ya pedida y decide qué hacer con cada parte.
//!
//! - Tipo desconocido: se ignora y el cursor avanza igual (si no, se pedirían siempre).
//! - Parte con cuenta elegida para su grupo y válida: se aplica al libro.
//! - Parte sin cuenta elegida, o con una cuenta que ya no sirve: queda pendiente.
//! - Parte ya aplicada (por ejemplo, tras reiniciar el cursor): no se hace nada.
//!
//! El resultado es un `LoteLocal`, que `SincronizacionSqlite::guardar_lote` guarda junto
//! con el cursor en una sola transacción (INV-13, INV-14, INV-15).

#![allow(dead_code)] // lo usa el comando de sincronización (Cristina - Tarea 9)

use aplicacion::{CasosDeUso, ErrorAplicacion, ParteDeGrupoEntrante};
use infraestructura::sincronizacion::{LoteLocal, SincronizacionSqlite};

use crate::dto::ErroresDto;
use crate::sesion::sin_campo;
use crate::sincronizacion_cliente::{EventoGastoDeGrupo, PaginaCambios, TIPO_GASTO_DE_GRUPO};

fn interno() -> ErroresDto {
    sin_campo("Ocurrió un problema interno. Intenta de nuevo.")
}

fn almacenamiento() -> ErroresDto {
    sin_campo("No se pudo leer o guardar en el almacenamiento del dispositivo. Intenta de nuevo.")
}

fn a_parte(e: &EventoGastoDeGrupo) -> Result<ParteDeGrupoEntrante, ErroresDto> {
    let incompleto = || sin_campo("El servidor envió un cambio que no se entiende.");
    let grupo = e.grupo.as_ref().ok_or_else(incompleto)?;
    let gasto = e.gasto.as_ref().ok_or_else(incompleto)?;
    Ok(ParteDeGrupoEntrante {
        evento_servidor_id: e.id.clone(),
        posicion: e.posicion,
        grupo_id: grupo.id.clone(),
        grupo_nombre: grupo.nombre.clone(),
        gasto_id: gasto.id.clone(),
        concepto: gasto.concepto.clone(),
        fecha: e.fecha.clone().ok_or_else(incompleto)?,
        moneda: e.moneda.clone().ok_or_else(incompleto)?,
        monto: e.monto.clone().ok_or_else(incompleto)?,
        incluye_redondeo: e.incluye_redondeo.unwrap_or(false),
    })
}

/// Arma el lote de una página. `desde` es el cursor con el que se pidió: el cursor nuevo
/// nunca queda por debajo. La fecha de sincronización se anota solo en la última página.
pub fn armar_lote(
    casos: &CasosDeUso,
    sync: &SincronizacionSqlite,
    pagina: &PaginaCambios,
    desde: i64,
    ahora: &str,
) -> Result<LoteLocal, ErroresDto> {
    let mut lote = LoteLocal::default();
    for e in &pagina.eventos {
        if e.tipo != TIPO_GASTO_DE_GRUPO {
            continue;
        }
        let parte = a_parte(e)?;
        let cuenta = sync
            .cuenta_de_grupo(&parte.grupo_id)
            .map_err(|_| almacenamiento())?;
        let Some(cuenta) = cuenta else {
            lote.pendientes_nuevas.push(parte);
            continue;
        };
        match casos.preparar_parte_de_grupo(parte.con_cuenta(&cuenta)) {
            Ok(Some(evento)) => {
                lote.pendientes_resueltas
                    .push(parte.evento_servidor_id.clone());
                lote.eventos.push(evento);
            }
            Ok(None) => lote
                .pendientes_resueltas
                .push(parte.evento_servidor_id.clone()),
            Err(ErrorAplicacion::Validacion(_)) => lote.pendientes_nuevas.push(parte),
            Err(_) => return Err(interno()),
        }
    }
    let ultima = pagina.eventos.iter().map(|e| e.posicion).max().unwrap_or(0);
    lote.cursor = Some(pagina.cursor.max(ultima).max(desde));
    if !pagina.hay_mas {
        lote.sincronizado_en = Some(ahora.to_string());
    }
    Ok(lote)
}

/// Al elegir la cuenta de un grupo: aplica sus partes pendientes. Las que la cuenta no
/// admite (otra moneda, por ejemplo) siguen pendientes.
pub fn aplicar_pendientes_del_grupo(
    casos: &CasosDeUso,
    sync: &SincronizacionSqlite,
    grupo_id: &str,
    cuenta_id: &str,
) -> Result<LoteLocal, ErroresDto> {
    let mut lote = LoteLocal::default();
    let pendientes = sync.partes_pendientes().map_err(|_| almacenamiento())?;
    for parte in pendientes.into_iter().filter(|p| p.grupo_id == grupo_id) {
        match casos.preparar_parte_de_grupo(parte.con_cuenta(cuenta_id)) {
            Ok(Some(evento)) => {
                lote.eventos.push(evento);
                lote.pendientes_resueltas.push(parte.evento_servidor_id);
            }
            Ok(None) => lote.pendientes_resueltas.push(parte.evento_servidor_id),
            Err(ErrorAplicacion::Validacion(_)) => {}
            Err(_) => return Err(interno()),
        }
    }
    Ok(lote)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::sincronizacion_cliente::{GastoDelEvento, GrupoDelEvento};
    use aplicacion::{DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo};
    use infraestructura::db::BaseDatos;
    use infraestructura::repositorios::RepositorioEventosSqlite;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// `ErroresDto` no implementa `Debug`: en las pruebas se abre con un mensaje fijo.
    fn ok<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(_) => panic!("se esperaba Ok y llegó un error de la aplicación"),
        }
    }

    const AHORA: &str = "2026-11-20T12:00:00Z";

    struct Libro {
        casos: CasosDeUso,
        sync: SincronizacionSqlite,
        ruta: PathBuf,
        _base: BaseDatos,
    }

    impl Drop for Libro {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.ruta);
        }
    }

    fn libro(nombre: &str) -> Libro {
        let ruta = std::env::temp_dir().join(format!("cc_lote_{}_{nombre}.db", std::process::id()));
        let _ = std::fs::remove_file(&ruta);
        let base = BaseDatos::persistente(&ruta).unwrap();
        let conn = base.conexion();
        let casos = CasosDeUso::nuevo(
            Box::new(RepositorioEventosSqlite::with_conn(Arc::clone(&conn))),
            Box::new(RelojFijo::nuevo(AHORA)),
            Box::new(GeneradorSecuencial::nuevo()),
            Box::new(DispositivoFijo::nuevo("disp-1")),
        );
        let sync = SincronizacionSqlite::nuevo(conn).unwrap();
        Libro {
            casos,
            sync,
            ruta,
            _base: base,
        }
    }

    fn cuenta(l: &mut Libro, moneda: &str) -> String {
        l.casos
            .crear_cuenta(NuevaCuenta {
                nombre: format!("Efectivo {moneda}"),
                tipo: "efectivo".to_string(),
                moneda: moneda.to_string(),
                saldo_inicial: "200.00".to_string(),
            })
            .unwrap()
            .id
    }

    fn saldo(l: &Libro, cuenta: &str) -> String {
        l.casos.detalle_cuenta(cuenta).unwrap().cuenta.saldo_actual
    }

    fn movimientos(l: &Libro, cuenta: &str) -> usize {
        l.casos.detalle_cuenta(cuenta).unwrap().movimientos.len()
    }

    fn evento(id: &str, posicion: i64, grupo: &str, monto: &str) -> EventoGastoDeGrupo {
        EventoGastoDeGrupo {
            id: id.to_string(),
            posicion,
            tipo: TIPO_GASTO_DE_GRUPO.to_string(),
            grupo: Some(GrupoDelEvento {
                id: grupo.to_string(),
                nombre: "Viaje a Mérida".to_string(),
            }),
            gasto: Some(GastoDelEvento {
                id: format!("x-{id}"),
                concepto: "Hotel".to_string(),
            }),
            fecha: Some("2026-11-20".to_string()),
            moneda: Some("USD".to_string()),
            monto: Some(monto.to_string()),
            incluye_redondeo: Some(false),
        }
    }

    fn pagina(eventos: Vec<EventoGastoDeGrupo>, cursor: i64, hay_mas: bool) -> PaginaCambios {
        PaginaCambios {
            eventos,
            cursor,
            hay_mas,
        }
    }

    #[test]
    fn una_parte_con_cuenta_se_aplica_y_sin_cuenta_queda_pendiente() {
        let mut l = libro("mezcla");
        let c = cuenta(&mut l, "USD");
        l.sync.elegir_cuenta("g1", &c).unwrap();
        let p = pagina(
            vec![
                evento("e-100", 1, "g1", "33.33"),
                evento("e-101", 2, "g2", "15.00"),
            ],
            2,
            false,
        );
        let lote = ok(armar_lote(&l.casos, &l.sync, &p, 0, AHORA));
        assert_eq!(lote.eventos.len(), 1);
        assert_eq!(lote.pendientes_nuevas.len(), 1);
        assert_eq!(lote.cursor, Some(2));
        assert_eq!(lote.sincronizado_en.as_deref(), Some(AHORA));
        l.sync.guardar_lote(&lote).unwrap();
        assert_eq!(saldo(&l, &c), "166.67");
        assert_eq!(l.sync.partes_pendientes().unwrap().len(), 1);
    }

    #[test]
    fn un_tipo_desconocido_se_ignora_pero_el_cursor_avanza() {
        let l = libro("desconocido");
        let mut ajeno = evento("e-9", 5, "g1", "1.00");
        ajeno.tipo = "tipo_futuro".to_string();
        let p = pagina(vec![ajeno], 5, false);
        let lote = ok(armar_lote(&l.casos, &l.sync, &p, 0, AHORA));
        assert!(lote.eventos.is_empty() && lote.pendientes_nuevas.is_empty());
        assert_eq!(lote.cursor, Some(5));
    }

    #[test]
    fn idempotencia_el_lote_repetido_con_el_cursor_en_cero_no_duplica() {
        let mut l = libro("idempotencia");
        let c = cuenta(&mut l, "USD");
        l.sync.elegir_cuenta("g1", &c).unwrap();
        let p = pagina(vec![evento("e-100", 1, "g1", "33.33")], 1, false);
        for _ in 0..3 {
            let lote = ok(armar_lote(&l.casos, &l.sync, &p, 0, AHORA));
            l.sync.guardar_lote(&lote).unwrap();
        }
        assert_eq!(movimientos(&l, &c), 1);
        assert_eq!(saldo(&l, &c), "166.67");
    }

    #[test]
    fn elegir_la_cuenta_aplica_las_pendientes_de_ese_grupo() {
        let mut l = libro("elegir");
        let c = cuenta(&mut l, "USD");
        let p = pagina(vec![evento("e-100", 1, "g1", "33.33")], 1, false);
        let lote = ok(armar_lote(&l.casos, &l.sync, &p, 0, AHORA));
        l.sync.guardar_lote(&lote).unwrap();
        assert_eq!(l.sync.partes_pendientes().unwrap().len(), 1);
        assert_eq!(movimientos(&l, &c), 0);

        l.sync.elegir_cuenta("g1", &c).unwrap();
        let lote = ok(aplicar_pendientes_del_grupo(&l.casos, &l.sync, "g1", &c));
        l.sync.guardar_lote(&lote).unwrap();
        assert!(l.sync.partes_pendientes().unwrap().is_empty());
        assert_eq!(saldo(&l, &c), "166.67");
    }

    #[test]
    fn una_cuenta_de_otra_moneda_deja_la_parte_pendiente() {
        let mut l = libro("moneda");
        let euros = cuenta(&mut l, "EUR");
        l.sync.elegir_cuenta("g1", &euros).unwrap();
        let p = pagina(vec![evento("e-100", 1, "g1", "33.33")], 1, false);
        let lote = ok(armar_lote(&l.casos, &l.sync, &p, 0, AHORA));
        assert!(lote.eventos.is_empty());
        assert_eq!(lote.pendientes_nuevas.len(), 1);
    }

    #[test]
    fn el_cursor_nunca_retrocede() {
        let l = libro("cursor");
        let lote = ok(armar_lote(
            &l.casos,
            &l.sync,
            &pagina(vec![], 0, false),
            7,
            AHORA,
        ));
        assert_eq!(lote.cursor, Some(7));
    }

    #[test]
    fn una_parte_incompleta_es_un_error_y_no_se_ignora() {
        let l = libro("incompleta");
        let mut roto = evento("e-1", 1, "g1", "1.00");
        roto.monto = None;
        let p = pagina(vec![roto], 1, false);
        assert!(armar_lote(&l.casos, &l.sync, &p, 0, AHORA).is_err());
    }
}
