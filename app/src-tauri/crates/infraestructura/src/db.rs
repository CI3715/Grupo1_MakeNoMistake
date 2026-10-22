use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Abre una conexión SQLite en la ruta indicada y activa claves foráneas.
pub fn abrir<P: AsRef<Path>>(ruta: P) -> Result<Connection> {
    let conn = Connection::open(ruta)?;
    // `execute_batch` tolera PRAGMAs que devuelven una fila (p. ej. journal_mode).
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    Ok(conn)
}

/// Inicializa el esquema local: el libro de eventos inmutable (`eventos`), su
/// proyección de lectura (`cuentas`) y la tabla de configuración interna.
///
/// Es idempotente: puede ejecutarse en cada arranque de la aplicación.
pub fn inicializar(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- 1) Libro: registro de eventos inmutable (fuente de verdad).
        CREATE TABLE IF NOT EXISTS eventos (
            id                 TEXT    PRIMARY KEY,
            tipo               TEXT    NOT NULL,
            version            INTEGER NOT NULL DEFAULT 1,
            momento_registro   TEXT    NOT NULL,
            dispositivo_origen TEXT    NOT NULL,
            datos              TEXT    NOT NULL,
            registrado_en      TEXT    NOT NULL
                                       DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE INDEX IF NOT EXISTS idx_eventos_tipo    ON eventos (tipo);
        CREATE INDEX IF NOT EXISTS idx_eventos_momento ON eventos (momento_registro);

        -- 2) Proyección: cuentas (caché de lectura, reconstruible desde eventos).
        CREATE TABLE IF NOT EXISTS cuentas (
            id             TEXT PRIMARY KEY,
            nombre         TEXT NOT NULL,
            tipo           TEXT NOT NULL,
            moneda         TEXT NOT NULL,
            saldo_inicial  TEXT NOT NULL,
            saldo_actual   TEXT NOT NULL,
            estado         TEXT NOT NULL DEFAULT 'activa',
            evento_origen  TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_cuentas_nombre ON cuentas (lower(nombre));

        -- 3) Configuración: pares clave/valor internos (id de dispositivo, etc.).
        CREATE TABLE IF NOT EXISTS configuracion (
            clave TEXT PRIMARY KEY,
            valor TEXT NOT NULL
        );

        -- 4) Inmutabilidad del libro (INV-13): un evento nunca se modifica ni se borra.
        CREATE TRIGGER IF NOT EXISTS eventos_no_update
        BEFORE UPDATE ON eventos
        BEGIN
            SELECT RAISE(ABORT, 'eventos es inmutable: no se puede modificar');
        END;
        CREATE TRIGGER IF NOT EXISTS eventos_no_delete
        BEFORE DELETE ON eventos
        BEGIN
            SELECT RAISE(ABORT, 'eventos es inmutable: no se puede borrar');
        END;
        ",
    )?;
    Ok(())
}

/// Acceso compartido a la base de datos local persistente.
///
/// Abre (o crea) el archivo SQLite, inicializa el esquema y expone la conexión
/// envuelta en `Arc<Mutex<_>>` para compartirla entre los adaptadores y los
/// hilos de la aplicación.
#[derive(Clone)]
pub struct BaseDatos {
    conexion: Arc<Mutex<Connection>>,
}

impl BaseDatos {
    /// Abre (o crea) la base en `ruta` y garantiza el esquema.
    pub fn persistente<P: AsRef<Path>>(ruta: P) -> Result<Self> {
        let conn = abrir(ruta)?;
        inicializar(&conn)?;
        Ok(Self {
            conexion: Arc::new(Mutex::new(conn)),
        })
    }

    /// Devuelve el acceso compartido a la conexión subyacente.
    pub fn conexion(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conexion)
    }
}

/// Inserta un evento dentro de la conexión/transacción dada.
///
/// No abre ni confirma una transacción propia: el llamador controla el límite
/// transaccional para poder escribir el evento y su proyección de forma atómica.
/// Devuelve `true` si fue insertado, `false` si ya existía (idempotente).
pub fn insertar_evento(
    conn: &Connection,
    id: &str,
    tipo: &str,
    momento_registro: &str,
    dispositivo_origen: &str,
    datos: &str,
) -> Result<bool> {
    let existe: i64 = conn.query_row(
        "SELECT COUNT(*) FROM eventos WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    if existe > 0 {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, tipo, momento_registro, dispositivo_origen, datos],
    )?;
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

/// Fila cruda de la proyección `cuentas` tal como se guarda en SQLite.
#[derive(Debug, Clone)]
pub struct CuentaRow {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub moneda: String,
    pub saldo_inicial: String,
    pub saldo_actual: String,
    pub estado: String,
    pub evento_origen: String,
    pub actualizado_en: String,
}

/// Inserta una fila de proyección de cuenta. Es determinista y se usa tanto en
/// `guardar` como en `reconstruir`.
pub fn insertar_cuenta(conn: &Connection, c: &CuentaRow) -> Result<()> {
    conn.execute(
        "INSERT INTO cuentas (id, nombre, tipo, moneda, saldo_inicial, saldo_actual, estado, evento_origen, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            c.id,
            c.nombre,
            c.tipo,
            c.moneda,
            c.saldo_inicial,
            c.saldo_actual,
            c.estado,
            c.evento_origen,
            c.actualizado_en,
        ],
    )?;
    Ok(())
}

