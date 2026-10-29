use aplicacion::{codec, unidades};
use dominio::{TipoMovimiento, categorias_de_fabrica};
use rusqlite::{Connection, Result, params};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn fecha_a_dia_mes(fecha: &str) -> (i64, i64) {
    // Espera formato YYYY-MM-DD
    let mut partes = fecha.split('-');
    let _anio = partes.next().and_then(|s| s.parse::<i64>().ok());
    let mes = partes
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let dia = partes
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    (dia, mes)
}

pub fn fecha_a_dia_mes_pub(fecha: &str) -> (i64, i64) {
    fecha_a_dia_mes(fecha)
}

/// Montos guardados antes de la Entrega 4 como texto: todas las monedas tenían
/// 2 decimales, así que su escala en unidades mínimas es 10^2. Un texto que no
/// se pueda convertir queda en 0; la proyección se reconstruye desde los
/// eventos justo después de migrar.
fn texto_legado_a_unidades(s: &str) -> i64 {
    unidades::a_unidades(s, 2).unwrap_or(0)
}

/// Abre una conexión SQLite en la ruta indicada y activa claves foráneas.
pub fn abrir<P: AsRef<Path>>(ruta: P) -> Result<Connection> {
    let conn = Connection::open(ruta)?;
    // `execute_batch` tolera PRAGMAs que devuelven una fila (p. ej. journal_mode).
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    Ok(conn)
}

/// Tipos de evento del libro inmutable que afectan a las proyecciones de
/// movimientos y categorías. El esquema no los restringe: `eventos.tipo` es una
/// etiqueta de texto, pero se centralizan aquí para la semilla y los adaptadores.
pub const TIPO_CATEGORIA_CREADA: &str = "CategoriaCreada";
pub const TIPO_MOVIMIENTO_REGISTRADO: &str = "MovimientoRegistrado";
pub const TIPO_MOVIMIENTO_CORREGIDO: &str = "MovimientoCorregido";
pub const TIPO_TASA_REGISTRADA: &str = "TasaRegistrada";
pub const TIPO_TASA_ELIMINADA: &str = "TasaEliminada";
pub const TIPO_MONEDA_BASE_CAMBIADA: &str = "MonedaBaseCambiada";
pub const TIPO_MONEDA_CREADA: &str = "MonedaCreada";

/// Tipo declarado de una columna (`"TEXT"`, `"INTEGER"`, ...), o `None` si no existe.
fn tipo_de_columna(conn: &Connection, tabla: &str, columna: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({tabla})"))?;
    let columnas = stmt.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
    for fila in columnas {
        let (nombre, tipo) = fila?;
        if nombre == columna {
            return Ok(Some(tipo));
        }
    }
    Ok(None)
}

/// Fila de `cuentas` con los saldos todavía en texto (antes de la migración).
type FilaCuentaLegada = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

/// Fila de `movimientos` con el monto todavía en texto (antes de la migración).
type FilaMovimientoLegada = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
);

/// Migra una base existente a la estructura con montos enteros en unidades
/// mínimas y columnas dia/mes.
fn migrar_si_necesario(conn: &Connection) -> Result<()> {
    // Desactivar FKs durante migración para poder recrear tablas
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    // Limpiar tablas temporales si quedaron de migraciones fallidas
    let _ = conn
        .execute_batch("DROP TABLE IF EXISTS cuentas_new; DROP TABLE IF EXISTS movimientos_new;");

    // Si no existe la tabla movimientos (base nueva aún sin inicializar), no migrar
    let existe_movimientos: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='movimientos'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if existe_movimientos == 0 {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        return Ok(());
    }

    // Comprobar si movimientos tiene la columna dia
    if tipo_de_columna(conn, "movimientos", "dia")?.is_none() {
        // Añadir columnas si no existen
        let _ = conn.execute_batch(
            "ALTER TABLE movimientos ADD COLUMN dia INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE movimientos ADD COLUMN mes INTEGER NOT NULL DEFAULT 0;",
        );
        // Poblar dia/mes desde fecha
        let mut stmt = conn.prepare("SELECT id, fecha FROM movimientos")?;
        let movimientos: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (id, fecha) in movimientos {
            let (dia, mes) = fecha_a_dia_mes(&fecha);
            let _ = conn.execute(
                "UPDATE movimientos SET dia = ?1, mes = ?2 WHERE id = ?3",
                params![dia, mes, id],
            );
        }
    }

    // Migrar cuentas de TEXT a INTEGER si es necesario
    let tipo_saldo_inicial = tipo_de_columna(conn, "cuentas", "saldo_inicial")?.unwrap_or_default();
    if tipo_saldo_inicial.to_uppercase().contains("TEXT") {
        // Crear tabla nueva, copiar datos convirtiendo a centavos
        conn.execute_batch(
            "CREATE TABLE cuentas_new (
                id             TEXT PRIMARY KEY,
                nombre         TEXT NOT NULL,
                tipo           TEXT NOT NULL,
                moneda         TEXT NOT NULL,
                saldo_inicial  INTEGER NOT NULL,
                saldo_actual   INTEGER NOT NULL,
                estado         TEXT NOT NULL DEFAULT 'activa',
                evento_origen  TEXT NOT NULL,
                actualizado_en TEXT NOT NULL
            );",
        )?;
        let mut stmt = conn.prepare("SELECT id, nombre, tipo, moneda, saldo_inicial, saldo_actual, estado, evento_origen, actualizado_en FROM cuentas")?;
        let cuentas: Vec<FilaCuentaLegada> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for c in cuentas {
            let si = texto_legado_a_unidades(&c.4);
            let sa = texto_legado_a_unidades(&c.5);
            let _ = conn.execute(
                "INSERT INTO cuentas_new (id, nombre, tipo, moneda, saldo_inicial, saldo_actual, estado, evento_origen, actualizado_en) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![c.0, c.1, c.2, c.3, si, sa, c.6, c.7, c.8],
            );
        }
        conn.execute_batch("DROP TABLE cuentas; ALTER TABLE cuentas_new RENAME TO cuentas; CREATE UNIQUE INDEX IF NOT EXISTS idx_cuentas_nombre ON cuentas (lower(nombre));")?;
    }

    // Migrar movimientos de TEXT monto a INTEGER centavos si es necesario
    let tipo_monto = tipo_de_columna(conn, "movimientos", "monto")?.unwrap_or_default();
    if tipo_monto.to_uppercase().contains("TEXT") {
        // Asegurar columnas dia/mes
        let _ = conn.execute_batch(
            "ALTER TABLE movimientos ADD COLUMN dia INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE movimientos ADD COLUMN mes INTEGER NOT NULL DEFAULT 0;",
        );
        conn.execute_batch(
            "CREATE TABLE movimientos_new (
                id                   TEXT PRIMARY KEY,
                cuenta_id            TEXT NOT NULL,
                tipo                 TEXT NOT NULL CHECK (tipo IN ('ingreso','egreso')),
                monto                INTEGER NOT NULL CHECK (typeof(monto) = 'integer' AND monto > 0),
                fecha                TEXT NOT NULL
                                      CHECK (fecha GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                dia                  INTEGER NOT NULL,
                mes                  INTEGER NOT NULL,
                categoria_id         TEXT NOT NULL,
                nota                 TEXT,
                estado               TEXT NOT NULL DEFAULT 'vigente'
                                      CHECK (estado IN ('vigente','corregido')),
                reemplaza_a          TEXT,
                corregido_por_evento TEXT,
                evento_origen        TEXT NOT NULL,
                actualizado_en       TEXT NOT NULL
            );",
        )?;
        let mut stmt = conn.prepare("SELECT id, cuenta_id, tipo, monto, fecha, categoria_id, nota, estado, reemplaza_a, corregido_por_evento, evento_origen, actualizado_en FROM movimientos")?;
        let movimientos: Vec<FilaMovimientoLegada> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for m in movimientos {
            let monto_cents = texto_legado_a_unidades(&m.3);
            if monto_cents <= 0 {
                continue;
            }
            let (dia, mes) = fecha_a_dia_mes(&m.4);
            let _ = conn.execute(
                "INSERT INTO movimientos_new (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, nota, estado, reemplaza_a, corregido_por_evento, evento_origen, actualizado_en) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![m.0, m.1, m.2, monto_cents, m.4, dia, mes, m.5, m.6, m.7, m.8, m.9, m.10, m.11],
            );
        }
        conn.execute_batch("DROP TABLE movimientos; ALTER TABLE movimientos_new RENAME TO movimientos; CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta ON movimientos (cuenta_id); CREATE INDEX IF NOT EXISTS idx_movimientos_fecha ON movimientos (fecha); CREATE INDEX IF NOT EXISTS idx_movimientos_categoria ON movimientos (categoria_id); CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta_estado_fecha ON movimientos (cuenta_id, estado, fecha); CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta_mes_dia ON movimientos (cuenta_id, mes, dia); CREATE UNIQUE INDEX IF NOT EXISTS idx_movimientos_reemplaza ON movimientos (reemplaza_a) WHERE reemplaza_a IS NOT NULL;")?;
    } else {
        // Ya es INTEGER pero puede faltar dia/mes
        let _ = conn.execute_batch(
            "ALTER TABLE movimientos ADD COLUMN dia INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE movimientos ADD COLUMN mes INTEGER NOT NULL DEFAULT 0;",
        );
        let mut stmt =
            conn.prepare("SELECT id, fecha FROM movimientos WHERE dia = 0 OR mes = 0")?;
        let movimientos: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (id, fecha) in movimientos {
            let (dia, mes) = fecha_a_dia_mes(&fecha);
            let _ = conn.execute(
                "UPDATE movimientos SET dia = ?1, mes = ?2 WHERE id = ?3",
                params![dia, mes, id],
            );
        }
    }

    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

