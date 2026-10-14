use rusqlite::{Connection, Result, params};
use std::path::Path;

/// Abre una conexión SQLite en la ruta indicada y activa claves foráneas.
pub fn abrir<P: AsRef<Path>>(ruta: P) -> Result<Connection> {
    let conn = Connection::open(ruta)?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    conn.execute("PRAGMA journal_mode = WAL;", [])?;
    Ok(conn)
}

/// Inicializa el esquema mínimo para el libro de eventos inmutable.
pub fn inicializar(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS eventos (
            id TEXT PRIMARY KEY,
            tipo TEXT NOT NULL,
            momento_registro TEXT NOT NULL,
            dispositivo_origen TEXT NOT NULL,
            datos TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_eventos_momento ON eventos (momento_registro)",
        [],
    )?;
    Ok(())
}

/// Inserta un evento. Devuelve true si fue insertado, false si ya existía (idempotente).
pub fn insertar_evento(
    conn: &mut Connection,
    id: &str,
    tipo: &str,
    momento_registro: &str,
    dispositivo_origen: &str,
    datos: &str,
) -> Result<bool> {
    let tx = conn.transaction()?;
    let existe: i64 = tx.query_row(
        "SELECT COUNT(*) FROM eventos WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    if existe > 0 {
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, tipo, momento_registro, dispositivo_origen, datos],
    )?;
    tx.commit()?;
    Ok(true)
}

/// Lista todos los eventos ordenados por fila de inserción (rowid).
pub fn listar_eventos(conn: &Connection) -> Result<Vec<EventoRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, tipo, momento_registro, dispositivo_origen, datos FROM eventos ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(EventoRow {
            id: r.get(0)?,
            tipo: r.get(1)?,
            momento_registro: r.get(2)?,
            dispositivo_origen: r.get(3)?,
            datos: r.get(4)?,
        })
    })?;
    let mut eventos = Vec::new();
    for row in rows {
        eventos.push(row?);
    }
    Ok(eventos)
}

#[derive(Debug, Clone)]
pub struct EventoRow {
    pub id: String,
    pub tipo: String,
    pub momento_registro: String,
    pub dispositivo_origen: String,
    pub datos: String,
}
