//! Estado local de la sincronización (Entrega 7): cursor, cuenta elegida por grupo y
//! partes pendientes de aplicar.
//!
//! Vive en el mismo archivo SQLite del libro de cada usuario (ADR-014), en tablas propias
//! fuera de `eventos`. Nada de esto viaja al servidor (INV-16).
//!
//! `guardar_lote` guarda los eventos nuevos del libro, los cambios de pendientes y el
//! cursor en UNA sola transacción: si algo falla a mitad no queda nada a medias y el
//! cursor no avanza. La siguiente sincronización vuelve a pedir desde el cursor anterior
//! y la comprobación por id evita duplicados (INV-13, INV-14, INV-15).

use std::sync::{Arc, Mutex};

use aplicacion::ParteDeGrupoEntrante;
use aplicacion::puertos::{ErrorRepositorio, EventoRegistrado};
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::insertar_evento;
use crate::repositorios::proyectar_evento;

const CLAVE_CURSOR: &str = "sync_cursor";
const CLAVE_ULTIMA: &str = "sync_ultima";

fn error(e: impl std::fmt::Display) -> ErrorRepositorio {
    ErrorRepositorio(e.to_string())
}

fn guardar_clave(conn: &Connection, clave: &str, valor: &str) -> Result<(), ErrorRepositorio> {
    conn.execute(
        "INSERT INTO configuracion (clave, valor) VALUES (?1, ?2)
         ON CONFLICT(clave) DO UPDATE SET valor = excluded.valor",
        params![clave, valor],
    )
    .map_err(error)?;
    Ok(())
}

/// Todo lo que cambia en un paso de sincronización, para guardarlo junto.
#[derive(Default)]
pub struct LoteLocal {
    /// Eventos del libro (partes aplicadas a una cuenta).
    pub eventos: Vec<EventoRegistrado>,
    /// Partes que bajaron pero aún no tienen cuenta donde aplicarse.
    pub pendientes_nuevas: Vec<ParteDeGrupoEntrante>,
    /// Ids de eventos del servidor que dejan de ser pendientes (ya se aplicaron).
    pub pendientes_resueltas: Vec<String>,
    /// Cursor nuevo; `None` si no cambia.
    pub cursor: Option<i64>,
    /// Fecha y hora de la sincronización exitosa; `None` si no cambia.
    pub sincronizado_en: Option<String>,
}

/// Un gasto de grupo que espera ser enviado al servidor. `estado` es "pendiente" o
/// "rechazado"; `cuerpo` es el JSON que se enviará tal cual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GastoEncolado {
    pub clave: String,
    pub grupo_id: String,
    pub grupo_nombre: String,
    pub concepto: String,
    pub monto: String,
    pub moneda: String,
    pub fecha: String,
    pub cuerpo: String,
    pub estado: String,
    pub motivo: Option<String>,
}

const COLA_SELECT: &str = "SELECT clave, grupo_id, grupo_nombre, concepto, monto, moneda,
                                  fecha, cuerpo, estado, motivo FROM cola_envio";

pub struct SincronizacionSqlite {
    conn: Arc<Mutex<Connection>>,
}