/// Inicializa el esquema local: el libro de eventos inmutable (`eventos`), sus
/// proyecciones de lectura (`cuentas`, `categorias` y `movimientos`) y la tabla
/// de configuración interna.
///
/// Es idempotente: puede ejecutarse en cada arranque de la aplicación.
pub fn inicializar(conn: &Connection) -> Result<()> {
    migrar_si_necesario(conn)?;
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
            saldo_inicial  INTEGER NOT NULL,
            saldo_actual   INTEGER NOT NULL,
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

        -- 4) Proyección: categorías de clasificación de movimientos.
        --    La unicidad es por tipo: 'Otros gastos' y 'Otros ingresos' conviven.
        CREATE TABLE IF NOT EXISTS categorias (
            id             TEXT PRIMARY KEY,
            nombre         TEXT NOT NULL,
            tipo           TEXT NOT NULL CHECK (tipo IN ('ingreso','egreso')),
            es_fabrica     INTEGER NOT NULL DEFAULT 0 CHECK (es_fabrica IN (0,1)),
            estado         TEXT NOT NULL DEFAULT 'activa'
                                CHECK (estado IN ('activa','inactiva')),
            evento_origen  TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_categorias_nombre
            ON categorias (tipo, lower(nombre));

        -- 5) Proyección: movimientos de cada cuenta.
        --    - `monto` se guarda como entero en unidades mínimas de la moneda de la
        --      cuenta: monto × 10^decimales (centavos para las de fábrica).
        --    - `dia` y `mes` se derivan de `fecha` (YYYY-MM-DD) para búsquedas rápidas.
        --    - `estado` distingue el movimiento vigente del corregido.
        --    - `reemplaza_a` apunta al movimiento original cuando esta fila es una
        --      corrección. La corrección nunca borra ni sobrescribe la original.
        CREATE TABLE IF NOT EXISTS movimientos (
            id                   TEXT PRIMARY KEY,
            cuenta_id            TEXT NOT NULL REFERENCES cuentas(id),
            tipo                 TEXT NOT NULL CHECK (tipo IN ('ingreso','egreso')),
            monto                INTEGER NOT NULL CHECK (typeof(monto) = 'integer' AND monto > 0),
            fecha                TEXT NOT NULL
                                  CHECK (fecha GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            dia                  INTEGER NOT NULL,
            mes                  INTEGER NOT NULL,
            categoria_id         TEXT NOT NULL REFERENCES categorias(id),
            nota                 TEXT,
            estado               TEXT NOT NULL DEFAULT 'vigente'
                                  CHECK (estado IN ('vigente','corregido')),
            reemplaza_a          TEXT REFERENCES movimientos(id),
            corregido_por_evento TEXT REFERENCES eventos(id),
            evento_origen        TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en       TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta
            ON movimientos (cuenta_id);
        CREATE INDEX IF NOT EXISTS idx_movimientos_fecha
            ON movimientos (fecha);
        CREATE INDEX IF NOT EXISTS idx_movimientos_categoria
            ON movimientos (categoria_id);
        CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta_estado_fecha
            ON movimientos (cuenta_id, estado, fecha);
        CREATE INDEX IF NOT EXISTS idx_movimientos_cuenta_mes_dia
            ON movimientos (cuenta_id, mes, dia);
        -- Un movimiento solo puede ser reemplazado por una corrección directa.
        CREATE UNIQUE INDEX IF NOT EXISTS idx_movimientos_reemplaza
            ON movimientos (reemplaza_a) WHERE reemplaza_a IS NOT NULL;

        -- 6) Inmutabilidad del libro (INV-13): un evento nunca se modifica ni se borra.
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

        -- 7) Los datos propios de un movimiento quedan congelados: solo cambia su
        --    estado de corrección y su trazabilidad. (Los triggers que impiden
        --    borrar movimientos y categorías de fábrica se crean en
        --    `proteger_proyecciones`, porque `reconstruir` debe poder limpiarlas.)
        CREATE TRIGGER IF NOT EXISTS movimientos_campos_inmutables
        BEFORE UPDATE ON movimientos
        WHEN OLD.id            IS NOT NEW.id
          OR OLD.cuenta_id     IS NOT NEW.cuenta_id
          OR OLD.tipo          IS NOT NEW.tipo
          OR OLD.monto         IS NOT NEW.monto
          OR OLD.fecha         IS NOT NEW.fecha
          OR OLD.dia           IS NOT NEW.dia
          OR OLD.mes           IS NOT NEW.mes
          OR OLD.categoria_id  IS NOT NEW.categoria_id
          OR OLD.nota          IS NOT NEW.nota
          OR OLD.reemplaza_a   IS NOT NEW.reemplaza_a
          OR OLD.evento_origen IS NOT NEW.evento_origen
        BEGIN
            SELECT RAISE(ABORT, 'movimientos: los datos son inmutables; registra una corrección');
        END;
        CREATE TRIGGER IF NOT EXISTS movimientos_corregido_no_vuelve
        BEFORE UPDATE OF estado ON movimientos
        WHEN OLD.estado = 'corregido' AND NEW.estado = 'vigente'
        BEGIN
            SELECT RAISE(ABORT, 'un movimiento corregido no puede volver a vigente');
        END;

        -- 8) Composición del saldo: una fila 'inicial' con el saldo inicial de la
        --    cuenta y una fila por cada movimiento vigente. La suma exacta se hace
        --    en Rust con Decimal; esta vista no usa aritmética de punto flotante.
        CREATE VIEW IF NOT EXISTS vista_composicion_saldo AS
        SELECT cuenta_id, cuenta, moneda, saldo_inicial,
               movimiento_id, tipo, monto, fecha, categoria, nota
        FROM (
            -- Fila base: el saldo inicial de cada cuenta.
            SELECT
                c.id            AS cuenta_id,
                c.nombre        AS cuenta,
                c.moneda        AS moneda,
                c.saldo_inicial AS saldo_inicial,
                NULL            AS movimiento_id,
                'inicial'       AS tipo,
                c.saldo_inicial AS monto,
                NULL            AS fecha,
                NULL            AS categoria,
                NULL            AS nota,
                0               AS orden
            FROM cuentas c
            UNION ALL
            -- Una fila por cada movimiento vigente.
            SELECT
                c.id            AS cuenta_id,
                c.nombre        AS cuenta,
                c.moneda        AS moneda,
                c.saldo_inicial AS saldo_inicial,
                m.id            AS movimiento_id,
                m.tipo          AS tipo,
                m.monto         AS monto,
                m.fecha         AS fecha,
                cat.nombre      AS categoria,
                m.nota          AS nota,
                1               AS orden
            FROM movimientos m
            JOIN cuentas c            ON c.id = m.cuenta_id
            LEFT JOIN categorias cat  ON cat.id = m.categoria_id
            WHERE m.estado = 'vigente'
        )
        ORDER BY cuenta_id, orden, fecha, movimiento_id;

        -- 9) Proyección: monedas propias (RF-CAM-01). Las cuatro de fábrica viven
        --    en el dominio y no se guardan aquí. `decimales` fija la escala con
        --    la que se guardan los montos de las cuentas en esa moneda, por eso
        --    una moneda nunca se modifica.
        CREATE TABLE IF NOT EXISTS monedas_propias (
            codigo         TEXT PRIMARY KEY
                                CHECK (length(codigo) BETWEEN 3 AND 5
                                       AND codigo NOT GLOB '*[^A-Z]*'),
            nombre         TEXT NOT NULL CHECK (trim(nombre) <> ''),
            simbolo        TEXT NOT NULL CHECK (trim(simbolo) <> ''),
            decimales      INTEGER NOT NULL CHECK (decimales BETWEEN 0 AND 8),
            evento_origen  TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en TEXT NOT NULL
        );
        CREATE TRIGGER IF NOT EXISTS monedas_propias_inmutables
        BEFORE UPDATE ON monedas_propias
        BEGIN
            SELECT RAISE(ABORT, 'monedas_propias: una moneda no se modifica');
        END;

        -- 10) Proyección: historial de tasas de cambio (RF-CAM-03, RF-CAM-06).
        --     1 moneda_origen = valor / 10^8 moneda_destino: el valor es un entero
        --     con escala fija de 8 decimales. Eliminar una tasa es un evento
        --     `TasaEliminada`; aquí solo cambia su estado.
        CREATE TABLE IF NOT EXISTS tasas (
            id                   TEXT PRIMARY KEY,
            moneda_origen        TEXT NOT NULL,
            moneda_destino       TEXT NOT NULL,
            valor                INTEGER NOT NULL CHECK (typeof(valor) = 'integer' AND valor > 0),
            fecha                TEXT NOT NULL
                                  CHECK (fecha GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            fuente               TEXT NOT NULL CHECK (trim(fuente) <> ''),
            estado               TEXT NOT NULL DEFAULT 'vigente'
                                  CHECK (estado IN ('vigente','eliminada')),
            eliminada_por_evento TEXT REFERENCES eventos(id),
            evento_origen        TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en       TEXT NOT NULL,
            CHECK (moneda_origen <> moneda_destino)
        );
        -- Búsqueda de la tasa vigente de un par en una fecha de consulta.
        CREATE INDEX IF NOT EXISTS idx_tasas_par_fecha
            ON tasas (moneda_origen, moneda_destino, estado, fecha);
        CREATE TRIGGER IF NOT EXISTS tasas_campos_inmutables
        BEFORE UPDATE ON tasas
        WHEN OLD.id             IS NOT NEW.id
          OR OLD.moneda_origen  IS NOT NEW.moneda_origen
          OR OLD.moneda_destino IS NOT NEW.moneda_destino
          OR OLD.valor          IS NOT NEW.valor
          OR OLD.fecha          IS NOT NEW.fecha
          OR OLD.fuente         IS NOT NEW.fuente
          OR OLD.evento_origen  IS NOT NEW.evento_origen
        BEGIN
            SELECT RAISE(ABORT, 'tasas: los datos son inmutables; elimina y registra otra');
        END;
        CREATE TRIGGER IF NOT EXISTS tasas_eliminada_no_vuelve
        BEFORE UPDATE OF estado ON tasas
        WHEN OLD.estado = 'eliminada'
        BEGIN
            SELECT RAISE(ABORT, 'una tasa eliminada no vuelve a cambiar');
        END;

        -- 11) Proyección: moneda base (RF-CAM-02). Una sola fila; sin fila, la
        --     aplicación usa la moneda por defecto.
        CREATE TABLE IF NOT EXISTS moneda_base (
            id             INTEGER PRIMARY KEY CHECK (id = 1),
            moneda         TEXT NOT NULL,
            evento_origen  TEXT NOT NULL REFERENCES eventos(id),
            actualizado_en TEXT NOT NULL
        );
        ",
    )?;
    proteger_proyecciones(conn)?;
    Ok(())
}

