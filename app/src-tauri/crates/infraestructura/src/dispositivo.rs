use aplicacion::puertos::DispositivoOrigen;
use rusqlite::{Connection, OptionalExtension, Result, params};
use std::sync::{Arc, Mutex};

const CLAVE: &str = "dispositivo_origen";

#[derive(Debug, Clone)]
pub struct DispositivoOrigenSqlite {
    id: String,
}

impl DispositivoOrigenSqlite {
    pub fn nuevo(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        let mut conn = conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(e.to_string()),
        ))?;
        let tx = conn.transaction()?;
        tx.execute(
            "CREATE TABLE IF NOT EXISTS configuracion (
                clave TEXT PRIMARY KEY,
                valor TEXT NOT NULL
            )",
            [],
        )?;
        let existente: Option<String> = tx.query_row(
            "SELECT valor FROM configuracion WHERE clave = ?1",
            params![CLAVE],
            |r| r.get(0),
        ).optional()?;
        let id = if let Some(v) = existente {
            v
        } else {
            let nuevo = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO configuracion (clave, valor) VALUES (?1, ?2)",
                params![CLAVE, nuevo],
            )?;
            nuevo
        };
        tx.commit()?;
        Ok(DispositivoOrigenSqlite { id })
    }
}

impl DispositivoOrigen for DispositivoOrigenSqlite {
    fn id_dispositivo(&self) -> String {
        self.id.clone()
    }
}