impl SincronizacionSqlite {
    /// Crea las tablas si no existen (como hace `DispositivoOrigenSqlite` con la suya).
    pub fn nuevo(conn: Arc<Mutex<Connection>>) -> Result<Self, ErrorRepositorio> {
        {
            let c = conn.lock().map_err(error)?;
            c.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS configuracion (
                    clave TEXT PRIMARY KEY,
                    valor TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS cuenta_de_grupo (
                    grupo_id  TEXT PRIMARY KEY,
                    cuenta_id TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS partes_pendientes (
                    evento_id        TEXT    PRIMARY KEY,
                    posicion         INTEGER NOT NULL,
                    grupo_id         TEXT    NOT NULL,
                    grupo_nombre     TEXT    NOT NULL,
                    gasto_id         TEXT    NOT NULL,
                    concepto         TEXT    NOT NULL,
                    fecha            TEXT    NOT NULL,
                    moneda           TEXT    NOT NULL,
                    monto            TEXT    NOT NULL,
                    incluye_redondeo INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS cola_envio (
                    orden        INTEGER PRIMARY KEY AUTOINCREMENT,
                    clave        TEXT    NOT NULL UNIQUE,
                    grupo_id     TEXT    NOT NULL,
                    grupo_nombre TEXT    NOT NULL,
                    concepto     TEXT    NOT NULL,
                    monto        TEXT    NOT NULL,
                    moneda       TEXT    NOT NULL,
                    fecha        TEXT    NOT NULL,
                    cuerpo       TEXT    NOT NULL,
                    estado       TEXT    NOT NULL,
                    motivo       TEXT
                );
                ",
            )
            .map_err(error)?;
        }
        Ok(Self { conn })
    }

    fn leer(&self, sql: &str, parametro: &str) -> Result<Option<String>, ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        conn.query_row(sql, params![parametro], |f| f.get::<_, String>(0))
            .optional()
            .map_err(error)
    }

    /// Posición del último evento guardado; 0 si aún no hubo sincronización.
    pub fn cursor(&self) -> Result<i64, ErrorRepositorio> {
        let valor = self.leer(
            "SELECT valor FROM configuracion WHERE clave = ?1",
            CLAVE_CURSOR,
        )?;
        Ok(valor.and_then(|v| v.parse().ok()).unwrap_or(0))
    }

    pub fn ultima_sincronizacion(&self) -> Result<Option<String>, ErrorRepositorio> {
        self.leer(
            "SELECT valor FROM configuracion WHERE clave = ?1",
            CLAVE_ULTIMA,
        )
    }

    /// Pone el cursor en cero (para la demostración): la siguiente sincronización baja
    /// todo otra vez y la comprobación por id evita duplicados.
    pub fn reiniciar_cursor(&self) -> Result<(), ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        guardar_clave(&conn, CLAVE_CURSOR, "0")
    }

    pub fn cuenta_de_grupo(&self, grupo_id: &str) -> Result<Option<String>, ErrorRepositorio> {
        self.leer(
            "SELECT cuenta_id FROM cuenta_de_grupo WHERE grupo_id = ?1",
            grupo_id,
        )
    }