/// Crea los triggers que protegen las proyecciones en operación normal: los
/// movimientos no se borran (se corrigen), las categorías de fábrica tampoco
/// (se desactivan), las tasas se marcan como eliminadas y las monedas propias
/// no desaparecen.
///
/// Están separados del bloque principal de [`inicializar`] para que `reconstruir`
/// pueda retirarlos temporalmente, limpiar las proyecciones y volver a crearlos
/// dentro de la misma transacción.
pub fn proteger_proyecciones(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TRIGGER IF NOT EXISTS movimientos_no_delete
        BEFORE DELETE ON movimientos
        BEGIN
            SELECT RAISE(ABORT, 'movimientos no se borran: registra una corrección');
        END;
        CREATE TRIGGER IF NOT EXISTS categorias_fabrica_no_delete
        BEFORE DELETE ON categorias
        WHEN OLD.es_fabrica = 1
        BEGIN
            SELECT RAISE(ABORT, 'categorias de fábrica: desactívalas, no las borres');
        END;
        CREATE TRIGGER IF NOT EXISTS tasas_no_delete
        BEFORE DELETE ON tasas
        BEGIN
            SELECT RAISE(ABORT, 'tasas no se borran: registra un evento TasaEliminada');
        END;
        CREATE TRIGGER IF NOT EXISTS monedas_propias_no_delete
        BEFORE DELETE ON monedas_propias
        BEGIN
            SELECT RAISE(ABORT, 'monedas_propias no se borran');
        END;
        ",
    )?;
    Ok(())
}