/// Lista todas las cuentas proyectadas en orden de inserción (por `rowid`).
pub fn listar_cuentas(conn: &Connection) -> Result<Vec<CuentaRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo, moneda, saldo_inicial, saldo_actual, estado, evento_origen, actualizado_en
         FROM cuentas ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CuentaRow {
            id: r.get(0)?,
            nombre: r.get(1)?,
            tipo: r.get(2)?,
            moneda: r.get(3)?,
            saldo_inicial: r.get(4)?,
            saldo_actual: r.get(5)?,
            estado: r.get(6)?,
            evento_origen: r.get(7)?,
            actualizado_en: r.get(8)?,
        })
    })?;
    let mut cuentas = Vec::new();
    for row in rows {
        cuentas.push(row?);
    }
    Ok(cuentas)
}

/// Elimina todas las filas de la proyección `cuentas`.
pub fn borrar_cuentas(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM cuentas", [])?;
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn conexion_vacia() -> Connection {
        let conn = Connection::open_in_memory().expect("abrir base en memoria");
        inicializar(&conn).expect("inicializar esquema");
        conn
    }

    fn tabla_existe(conn: &Connection, tabla: &str) -> bool {
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![tabla],
                |r| r.get(0),
            )
            .expect("consultar sqlite_master");
        total == 1
    }

    #[test]
    fn crea_las_tres_tablas() {
        let conn = conexion_vacia();
        assert!(tabla_existe(&conn, "eventos"), "falta la tabla eventos");
        assert!(tabla_existe(&conn, "cuentas"), "falta la tabla cuentas");
        assert!(
            tabla_existe(&conn, "configuracion"),
            "falta la tabla configuracion"
        );
    }

    #[test]
    fn inicializar_es_idempotente() {
        let conn = conexion_vacia();
        inicializar(&conn).expect("segunda inicialización");
    }

    #[test]
    fn insertar_evento_es_idempotente() {
        let conn = conexion_vacia();
        let primero = insertar_evento(
            &conn,
            "e1",
            "CuentaCreada",
            "2026-01-01T00:00:00Z",
            "d1",
            "{}",
        )
        .expect("insertar");
        let segundo = insertar_evento(
            &conn,
            "e1",
            "CuentaCreada",
            "2026-01-01T00:00:00Z",
            "d1",
            "{}",
        )
        .expect("reinsertar");
        assert!(primero, "el primer evento debe insertarse");
        assert!(!segundo, "el evento repetido no debe insertarse");
        assert_eq!(listar_eventos(&conn).unwrap().len(), 1);
    }

    #[test]
    fn nombre_de_cuenta_unico_sin_distinguir_mayusculas() {
        let conn = conexion_vacia();
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos) VALUES ('e1', 'CuentaCreada', '2026-01-01T00:00:00Z', 'd1', '{}')",
            [],
        )
        .unwrap();
        let insertar_cuenta = |nombre: &str| {
            conn.execute(
                "INSERT INTO cuentas (id, nombre, tipo, moneda, saldo_inicial, saldo_actual, evento_origen, actualizado_en)
                 VALUES (?1, ?2, 'efectivo', 'VES', '0.00', '0.00', 'e1', '2026-01-01T00:00:00Z')",
                params![format!("c-{nombre}"), nombre],
            )
        };
        insertar_cuenta("Cartera").expect("primera cuenta");
        assert!(
            insertar_cuenta("cartera").is_err(),
            "no debe permitir dos cuentas con el mismo nombre sin distinguir mayúsculas"
        );
    }

    #[test]
    fn configuracion_guarda_y_recupera() {
        let conn = conexion_vacia();
        conn.execute(
            "INSERT INTO configuracion (clave, valor) VALUES ('dispositivo_origen', 'abc')",
            [],
        )
        .unwrap();
        let valor: String = conn
            .query_row(
                "SELECT valor FROM configuracion WHERE clave = 'dispositivo_origen'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(valor, "abc");
    }

    #[test]
    fn persistente_crea_archivo_y_sobrevive_al_cierre() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let ruta = std::env::temp_dir().join(format!("cuentas_claras_prueba_{nanos}.db"));

        // Primera sesión: se crean las tablas en el archivo.
        {
            let bd = BaseDatos::persistente(&ruta).expect("abrir base persistente");
            let conn = bd.conexion();
            let conn = conn.lock().unwrap();
            assert!(tabla_existe(&conn, "eventos"));
            assert!(tabla_existe(&conn, "cuentas"));
            assert!(tabla_existe(&conn, "configuracion"));
        }
        assert!(ruta.exists(), "el archivo SQLite debe existir en disco");

        // Segunda sesión: se reabre el mismo archivo sin errores.
        {
            BaseDatos::persistente(&ruta).expect("reabrir base persistente");
        }

        let _ = std::fs::remove_file(&ruta);
        let _ = std::fs::remove_file(ruta.with_extension("db-wal"));
        let _ = std::fs::remove_file(ruta.with_extension("db-shm"));
    }

    #[test]
    fn eventos_no_se_pueden_modificar_ni_borrar() {
        let conn = conexion_vacia();
        insertar_evento(
            &conn,
            "e1",
            "CuentaCreada",
            "2026-01-01T00:00:00Z",
            "d1",
            "{}",
        )
        .expect("insertar");

        assert!(
            conn.execute("UPDATE eventos SET tipo = 'Otro' WHERE id = 'e1'", [])
                .is_err(),
            "UPDATE sobre eventos debe rechazarse"
        );
        assert!(
            conn.execute("DELETE FROM eventos WHERE id = 'e1'", [])
                .is_err(),
            "DELETE sobre eventos debe rechazarse"
        );

        let eventos = listar_eventos(&conn).unwrap();
        assert_eq!(eventos.len(), 1);
        assert_eq!(eventos[0].tipo, "CuentaCreada");
    }
}