    /// Elige (o cambia) la cuenta del grupo. Afecta solo a las partes que lleguen después.
    pub fn elegir_cuenta(&self, grupo_id: &str, cuenta_id: &str) -> Result<(), ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        conn.execute(
            "INSERT INTO cuenta_de_grupo (grupo_id, cuenta_id) VALUES (?1, ?2)
             ON CONFLICT(grupo_id) DO UPDATE SET cuenta_id = excluded.cuenta_id",
            params![grupo_id, cuenta_id],
        )
        .map_err(error)?;
        Ok(())
    }

    /// Agrega un gasto al final de la cola. Si la clave ya estaba, no hace nada.
    pub fn encolar(&self, g: &GastoEncolado) -> Result<(), ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        conn.execute(
            "INSERT OR IGNORE INTO cola_envio
             (clave, grupo_id, grupo_nombre, concepto, monto, moneda, fecha, cuerpo, estado, motivo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                g.clave,
                g.grupo_id,
                g.grupo_nombre,
                g.concepto,
                g.monto,
                g.moneda,
                g.fecha,
                g.cuerpo,
                g.estado,
                g.motivo
            ],
        )
        .map_err(error)?;
        Ok(())
    }

    fn leer_cola(&self, solo_pendientes: bool) -> Result<Vec<GastoEncolado>, ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        let filtro = if solo_pendientes {
            " WHERE estado = 'pendiente'"
        } else {
            ""
        };
        let sql = format!("{COLA_SELECT}{filtro} ORDER BY orden");
        let mut sentencia = conn.prepare(&sql).map_err(error)?;
        let filas = sentencia
            .query_map([], |f| {
                Ok(GastoEncolado {
                    clave: f.get(0)?,
                    grupo_id: f.get(1)?,
                    grupo_nombre: f.get(2)?,
                    concepto: f.get(3)?,
                    monto: f.get(4)?,
                    moneda: f.get(5)?,
                    fecha: f.get(6)?,
                    cuerpo: f.get(7)?,
                    estado: f.get(8)?,
                    motivo: f.get(9)?,
                })
            })
            .map_err(error)?;
        filas.collect::<Result<Vec<_>, _>>().map_err(error)
    }

    /// Toda la cola (pendientes y rechazados), en orden de llegada.
    pub fn cola(&self) -> Result<Vec<GastoEncolado>, ErrorRepositorio> {
        self.leer_cola(false)
    }

    /// Solo los que faltan por enviar, en orden de llegada.
    pub fn cola_pendiente(&self) -> Result<Vec<GastoEncolado>, ErrorRepositorio> {
        self.leer_cola(true)
    }

    /// El servidor lo rechazó: deja de reintentarse y guarda el motivo.
    pub fn marcar_rechazada(&self, clave: &str, motivo: &str) -> Result<(), ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        conn.execute(
            "UPDATE cola_envio SET estado = 'rechazado', motivo = ?2 WHERE clave = ?1",
            params![clave, motivo],
        )
        .map_err(error)?;
        Ok(())
    }

    /// Lo quita de la cola (ya se envió, o el usuario lo descartó).
    pub fn quitar(&self, clave: &str) -> Result<(), ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        conn.execute("DELETE FROM cola_envio WHERE clave = ?1", params![clave])
            .map_err(error)?;
        Ok(())
    }

    /// Partes que esperan una cuenta, en el orden en que llegaron.
    pub fn partes_pendientes(&self) -> Result<Vec<ParteDeGrupoEntrante>, ErrorRepositorio> {
        let conn = self.conn.lock().map_err(error)?;
        let mut sentencia = conn
            .prepare(
                "SELECT evento_id, posicion, grupo_id, grupo_nombre, gasto_id, concepto,
                        fecha, moneda, monto, incluye_redondeo
                 FROM partes_pendientes ORDER BY posicion, evento_id",
            )
            .map_err(error)?;
        let filas = sentencia
            .query_map([], |f| {
                Ok(ParteDeGrupoEntrante {
                    evento_servidor_id: f.get(0)?,
                    posicion: f.get(1)?,
                    grupo_id: f.get(2)?,
                    grupo_nombre: f.get(3)?,
                    gasto_id: f.get(4)?,
                    concepto: f.get(5)?,
                    fecha: f.get(6)?,
                    moneda: f.get(7)?,
                    monto: f.get(8)?,
                    incluye_redondeo: f.get::<_, i64>(9)? != 0,
                })
            })
            .map_err(error)?;
        filas.collect::<Result<Vec<_>, _>>().map_err(error)
    }

    /// Guarda el lote completo en una sola transacción. Un evento que ya estaba en el
    /// libro se omite sin proyectarlo otra vez (aplicar dos veces = aplicar una).
    pub fn guardar_lote(&self, lote: &LoteLocal) -> Result<(), ErrorRepositorio> {
        let mut conn = self.conn.lock().map_err(error)?;
        let tx = conn.transaction().map_err(error)?;

        for e in &lote.eventos {
            let nuevo = insertar_evento(
                &tx,
                &e.id,
                &e.tipo,
                &e.momento_registro,
                &e.dispositivo_origen,
                &e.datos,
            )
            .map_err(error)?;
            if nuevo {
                proyectar_evento(&tx, e)?;
            }
        }
        for p in &lote.pendientes_nuevas {
            tx.execute(
                "INSERT OR IGNORE INTO partes_pendientes
                 (evento_id, posicion, grupo_id, grupo_nombre, gasto_id, concepto,
                  fecha, moneda, monto, incluye_redondeo)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    p.evento_servidor_id,
                    p.posicion,
                    p.grupo_id,
                    p.grupo_nombre,
                    p.gasto_id,
                    p.concepto,
                    p.fecha,
                    p.moneda,
                    p.monto,
                    i64::from(p.incluye_redondeo)
                ],
            )
            .map_err(error)?;
        }
        for id in &lote.pendientes_resueltas {
            tx.execute(
                "DELETE FROM partes_pendientes WHERE evento_id = ?1",
                params![id],
            )
            .map_err(error)?;
        }
        if let Some(cursor) = lote.cursor {
            guardar_clave(&tx, CLAVE_CURSOR, &cursor.to_string())?;
        }
        if let Some(cuando) = &lote.sincronizado_en {
            guardar_clave(&tx, CLAVE_ULTIMA, cuando)?;
        }
        // Si algo falló antes, `tx` se descarta sin confirmar y SQLite lo deshace todo.
        tx.commit().map_err(error)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::db::BaseDatos;
    use crate::repositorios::RepositorioEventosSqlite;
    use aplicacion::{CasosDeUso, DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo};

    struct Libro {
        casos: CasosDeUso,
        sync: SincronizacionSqlite,
        cuenta: String,
        ruta: std::path::PathBuf,
        _base: BaseDatos,
    }

    impl Drop for Libro {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.ruta);
        }
    }

    fn libro(nombre: &str) -> Libro {
        let ruta = std::env::temp_dir().join(format!("cc_sync_{}_{nombre}.db", std::process::id()));
        let _ = std::fs::remove_file(&ruta);
        let base = BaseDatos::persistente(&ruta).unwrap();
        let conn = base.conexion();
        let mut casos = CasosDeUso::nuevo(
            Box::new(RepositorioEventosSqlite::with_conn(Arc::clone(&conn))),
            Box::new(RelojFijo::nuevo("2026-11-20T12:00:00Z")),
            Box::new(GeneradorSecuencial::nuevo()),
            Box::new(DispositivoFijo::nuevo("disp-1")),
        );
        let cuenta = casos
            .crear_cuenta(NuevaCuenta {
                nombre: "Efectivo USD".to_string(),
                tipo: "efectivo".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "200.00".to_string(),
            })
            .unwrap()
            .id;
        let sync = SincronizacionSqlite::nuevo(conn).unwrap();
        Libro {
            casos,
            sync,
            cuenta,
            ruta,
            _base: base,
        }
    }

    fn parte(id: &str, posicion: i64, monto: &str) -> ParteDeGrupoEntrante {
        ParteDeGrupoEntrante {
            evento_servidor_id: id.to_string(),
            posicion,
            grupo_id: "g1".to_string(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: format!("gasto-{id}"),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto: monto.to_string(),
            incluye_redondeo: false,
        }
    }

    fn evento_de(l: &Libro, p: &ParteDeGrupoEntrante) -> EventoRegistrado {
        l.casos
            .preparar_parte_de_grupo(p.con_cuenta(&l.cuenta))
            .unwrap()
            .expect("la parte aún no estaba aplicada")
    }

    /// Un evento que la proyección no puede procesar: rompe el lote a la mitad.
    fn evento_roto() -> EventoRegistrado {
        EventoRegistrado {
            id: "e-roto".to_string(),
            tipo: "GastoDeGrupoAplicado".to_string(),
            momento_registro: "2026-11-20T12:00:00Z".to_string(),
            dispositivo_origen: "disp-1".to_string(),
            datos: "{}".to_string(),
        }
    }

    fn saldo(l: &Libro) -> String {
        l.casos
            .detalle_cuenta(&l.cuenta)
            .unwrap()
            .cuenta
            .saldo_actual
    }

    fn movimientos(l: &Libro) -> usize {
        l.casos.detalle_cuenta(&l.cuenta).unwrap().movimientos.len()
    }

    #[test]
    fn interrupcion_a_mitad_de_lote_no_deja_nada_a_medias_ni_mueve_el_cursor() {
        let l = libro("interrupcion");
        let (a, b) = (parte("e-100", 1, "33.33"), parte("e-101", 2, "15.00"));
        let lote = LoteLocal {
            eventos: vec![evento_de(&l, &a), evento_de(&l, &b), evento_roto()],
            cursor: Some(2),
            sincronizado_en: Some("2026-11-20T12:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(l.sync.guardar_lote(&lote).is_err());
        assert_eq!(
            movimientos(&l),
            0,
            "no debía quedar ningún movimiento a medias"
        );
        assert_eq!(saldo(&l), "200.00");
        assert_eq!(l.sync.cursor().unwrap(), 0, "el cursor no debía avanzar");
        assert_eq!(l.sync.ultima_sincronizacion().unwrap(), None);

        // La siguiente sincronización pide desde el último cursor guardado y termina el trabajo.
        let lote = LoteLocal {
            eventos: vec![evento_de(&l, &a), evento_de(&l, &b)],
            cursor: Some(2),
            ..Default::default()
        };
        l.sync.guardar_lote(&lote).unwrap();
        assert_eq!(movimientos(&l), 2);
        assert_eq!(saldo(&l), "151.67");
        assert_eq!(l.sync.cursor().unwrap(), 2);
    }

    #[test]
    fn idempotencia_repetir_el_mismo_lote_no_duplica_movimientos() {
        let l = libro("idempotencia");
        let a = parte("e-100", 1, "33.33");
        let evento = evento_de(&l, &a);
        for _ in 0..3 {
            let lote = LoteLocal {
                eventos: vec![evento.clone()],
                cursor: Some(1),
                ..Default::default()
            };
            l.sync.guardar_lote(&lote).unwrap();
        }
        assert_eq!(movimientos(&l), 1);
        assert_eq!(saldo(&l), "166.67");

        // Con el cursor reiniciado la parte vuelve a bajar y se reconoce como ya aplicada.
        l.sync.reiniciar_cursor().unwrap();
        assert_eq!(l.sync.cursor().unwrap(), 0);
        let otra_vez = l
            .casos
            .preparar_parte_de_grupo(a.con_cuenta(&l.cuenta))
            .unwrap();
        assert!(otra_vez.is_none());
    }

    #[test]
    fn las_partes_pendientes_se_guardan_y_se_resuelven_en_el_mismo_lote_que_las_aplica() {
        let l = libro("pendientes");
        let a = parte("e-100", 1, "33.33");
        for _ in 0..2 {
            let lote = LoteLocal {
                pendientes_nuevas: vec![a.clone()],
                cursor: Some(1),
                ..Default::default()
            };
            l.sync.guardar_lote(&lote).unwrap();
        }
        assert_eq!(l.sync.partes_pendientes().unwrap(), vec![a.clone()]);
        assert_eq!(movimientos(&l), 0);

        let lote = LoteLocal {
            eventos: vec![evento_de(&l, &a)],
            pendientes_resueltas: vec![a.evento_servidor_id.clone()],
            ..Default::default()
        };
        l.sync.guardar_lote(&lote).unwrap();
        assert!(l.sync.partes_pendientes().unwrap().is_empty());
        assert_eq!(movimientos(&l), 1);
        assert_eq!(saldo(&l), "166.67");
    }

    #[test]
    fn la_cuenta_del_grupo_el_cursor_y_la_fecha_se_guardan_y_se_pueden_cambiar() {
        let l = libro("estado");
        assert_eq!(l.sync.cursor().unwrap(), 0);
        assert_eq!(l.sync.cuenta_de_grupo("g1").unwrap(), None);
        l.sync.elegir_cuenta("g1", "c1").unwrap();
        l.sync.elegir_cuenta("g1", "c2").unwrap();
        assert_eq!(l.sync.cuenta_de_grupo("g1").unwrap().as_deref(), Some("c2"));
        let lote = LoteLocal {
            cursor: Some(7),
            sincronizado_en: Some("2026-11-20T12:00:00Z".to_string()),
            ..Default::default()
        };
        l.sync.guardar_lote(&lote).unwrap();
        assert_eq!(l.sync.cursor().unwrap(), 7);
        assert_eq!(
            l.sync.ultima_sincronizacion().unwrap().as_deref(),
            Some("2026-11-20T12:00:00Z")
        );
    }
}