/// Retira los triggers de borrado de las proyecciones para permitir una
/// reconstrucción completa desde el libro de eventos.
pub fn desproteger_proyecciones(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        DROP TRIGGER IF EXISTS movimientos_no_delete;
        DROP TRIGGER IF EXISTS categorias_fabrica_no_delete;
        DROP TRIGGER IF EXISTS tasas_no_delete;
        DROP TRIGGER IF EXISTS monedas_propias_no_delete;
        ",
    )?;
    Ok(())
}

/// Categorías de fábrica: siete de gasto (egreso) y cuatro de ingreso.
///
/// Se toman del catálogo del dominio para que exista una sola identidad
/// canónica (`gasto:alimentacion`, `ingreso:salario`, ...). La siembra histórica
/// usaba IDs `cat-fabrica-<tipo>-<slug>`; `aplicacion::categorias` se encarga de
/// traducirlos al proyectar.
fn tipo_categoria_texto(tipo: TipoMovimiento) -> &'static str {
    match tipo {
        TipoMovimiento::Gasto => "egreso",
        TipoMovimiento::Ingreso => "ingreso",
    }
}

/// Claves lógicas `(tipo, nombre en minúsculas)` de las categorías de fábrica
/// que ya existen en el libro de eventos.
///
/// La comprobación es por nombre y tipo, no por ID, para no volver a sembrar el
/// catálogo en bases que todavía tienen los IDs históricos `cat-fabrica-*`.
fn categorias_fabrica_existentes(conn: &Connection) -> Result<HashSet<(String, String)>> {
    let mut existentes = HashSet::new();
    for evento in listar_eventos(conn)?
        .into_iter()
        .filter(|e| e.tipo == TIPO_CATEGORIA_CREADA)
    {
        let Ok(datos) = codec::deserializar_categoria(&evento.datos) else {
            continue;
        };
        if datos.es_fabrica {
            existentes.insert((datos.tipo, datos.nombre.trim().to_lowercase()));
        }
    }
    Ok(existentes)
}

/// Siembra las categorías de fábrica como eventos `CategoriaCreada` y sus filas
/// de proyección, usando los IDs canónicos del dominio.
///
/// Es idempotente: si una categoría de fábrica ya existe en el libro (aunque sea
/// con un ID histórico), no se inserta de nuevo.
///
/// Se registran como eventos (y no solo en la proyección) para que `reconstruir`
/// reproduzca exactamente el mismo catálogo de categorías.
pub fn sembrar_categorias_fabrica(conn: &Connection) -> Result<()> {
    const MOMENTO: &str = "2026-01-01T00:00:00Z";
    const DISPOSITIVO: &str = "fabrica";

    let tx = conn.unchecked_transaction()?;
    let existentes = categorias_fabrica_existentes(&tx)?;

    for categoria in categorias_de_fabrica() {
        let tipo = tipo_categoria_texto(categoria.tipo);
        let clave = (tipo.to_string(), categoria.nombre.trim().to_lowercase());
        if existentes.contains(&clave) {
            continue;
        }

        let id = categoria.id;
        let datos = format!(
            r#"{{"id":"{id}","id_categoria":"{id}","nombre":"{}","tipo":"{tipo}","es_fabrica":true}}"#,
            categoria.nombre
        );

        tx.execute(
            "INSERT OR IGNORE INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, TIPO_CATEGORIA_CREADA, MOMENTO, DISPOSITIVO, datos],
        )?;
        tx.execute(
            "INSERT INTO categorias (id, nombre, tipo, es_fabrica, evento_origen, actualizado_en)
             VALUES (?1, ?2, ?3, 1, ?1, ?4)
             ON CONFLICT(id) DO NOTHING",
            params![id, categoria.nombre, tipo, MOMENTO],
        )?;
    }
    tx.commit()?;
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
    /// Abre (o crea) la base en `ruta`, garantiza el esquema, siembra las
    /// categorías de fábrica que falten y reconstruye las proyecciones desde el
    /// libro de eventos.
    ///
    /// La reconstrucción es la migración: traduce los IDs históricos de
    /// categoría a los IDs canónicos del dominio sin modificar los eventos.
    pub fn persistente<P: AsRef<Path>>(ruta: P) -> Result<Self, aplicacion::ErrorRepositorio> {
        let conn = abrir(ruta).map_err(|e| aplicacion::ErrorRepositorio(e.to_string()))?;
        inicializar(&conn).map_err(|e| aplicacion::ErrorRepositorio(e.to_string()))?;
        sembrar_categorias_fabrica(&conn)
            .map_err(|e| aplicacion::ErrorRepositorio(e.to_string()))?;
        crate::repositorios::reconstruir_proyecciones(&conn)?;
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
    pub saldo_inicial: i64,
    pub saldo_actual: i64,
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

/// Fila cruda de la proyección `categorias` tal como se guarda en SQLite.
#[derive(Debug, Clone)]
pub struct CategoriaRow {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub es_fabrica: i64,
    pub estado: String,
    pub evento_origen: String,
    pub actualizado_en: String,
}

/// Inserta una fila de proyección de categoría.
pub fn insertar_categoria(conn: &Connection, c: &CategoriaRow) -> Result<()> {
    conn.execute(
        "INSERT INTO categorias (id, nombre, tipo, es_fabrica, estado, evento_origen, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO NOTHING",
        params![
            c.id,
            c.nombre,
            c.tipo,
            c.es_fabrica,
            c.estado,
            c.evento_origen,
            c.actualizado_en,
        ],
    )?;
    Ok(())
}

/// Lista todas las categorías proyectadas en orden de inserción (por `rowid`).
pub fn listar_categorias(conn: &Connection) -> Result<Vec<CategoriaRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo, es_fabrica, estado, evento_origen, actualizado_en
         FROM categorias ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CategoriaRow {
            id: r.get(0)?,
            nombre: r.get(1)?,
            tipo: r.get(2)?,
            es_fabrica: r.get(3)?,
            estado: r.get(4)?,
            evento_origen: r.get(5)?,
            actualizado_en: r.get(6)?,
        })
    })?;
    let mut categorias = Vec::new();
    for row in rows {
        categorias.push(row?);
    }
    Ok(categorias)
}

/// Elimina todas las filas de la proyección `categorias`.
pub fn borrar_categorias(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM categorias", [])?;
    Ok(())
}

/// Fila cruda de la proyección `movimientos` tal como se guarda en SQLite.
#[derive(Debug, Clone)]
pub struct MovimientoRow {
    pub id: String,
    pub cuenta_id: String,
    pub tipo: String,
    pub monto: i64,
    pub fecha: String,
    pub dia: i64,
    pub mes: i64,
    pub categoria_id: String,
    pub nota: Option<String>,
    pub estado: String,
    pub reemplaza_a: Option<String>,
    pub corregido_por_evento: Option<String>,
    pub evento_origen: String,
    pub actualizado_en: String,
}

/// Inserta una fila de proyección de movimiento.
pub fn insertar_movimiento(conn: &Connection, m: &MovimientoRow) -> Result<()> {
    conn.execute(
        "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, nota, estado, reemplaza_a, corregido_por_evento, evento_origen, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            m.id,
            m.cuenta_id,
            m.tipo,
            m.monto,
            m.fecha,
            m.dia,
            m.mes,
            m.categoria_id,
            m.nota,
            m.estado,
            m.reemplaza_a,
            m.corregido_por_evento,
            m.evento_origen,
            m.actualizado_en,
        ],
    )?;
    Ok(())
}

/// Lista todos los movimientos proyectados en orden de registro (por `rowid`).
pub fn listar_movimientos(conn: &Connection) -> Result<Vec<MovimientoRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, nota, estado, reemplaza_a, corregido_por_evento, evento_origen, actualizado_en
         FROM movimientos ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(MovimientoRow {
            id: r.get(0)?,
            cuenta_id: r.get(1)?,
            tipo: r.get(2)?,
            monto: r.get(3)?,
            fecha: r.get(4)?,
            dia: r.get(5)?,
            mes: r.get(6)?,
            categoria_id: r.get(7)?,
            nota: r.get(8)?,
            estado: r.get(9)?,
            reemplaza_a: r.get(10)?,
            corregido_por_evento: r.get(11)?,
            evento_origen: r.get(12)?,
            actualizado_en: r.get(13)?,
        })
    })?;
    let mut movimientos = Vec::new();
    for row in rows {
        movimientos.push(row?);
    }
    Ok(movimientos)
}

/// Elimina todas las filas de la proyección `movimientos`.
pub fn borrar_movimientos(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM movimientos", [])?;
    Ok(())
}

/// Marca un movimiento original como corregido por el evento indicado.
pub fn marcar_movimiento_corregido(
    conn: &Connection,
    original_id: &str,
    evento_id: &str,
    momento: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE movimientos
            SET estado = 'corregido', corregido_por_evento = ?2, actualizado_en = ?3
          WHERE id = ?1",
        params![original_id, evento_id, momento],
    )?;
    Ok(())
}

/// Devuelve `(saldo_inicial, moneda)` de una cuenta, o `None` si no existe.
pub fn saldo_inicial_y_moneda(conn: &Connection, cuenta_id: &str) -> Result<Option<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT saldo_inicial, moneda FROM cuentas WHERE id = ?1")?;
    let mut rows = stmt.query(params![cuenta_id])?;
    match rows.next()? {
        Some(fila) => Ok(Some((fila.get(0)?, fila.get(1)?))),
        None => Ok(None),
    }
}

/// Pares `(tipo, monto)` de los movimientos vigentes de una cuenta, en orden de registro.
pub fn movimientos_vigentes(conn: &Connection, cuenta_id: &str) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT tipo, monto FROM movimientos
          WHERE cuenta_id = ?1 AND estado = 'vigente'
          ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map(params![cuenta_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let mut pares = Vec::new();
    for row in rows {
        pares.push(row?);
    }
    Ok(pares)
}

/// Actualiza el saldo actual de una cuenta (nunca toca su saldo inicial).
pub fn actualizar_saldo_cuenta(
    conn: &Connection,
    cuenta_id: &str,
    saldo_actual_cents: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE cuentas SET saldo_actual = ?2 WHERE id = ?1",
        params![cuenta_id, saldo_actual_cents],
    )?;
    Ok(())
}

/// Fila cruda de la proyección `monedas_propias`.
#[derive(Debug, Clone)]
pub struct MonedaPropiaRow {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: i64,
    pub evento_origen: String,
    pub actualizado_en: String,
}

/// Inserta una moneda propia. Un código repetido viola la clave primaria.
pub fn insertar_moneda_propia(conn: &Connection, m: &MonedaPropiaRow) -> Result<()> {
    conn.execute(
        "INSERT INTO monedas_propias (codigo, nombre, simbolo, decimales, evento_origen, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            m.codigo,
            m.nombre,
            m.simbolo,
            m.decimales,
            m.evento_origen,
            m.actualizado_en
        ],
    )?;
    Ok(())
}

/// Lista las monedas propias en orden de creación (por `rowid`).
pub fn listar_monedas_propias(conn: &Connection) -> Result<Vec<MonedaPropiaRow>> {
    let mut stmt = conn.prepare(
        "SELECT codigo, nombre, simbolo, decimales, evento_origen, actualizado_en
         FROM monedas_propias ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(MonedaPropiaRow {
            codigo: r.get(0)?,
            nombre: r.get(1)?,
            simbolo: r.get(2)?,
            decimales: r.get(3)?,
            evento_origen: r.get(4)?,
            actualizado_en: r.get(5)?,
        })
    })?;
    rows.collect()
}

/// Elimina todas las filas de la proyección `monedas_propias` (solo al reconstruir).
pub fn borrar_monedas_propias(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM monedas_propias", [])?;
    Ok(())
}

/// Fila cruda de la proyección `tasas`. `valor` es el valor × 10^8.
#[derive(Debug, Clone)]
pub struct TasaRow {
    pub id: String,
    pub moneda_origen: String,
    pub moneda_destino: String,
    pub valor: i64,
    pub fecha: String,
    pub fuente: String,
    pub estado: String,
    pub eliminada_por_evento: Option<String>,
    pub evento_origen: String,
    pub actualizado_en: String,
}

/// Inserta una tasa en el historial. Un `id` repetido viola la clave primaria.
pub fn insertar_tasa(conn: &Connection, t: &TasaRow) -> Result<()> {
    conn.execute(
        "INSERT INTO tasas (id, moneda_origen, moneda_destino, valor, fecha, fuente, estado, eliminada_por_evento, evento_origen, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            t.id,
            t.moneda_origen,
            t.moneda_destino,
            t.valor,
            t.fecha,
            t.fuente,
            t.estado,
            t.eliminada_por_evento,
            t.evento_origen,
            t.actualizado_en,
        ],
    )?;
    Ok(())
}

/// Lista el historial de tasas en orden de registro (por `rowid`).
pub fn listar_tasas(conn: &Connection) -> Result<Vec<TasaRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, moneda_origen, moneda_destino, valor, fecha, fuente, estado, eliminada_por_evento, evento_origen, actualizado_en
         FROM tasas ORDER BY rowid ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TasaRow {
            id: r.get(0)?,
            moneda_origen: r.get(1)?,
            moneda_destino: r.get(2)?,
            valor: r.get(3)?,
            fecha: r.get(4)?,
            fuente: r.get(5)?,
            estado: r.get(6)?,
            eliminada_por_evento: r.get(7)?,
            evento_origen: r.get(8)?,
            actualizado_en: r.get(9)?,
        })
    })?;
    rows.collect()
}

/// Marca una tasa vigente como eliminada por el evento indicado.
///
/// Devuelve cuántas filas cambió: 0 si la tasa no existe o ya estaba eliminada.
pub fn marcar_tasa_eliminada(
    conn: &Connection,
    tasa_id: &str,
    evento_id: &str,
    momento: &str,
) -> Result<usize> {
    conn.execute(
        "UPDATE tasas
            SET estado = 'eliminada', eliminada_por_evento = ?2, actualizado_en = ?3
          WHERE id = ?1 AND estado = 'vigente'",
        params![tasa_id, evento_id, momento],
    )
}

/// Elimina todas las filas de la proyección `tasas` (solo al reconstruir).
pub fn borrar_tasas(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM tasas", [])?;
    Ok(())
}

/// Fija la moneda base (fila única de la proyección `moneda_base`).
pub fn fijar_moneda_base(
    conn: &Connection,
    moneda: &str,
    evento_id: &str,
    momento: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO moneda_base (id, moneda, evento_origen, actualizado_en)
         VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE
            SET moneda = excluded.moneda,
                evento_origen = excluded.evento_origen,
                actualizado_en = excluded.actualizado_en",
        params![moneda, evento_id, momento],
    )?;
    Ok(())
}

/// Moneda base elegida, o `None` si nunca se cambió.
pub fn leer_moneda_base(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT moneda FROM moneda_base WHERE id = 1")?;
    let mut rows = stmt.query([])?;
    match rows.next()? {
        Some(fila) => Ok(Some(fila.get(0)?)),
        None => Ok(None),
    }
}

/// Elimina la moneda base proyectada (solo al reconstruir).
pub fn borrar_moneda_base(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM moneda_base", [])?;
    Ok(())
}

/// Moneda de una cuenta proyectada, o `None` si la cuenta no existe.
pub fn moneda_de_cuenta(conn: &Connection, cuenta_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT moneda FROM cuentas WHERE id = ?1")?;
    let mut rows = stmt.query(params![cuenta_id])?;
    match rows.next()? {
        Some(fila) => Ok(Some(fila.get(0)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn conexion_vacia() -> Connection {
        let conn = Connection::open_in_memory().expect("abrir base en memoria");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("activar claves foráneas");
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

    fn vista_existe(conn: &Connection, vista: &str) -> bool {
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'view' AND name = ?1",
                params![vista],
                |r| r.get(0),
            )
            .expect("consultar sqlite_master");
        total == 1
    }

    /// Conexión con el esquema inicializado, las categorías de fábrica sembradas
    /// y una cuenta `c1` (USD, saldo 100.00 = 10000 centavos) lista para recibir
    /// movimientos.
    fn base_con_cuenta() -> Connection {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("sembrar categorías de fábrica");
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('ev-cuenta', 'CuentaCreada', '2026-01-01T00:00:00Z', 'd1', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cuentas (id, nombre, tipo, moneda, saldo_inicial, saldo_actual, evento_origen, actualizado_en)
             VALUES ('c1', 'Cartera', 'efectivo', 'USD', 10000, 10000, 'ev-cuenta', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn
    }

    /// Inserta un evento `MovimientoRegistrado` y su fila en `movimientos`.
    fn insertar_movimiento(
        conn: &Connection,
        id: &str,
        tipo: &str,
        monto: &str,
        categoria: &str,
        reemplaza_a: Option<&str>,
    ) {
        let evento_id = format!("ev-{id}");
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES (?1, 'MovimientoRegistrado', '2026-10-19T00:00:00Z', 'd1', '{}')",
            params![evento_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, reemplaza_a, evento_origen, actualizado_en)
             VALUES (?1, 'c1', ?2, ?3, '2026-10-19', 19, 10, ?4, ?5, ?6, '2026-10-19T00:00:00Z')",
            params![
                id,
                tipo,
                unidades::a_unidades(monto, 2).expect("monto USD válido"),
                categoria,
                reemplaza_a,
                evento_id
            ],
        )
        .unwrap();
    }

    fn contar_por(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).expect("contar")
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
    fn crea_las_tablas_de_categorias_y_movimientos() {
        let conn = conexion_vacia();
        assert!(
            tabla_existe(&conn, "categorias"),
            "falta la tabla categorias"
        );
        assert!(
            tabla_existe(&conn, "movimientos"),
            "falta la tabla movimientos"
        );
        assert!(
            vista_existe(&conn, "vista_composicion_saldo"),
            "falta la vista vista_composicion_saldo"
        );
    }

    #[test]
    fn siembra_con_ids_canonicos_del_dominio() {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("sembrar");

        let mut stmt = conn.prepare("SELECT id FROM categorias").unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|id| id.unwrap())
            .collect();

        for categoria in dominio::categorias_de_fabrica() {
            assert!(
                ids.contains(&categoria.id),
                "falta la categoría canónica {}",
                categoria.id
            );
        }
        assert_eq!(ids.len(), 11, "no debe haber IDs duplicados");
        assert!(
            !ids.iter().any(|id| id.starts_with("cat-fabrica-")),
            "la siembra nueva no debe usar IDs históricos"
        );
    }

    #[test]
    fn siembra_no_duplica_categorias_historicas() {
        let conn = conexion_vacia();
        // Simula una base anterior: la categoría existe con su ID histórico.
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('cat-fabrica-egreso-alimentacion', 'CategoriaCreada', '2026-01-01T00:00:00Z', 'fabrica',
                     '{\"id\":\"cat-fabrica-egreso-alimentacion\",\"nombre\":\"Alimentación\",\"tipo\":\"egreso\",\"es_fabrica\":true}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO categorias (id, nombre, tipo, es_fabrica, evento_origen, actualizado_en)
             VALUES ('cat-fabrica-egreso-alimentacion', 'Alimentación', 'egreso', 1,
                     'cat-fabrica-egreso-alimentacion', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        sembrar_categorias_fabrica(&conn).expect("segunda siembra");

        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM eventos WHERE tipo = 'CategoriaCreada'"
            ),
            11,
            "no debe agregar un evento canónico si ya existe la categoría lógica"
        );
        assert_eq!(
            contar_por(&conn, "SELECT COUNT(*) FROM categorias"),
            11,
            "no debe duplicar la categoría histórica"
        );
    }

    #[test]
    fn siembra_once_categorias_de_fabrica() {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("sembrar");

        assert_eq!(contar_por(&conn, "SELECT COUNT(*) FROM categorias"), 11);
        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM categorias WHERE es_fabrica = 1"
            ),
            11
        );
        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM categorias WHERE tipo = 'egreso'"
            ),
            7
        );
        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM categorias WHERE tipo = 'ingreso'"
            ),
            4
        );
        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM eventos WHERE tipo = 'CategoriaCreada'"
            ),
            11
        );

        let mut stmt = conn
            .prepare("SELECT nombre FROM categorias WHERE tipo = 'egreso'")
            .unwrap();
        let nombres: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|n| n.unwrap())
            .collect();
        for esperado in [
            "Alimentación",
            "Transporte",
            "Servicios",
            "Salud",
            "Educación",
            "Entretenimiento",
            "Otros gastos",
        ] {
            assert!(nombres.iter().any(|n| n == esperado), "falta {esperado}");
        }
    }

    #[test]
    fn sembrar_categorias_es_idempotente() {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("primera siembra");
        sembrar_categorias_fabrica(&conn).expect("segunda siembra");
        assert_eq!(contar_por(&conn, "SELECT COUNT(*) FROM categorias"), 11);
        assert_eq!(
            contar_por(
                &conn,
                "SELECT COUNT(*) FROM eventos WHERE tipo = 'CategoriaCreada'"
            ),
            11
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
    fn categoria_repetida_en_el_mismo_tipo_falla() {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("sembrar");
        let err = conn.execute(
            "INSERT INTO categorias (id, nombre, tipo, es_fabrica, evento_origen, actualizado_en)
             VALUES ('otra', 'alimentación', 'egreso', 0, 'gasto:alimentacion', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            err.is_err(),
            "no debe repetirse el nombre dentro del mismo tipo"
        );
    }

    #[test]
    fn mismo_nombre_en_distinto_tipo_es_valido() {
        let conn = conexion_vacia();
        sembrar_categorias_fabrica(&conn).expect("sembrar");
        let resultado = conn.execute(
            "INSERT INTO categorias (id, nombre, tipo, es_fabrica, evento_origen, actualizado_en)
             VALUES ('otra', 'Alimentación', 'ingreso', 0, 'gasto:alimentacion', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(resultado.is_ok(), "el nombre puede repetirse en otro tipo");
    }

    #[test]
    fn movimiento_valido_se_inserta() {
        let conn = base_con_cuenta();
        insertar_movimiento(&conn, "m1", "ingreso", "10.50", "ingreso:salario", None);
        let (tipo, monto, estado): (String, i64, String) = conn
            .query_row(
                "SELECT tipo, monto, estado FROM movimientos WHERE id = 'm1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(tipo, "ingreso");
        assert_eq!(monto, 1050, "10.50 USD se guarda como 1050 centavos");
        assert_eq!(estado, "vigente");
    }

    #[test]
    fn movimiento_con_cuenta_inexistente_falla() {
        let conn = base_con_cuenta();
        let err = conn.execute(
            "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, evento_origen, actualizado_en)
             VALUES ('m', 'no-existe', 'ingreso', '1.00', '2026-10-19', 19, 10, 'ingreso:salario', 'ev-cuenta', '2026-10-19T00:00:00Z')",
            [],
        );
        assert!(err.is_err(), "la cuenta debe existir (FK)");
    }

    #[test]
    fn movimiento_con_monto_no_positivo_falla() {
        let conn = base_con_cuenta();
        // Enteros no positivos, y cualquier valor que no sea un entero: un
        // decimal o un texto nunca entran a la columna de unidades mínimas.
        for monto in ["0", "-500", "10.5", "abc"] {
            let err = conn.execute(
                "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, evento_origen, actualizado_en)
                 VALUES (?1, 'c1', 'ingreso', ?2, '2026-10-19', 19, 10, 'ingreso:salario', 'ev-cuenta', '2026-10-19T00:00:00Z')",
                params![format!("m-{monto}"), monto],
            );
            assert!(err.is_err(), "el monto {monto} debía rechazarse");
        }
    }

    #[test]
    fn movimiento_con_tipo_invalido_falla() {
        let conn = base_con_cuenta();
        let err = conn.execute(
            "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, evento_origen, actualizado_en)
             VALUES ('m', 'c1', 'transferencia', '1.00', '2026-10-19', 19, 10, 'ingreso:salario', 'ev-cuenta', '2026-10-19T00:00:00Z')",
            [],
        );
        assert!(err.is_err(), "el tipo debe ser ingreso o egreso");
    }

    #[test]
    fn movimiento_con_fecha_invalida_falla() {
        let conn = base_con_cuenta();
        let err = conn.execute(
            "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, evento_origen, actualizado_en)
             VALUES ('m', 'c1', 'ingreso', '1.00', '19/10/2026', 19, 10, 'ingreso:salario', 'ev-cuenta', '2026-10-19T00:00:00Z')",
            [],
        );
        assert!(err.is_err(), "la fecha debe tener formato AAAA-MM-DD");
    }

    #[test]
    fn correccion_marca_original_y_referencia() {
        let conn = base_con_cuenta();
        insertar_movimiento(&conn, "m1", "egreso", "10.00", "gasto:alimentacion", None);
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('ev-corr', 'MovimientoCorregido', '2026-10-19T01:00:00Z', 'd1', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE movimientos
                SET estado = 'corregido', corregido_por_evento = 'ev-corr',
                    actualizado_en = '2026-10-19T01:00:00Z'
              WHERE id = 'm1'",
            [],
        )
        .unwrap();
        insertar_movimiento(
            &conn,
            "m2",
            "egreso",
            "12.00",
            "gasto:alimentacion",
            Some("m1"),
        );

        let (estado, corregido_por): (String, Option<String>) = conn
            .query_row(
                "SELECT estado, corregido_por_evento FROM movimientos WHERE id = 'm1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(estado, "corregido");
        assert_eq!(corregido_por.as_deref(), Some("ev-corr"));

        let reemplaza: Option<String> = conn
            .query_row(
                "SELECT reemplaza_a FROM movimientos WHERE id = 'm2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reemplaza.as_deref(), Some("m1"));
    }

    #[test]
    fn un_movimiento_solo_se_corrige_una_vez() {
        let conn = base_con_cuenta();
        insertar_movimiento(&conn, "m1", "egreso", "10.00", "gasto:alimentacion", None);
        conn.execute(
            "UPDATE movimientos SET estado = 'corregido' WHERE id = 'm1'",
            [],
        )
        .unwrap();
        insertar_movimiento(
            &conn,
            "m2",
            "egreso",
            "12.00",
            "gasto:alimentacion",
            Some("m1"),
        );
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('ev-m3', 'MovimientoRegistrado', '2026-10-19T02:00:00Z', 'd1', '{}')",
            [],
        )
        .unwrap();
        let err = conn.execute(
            "INSERT INTO movimientos (id, cuenta_id, tipo, monto, fecha, dia, mes, categoria_id, reemplaza_a, evento_origen, actualizado_en)
             VALUES ('m3', 'c1', 'egreso', '1.00', '2026-10-19', 19, 10, 'gasto:alimentacion', 'm1', 'ev-m3', '2026-10-19T02:00:00Z')",
            [],
        );
        assert!(
            err.is_err(),
            "un original solo admite una corrección directa"
        );
    }

    #[test]
    fn movimientos_no_se_borran_ni_se_modifican() {
        let conn = base_con_cuenta();
        insertar_movimiento(&conn, "m1", "egreso", "10.00", "gasto:alimentacion", None);

        assert!(
            conn.execute("UPDATE movimientos SET monto = '99.00' WHERE id = 'm1'", [])
                .is_err(),
            "el monto es inmutable"
        );
        assert!(
            conn.execute("UPDATE movimientos SET nota = 'otra' WHERE id = 'm1'", [])
                .is_err(),
            "la nota es inmutable"
        );
        assert!(
            conn.execute("DELETE FROM movimientos WHERE id = 'm1'", [])
                .is_err(),
            "los movimientos no se borran"
        );

        // Cambiar el estado sí está permitido, pero no revertirlo.
        conn.execute(
            "UPDATE movimientos SET estado = 'corregido' WHERE id = 'm1'",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "UPDATE movimientos SET estado = 'vigente' WHERE id = 'm1'",
                []
            )
            .is_err(),
            "un movimiento corregido no vuelve a vigente"
        );
    }

    #[test]
    fn eventos_siguen_siendo_inmutables() {
        let conn = base_con_cuenta();
        assert!(
            conn.execute(
                "UPDATE eventos SET tipo = 'Otro' WHERE id = 'ev-cuenta'",
                []
            )
            .is_err(),
            "UPDATE sobre eventos debe rechazarse"
        );
        assert!(
            conn.execute("DELETE FROM eventos WHERE id = 'ev-cuenta'", [])
                .is_err(),
            "DELETE sobre eventos debe rechazarse"
        );
    }

    #[test]
    fn categorias_de_fabrica_no_se_borran() {
        let conn = base_con_cuenta();
        assert!(
            conn.execute("DELETE FROM categorias WHERE id = 'gasto:alimentacion'", [])
                .is_err(),
            "las categorías de fábrica se desactivan, no se borran"
        );
    }

    #[test]
    fn vista_composicion_saldo_muestra_solo_vigentes() {
        let conn = base_con_cuenta();
        insertar_movimiento(&conn, "m1", "ingreso", "10.00", "ingreso:salario", None);
        insertar_movimiento(&conn, "m2", "egreso", "4.00", "gasto:alimentacion", None);
        insertar_movimiento(&conn, "m3", "egreso", "9.00", "gasto:transporte", None);
        conn.execute(
            "UPDATE movimientos SET estado = 'corregido' WHERE id = 'm3'",
            [],
        )
        .unwrap();
        insertar_movimiento(
            &conn,
            "m4",
            "egreso",
            "8.00",
            "gasto:transporte",
            Some("m3"),
        );

        let vigentes = contar_por(
            &conn,
            "SELECT COUNT(*) FROM vista_composicion_saldo WHERE cuenta_id = 'c1' AND tipo <> 'inicial'",
        );
        assert_eq!(vigentes, 3, "deben verse m1, m2 y m4 (m3 quedó corregido)");

        let inicial: i64 = conn
            .query_row(
                "SELECT monto FROM vista_composicion_saldo WHERE cuenta_id = 'c1' AND tipo = 'inicial'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(inicial, 10000);
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
            assert!(tabla_existe(&conn, "categorias"));
            assert!(tabla_existe(&conn, "movimientos"));
            assert_eq!(
                contar_por(
                    &conn,
                    "SELECT COUNT(*) FROM categorias WHERE es_fabrica = 1"
                ),
                11
            );
        }
        assert!(ruta.exists(), "el archivo SQLite debe existir en disco");

        // Segunda sesión: se reabre el mismo archivo sin duplicar la semilla.
        {
            let bd = BaseDatos::persistente(&ruta).expect("reabrir base persistente");
            let conn = bd.conexion();
            let conn = conn.lock().unwrap();
            assert_eq!(contar_por(&conn, "SELECT COUNT(*) FROM categorias"), 11);
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

    #[test]
    fn tasas_y_monedas_propias_no_se_borran_ni_se_modifican() {
        let conn = conexion_vacia();
        conn.execute_batch(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('ev-btc', 'MonedaCreada', '2026-10-20T00:00:00Z', 'd1', '{}'),
                    ('ev-t1', 'TasaRegistrada', '2026-10-20T00:00:00Z', 'd1', '{}'),
                    ('ev-del', 'TasaEliminada', '2026-10-21T00:00:00Z', 'd1', '{}');
             INSERT INTO monedas_propias (codigo, nombre, simbolo, decimales, evento_origen, actualizado_en)
             VALUES ('BTC', 'Bitcoin', 'B', 8, 'ev-btc', '2026-10-20T00:00:00Z');
             INSERT INTO tasas (id, moneda_origen, moneda_destino, valor, fecha, fuente, evento_origen, actualizado_en)
             VALUES ('t1', 'USD', 'VES', 88700000000, '2026-10-20', 'BCV', 'ev-t1', '2026-10-20T00:00:00Z');",
        )
        .unwrap();

        assert!(conn.execute("DELETE FROM tasas", []).is_err());
        assert!(conn.execute("UPDATE tasas SET valor = 1", []).is_err());
        assert!(conn.execute("DELETE FROM monedas_propias", []).is_err());
        assert!(
            conn.execute("UPDATE monedas_propias SET decimales = 2", [])
                .is_err(),
            "cambiar los decimales invalidaría los montos guardados"
        );

        assert_eq!(
            marcar_tasa_eliminada(&conn, "t1", "ev-del", "2026-10-21T00:00:00Z").unwrap(),
            1
        );
        assert_eq!(
            marcar_tasa_eliminada(&conn, "t1", "ev-del", "2026-10-21T00:00:00Z").unwrap(),
            0,
            "una tasa eliminada no se vuelve a eliminar"
        );
        assert!(
            conn.execute("UPDATE tasas SET estado = 'vigente'", [])
                .is_err()
        );
    }

    #[test]
    fn las_restricciones_de_tasas_y_monedas_propias_se_cumplen() {
        let conn = conexion_vacia();
        conn.execute(
            "INSERT INTO eventos (id, tipo, momento_registro, dispositivo_origen, datos)
             VALUES ('ev', 'x', '2026-10-20T00:00:00Z', 'd1', '{}')",
            [],
        )
        .unwrap();
        let tasa = |origen: &str, destino: &str, valor: &str, fuente: &str| {
            conn.execute(
                &format!(
                    "INSERT INTO tasas (id, moneda_origen, moneda_destino, valor, fecha, fuente, evento_origen, actualizado_en)
                     VALUES ('t-{origen}{destino}{valor}', '{origen}', '{destino}', {valor}, '2026-10-20', '{fuente}', 'ev', 'x')"
                ),
                [],
            )
        };
        assert!(tasa("USD", "VES", "1", "BCV").is_ok());
        assert!(tasa("USD", "USD", "1", "BCV").is_err(), "monedas iguales");
        assert!(tasa("USD", "EUR", "0", "BCV").is_err(), "valor cero");
        assert!(tasa("USD", "EUR", "1.5", "BCV").is_err(), "valor no entero");
        assert!(tasa("USD", "EUR", "1", " ").is_err(), "sin fuente");

        let moneda = |codigo: &str, decimales: i64| {
            conn.execute(
                "INSERT INTO monedas_propias (codigo, nombre, simbolo, decimales, evento_origen, actualizado_en)
                 VALUES (?1, 'X', 'X', ?2, 'ev', 'x')",
                params![codigo, decimales],
            )
        };
        assert!(moneda("BTC", 8).is_ok());
        assert!(moneda("BTC", 2).is_err(), "código repetido");
        assert!(moneda("btc", 2).is_err(), "minúsculas");
        assert!(moneda("AB", 2).is_err(), "muy corto");
        assert!(moneda("ABCDEF", 2).is_err(), "muy largo");
        assert!(moneda("ABC", 9).is_err(), "más de 8 decimales");
    }
}
