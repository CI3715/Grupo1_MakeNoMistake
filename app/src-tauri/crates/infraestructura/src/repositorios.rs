use crate::db::{self, insertar_evento};
use aplicacion::proyector::{proyectar_categoria, proyectar_cuenta, proyectar_movimiento};
use aplicacion::puertos::{
    CategoriaProyectada, CuentaProyectada, ErrorRepositorio, EventoRegistrado,
    MovimientoProyectado, RepositorioEventos, ResultadoGuardado,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Implementación persistente de [`RepositorioEventos`] con SQLite.
///
/// Guarda el libro de eventos y sus proyecciones (`cuentas`, `categorias` y
/// `movimientos`) en la misma transacción para conservar la atomicidad.
/// `listar_*` lee directamente las proyecciones y `reconstruir` vuelve a
/// materializarlas desde los eventos.
pub struct RepositorioEventosSqlite {
    conn: Arc<Mutex<Connection>>,
}

impl RepositorioEventosSqlite {
    pub fn nuevo(conn: Connection) -> Self {
        RepositorioEventosSqlite {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn with_conn(conn: Arc<Mutex<Connection>>) -> Self {
        RepositorioEventosSqlite { conn }
    }
}

/// Aplica a las proyecciones el efecto de un único evento.
///
/// Es la contraparte persistente de la proyección: la usan tanto `guardar`
/// (proyección incremental) como `reconstruir` (proyección completa).
fn proyectar_evento(conn: &Connection, evento: &EventoRegistrado) -> Result<(), ErrorRepositorio> {
    if let Some(cuenta) = proyectar_cuenta(evento).map_err(|e| ErrorRepositorio(e.to_string()))? {
        let fila = db::CuentaRow {
            id: cuenta.id,
            nombre: cuenta.nombre,
            tipo: cuenta.tipo,
            moneda: cuenta.moneda,
            saldo_inicial: cuenta.saldo_inicial,
            saldo_actual: cuenta.saldo_actual,
            estado: cuenta.estado,
            evento_origen: evento.id.clone(),
            actualizado_en: evento.momento_registro.clone(),
        };
        db::insertar_cuenta(conn, &fila).map_err(|e| ErrorRepositorio(e.to_string()))?;
    }

    if let Some(categoria) =
        proyectar_categoria(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
    {
        let fila = db::CategoriaRow {
            id: categoria.id,
            nombre: categoria.nombre,
            tipo: categoria.tipo,
            es_fabrica: i64::from(categoria.es_fabrica),
            estado: categoria.estado,
            evento_origen: evento.id.clone(),
            actualizado_en: evento.momento_registro.clone(),
        };
        db::insertar_categoria(conn, &fila).map_err(|e| ErrorRepositorio(e.to_string()))?;
    }

    if let Some(movimiento) =
        proyectar_movimiento(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
    {
        if let Some(original) = movimiento.reemplaza_a.as_deref() {
            db::marcar_movimiento_corregido(conn, original, &evento.id, &evento.momento_registro)
                .map_err(|e| ErrorRepositorio(e.to_string()))?;
        }
        let cuenta_id = movimiento.cuenta_id.clone();
        let fila = db::MovimientoRow {
            id: movimiento.id,
            cuenta_id: movimiento.cuenta_id,
            tipo: movimiento.tipo,
            monto: movimiento.monto,
            fecha: movimiento.fecha,
            categoria_id: movimiento.categoria_id,
            nota: movimiento.nota,
            estado: movimiento.estado,
            reemplaza_a: movimiento.reemplaza_a,
            corregido_por_evento: None,
            evento_origen: evento.id.clone(),
            actualizado_en: evento.momento_registro.clone(),
        };
        db::insertar_movimiento(conn, &fila).map_err(|e| ErrorRepositorio(e.to_string()))?;
        recalcular_saldo(conn, &cuenta_id)?;
    }

    Ok(())
}

/// Reconstruye las tres proyecciones (`cuentas`, `categorias` y `movimientos`)
/// desde el libro de eventos, en una sola transacción.
///
/// Se usa al abrir la base para materializar las proyecciones nuevas de una base
/// de la Entrega 2 sin perder nada, y también desde
/// [`RepositorioEventos::reconstruir`].
pub fn reconstruir_proyecciones(conn: &Connection) -> Result<(), ErrorRepositorio> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| ErrorRepositorio(e.to_string()))?;

    // Los triggers de borrado protegen las proyecciones en operación normal;
    // aquí hay que retirarlos para reconstruirlas por completo. Se crean de
    // nuevo al final, dentro de la misma transacción.
    db::desproteger_proyecciones(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
    db::borrar_movimientos(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
    db::borrar_categorias(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
    db::borrar_cuentas(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;

    let eventos = db::listar_eventos(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
    for row in &eventos {
        let evento = EventoRegistrado {
            id: row.id.clone(),
            tipo: row.tipo.clone(),
            momento_registro: row.momento_registro.clone(),
            dispositivo_origen: row.dispositivo_origen.clone(),
            datos: row.datos.clone(),
        };
        proyectar_evento(&tx, &evento)?;
    }

    db::proteger_proyecciones(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
    tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
    Ok(())
}

/// Recalcula `saldo_actual` de una cuenta a partir de su saldo inicial y sus
/// movimientos vigentes, sin tocar `saldo_inicial` (D-11).
fn recalcular_saldo(conn: &Connection, cuenta_id: &str) -> Result<(), ErrorRepositorio> {
    let Some((saldo_inicial, moneda)) =
        db::saldo_inicial_y_moneda(conn, cuenta_id).map_err(|e| ErrorRepositorio(e.to_string()))?
    else {
        return Ok(());
    };
    let vigentes =
        db::movimientos_vigentes(conn, cuenta_id).map_err(|e| ErrorRepositorio(e.to_string()))?;
    let saldo = aplicacion::proyector::calcular_saldo_actual(&saldo_inicial, &moneda, &vigentes)
        .map_err(|e| ErrorRepositorio(e.to_string()))?;
    db::actualizar_saldo_cuenta(conn, cuenta_id, &saldo)
        .map_err(|e| ErrorRepositorio(e.to_string()))?;
    Ok(())
}

impl RepositorioEventos for RepositorioEventosSqlite {
    fn guardar(
        &mut self,
        evento: &EventoRegistrado,
    ) -> Result<ResultadoGuardado, ErrorRepositorio> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;

        match insertar_evento(
            &tx,
            &evento.id,
            &evento.tipo,
            &evento.momento_registro,
            &evento.dispositivo_origen,
            &evento.datos,
        ) {
            Ok(false) => {
                tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
                return Ok(ResultadoGuardado::YaExistia);
            }
            Ok(true) => {}
            Err(e) => return Err(ErrorRepositorio(e.to_string())),
        }

        proyectar_evento(&tx, evento)?;

        tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(ResultadoGuardado::Guardado)
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_eventos(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| EventoRegistrado {
                    id: r.id,
                    tipo: r.tipo,
                    momento_registro: r.momento_registro,
                    dispositivo_origen: r.dispositivo_origen,
                    datos: r.datos,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn listar_cuentas(&self) -> Result<Vec<CuentaProyectada>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_cuentas(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| CuentaProyectada {
                    id: r.id,
                    nombre: r.nombre,
                    tipo: r.tipo,
                    moneda: r.moneda,
                    saldo_inicial: r.saldo_inicial,
                    saldo_actual: r.saldo_actual,
                    estado: r.estado,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn listar_categorias(&self) -> Result<Vec<CategoriaProyectada>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_categorias(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| CategoriaProyectada {
                    id: r.id,
                    nombre: r.nombre,
                    tipo: r.tipo,
                    es_fabrica: r.es_fabrica != 0,
                    estado: r.estado,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn listar_movimientos(&self) -> Result<Vec<MovimientoProyectado>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_movimientos(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| MovimientoProyectado {
                    id: r.id,
                    cuenta_id: r.cuenta_id,
                    tipo: r.tipo,
                    monto: r.monto,
                    fecha: r.fecha,
                    categoria_id: r.categoria_id,
                    nota: r.nota,
                    estado: r.estado,
                    reemplaza_a: r.reemplaza_a,
                    corregido_por_evento: r.corregido_por_evento,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        reconstruir_proyecciones(&conn)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use aplicacion::contrato::bateria_contrato_repositorio;

    fn repositorio_en_memoria_sqlite() -> RepositorioEventosSqlite {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::inicializar(&conn).unwrap();
        RepositorioEventosSqlite::nuevo(conn)
    }

    fn guardar_evento(repo: &mut RepositorioEventosSqlite, id: &str, tipo: &str, datos: &str) {
        let evento = EventoRegistrado {
            id: id.to_string(),
            tipo: tipo.to_string(),
            momento_registro: "2026-10-14T12:00:00Z".to_string(),
            dispositivo_origen: "d1".to_string(),
            datos: datos.to_string(),
        };
        repo.guardar(&evento).unwrap();
    }

    #[test]
    fn contrato_sqlite_pasa_bateria_completa() {
        bateria_contrato_repositorio(repositorio_en_memoria_sqlite);
    }

    #[test]
    fn reconstruir_rearma_las_tres_proyecciones() {
        let mut repo = repositorio_en_memoria_sqlite();
        guardar_evento(
            &mut repo,
            "e-cuenta",
            "CuentaCreada",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        );
        guardar_evento(
            &mut repo,
            "e-cat",
            "CategoriaCreada",
            r#"{"id":"cat-1","nombre":"Comida","tipo":"egreso","es_fabrica":false}"#,
        );
        guardar_evento(
            &mut repo,
            "e-mov",
            "MovimientoRegistrado",
            r#"{"id":"m1","cuenta_id":"c1","tipo":"egreso","monto":"25.00","fecha":"2026-10-14","categoria_id":"cat-1","nota":"cena"}"#,
        );

        let cuentas_antes = repo.listar_cuentas().unwrap();
        let categorias_antes = repo.listar_categorias().unwrap();
        let movimientos_antes = repo.listar_movimientos().unwrap();
        assert_eq!(cuentas_antes[0].saldo_inicial, "100.00");
        assert_eq!(cuentas_antes[0].saldo_actual, "75.00");
        assert_eq!(movimientos_antes[0].nota.as_deref(), Some("cena"));

        repo.reconstruir().expect("reconstruir");

        assert_eq!(repo.listar_cuentas().unwrap(), cuentas_antes);
        assert_eq!(repo.listar_categorias().unwrap(), categorias_antes);
        assert_eq!(repo.listar_movimientos().unwrap(), movimientos_antes);
    }

    #[test]
    fn reconstruir_conserva_la_correccion_y_el_saldo() {
        let mut repo = repositorio_en_memoria_sqlite();
        guardar_evento(
            &mut repo,
            "e-cuenta",
            "CuentaCreada",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        );
        guardar_evento(
            &mut repo,
            "e-cat",
            "CategoriaCreada",
            r#"{"id":"cat-1","nombre":"Comida","tipo":"egreso","es_fabrica":false}"#,
        );
        guardar_evento(
            &mut repo,
            "e-m1",
            "MovimientoRegistrado",
            r#"{"id":"m1","cuenta_id":"c1","tipo":"egreso","monto":"30.00","fecha":"2026-10-14","categoria_id":"cat-1"}"#,
        );
        guardar_evento(
            &mut repo,
            "e-m2",
            "MovimientoCorregido",
            r#"{"id":"m2","movimiento_original_id":"m1","cuenta_id":"c1","tipo":"egreso","monto":"20.00","fecha":"2026-10-14","categoria_id":"cat-1"}"#,
        );

        let antes = (
            repo.listar_cuentas().unwrap(),
            repo.listar_categorias().unwrap(),
            repo.listar_movimientos().unwrap(),
        );
        assert_eq!(antes.0[0].saldo_actual, "80.00");
        assert_eq!(antes.2[0].estado, "corregido");
        assert_eq!(antes.2[1].reemplaza_a.as_deref(), Some("m1"));

        repo.reconstruir().expect("reconstruir");

        assert_eq!(
            (
                repo.listar_cuentas().unwrap(),
                repo.listar_categorias().unwrap(),
                repo.listar_movimientos().unwrap(),
            ),
            antes
        );
    }

    #[test]
    fn el_saldo_inicial_no_cambia_con_los_movimientos() {
        let mut repo = repositorio_en_memoria_sqlite();
        guardar_evento(
            &mut repo,
            "e-cuenta",
            "CuentaCreada",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        );
        guardar_evento(
            &mut repo,
            "e-cat",
            "CategoriaCreada",
            r#"{"id":"cat-1","nombre":"Salario","tipo":"ingreso","es_fabrica":false}"#,
        );
        guardar_evento(
            &mut repo,
            "e-m1",
            "MovimientoRegistrado",
            r#"{"id":"m1","cuenta_id":"c1","tipo":"ingreso","monto":"40.00","fecha":"2026-10-14","categoria_id":"cat-1"}"#,
        );

        let cuenta = &repo.listar_cuentas().unwrap()[0];
        assert_eq!(cuenta.saldo_inicial, "100.00", "el saldo inicial no cambia");
        assert_eq!(cuenta.saldo_actual, "140.00", "el saldo actual sí cambia");
    }

    #[test]
    fn abrir_una_base_de_entrega_2_reconstruye_sin_perder_cuentas() {
        // Simula una base de la Entrega 2: solo existen `eventos`, `cuentas` y
        // `configuracion`; las proyecciones nuevas todavía no.
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::inicializar(&conn).unwrap();

        crate::db::insertar_evento(
            &conn,
            "e-cuenta",
            "CuentaCreada",
            "2026-10-14T12:00:00Z",
            "d1",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cuentas
                (id, nombre, tipo, moneda, saldo_inicial, saldo_actual, estado, evento_origen, actualizado_en)
             VALUES ('c1', 'Cartera', 'efectivo', 'USD', '100.00', '100.00', 'activa', 'e-cuenta', '2026-10-14T12:00:00Z')",
            [],
        )
        .unwrap();

        // La Entrega 2 no tenía estas tablas: se eliminan para reproducir el
        // estado del archivo que se abrirá con la versión nueva.
        conn.execute_batch("DROP TABLE movimientos; DROP TABLE categorias;")
            .unwrap();

        // Apertura con la versión nueva: se crean las tablas que faltan, se
        // siembran las categorías de fábrica y se reconstruyen las proyecciones.
        crate::db::inicializar(&conn).unwrap();
        crate::db::sembrar_categorias_fabrica(&conn).unwrap();
        reconstruir_proyecciones(&conn).unwrap();

        let repo = RepositorioEventosSqlite::nuevo(conn);
        let cuentas = repo.listar_cuentas().unwrap();
        assert_eq!(cuentas.len(), 1, "la cuenta de la Entrega 2 no se pierde");
        assert_eq!(cuentas[0].id, "c1");
        assert_eq!(cuentas[0].nombre, "Cartera");
        assert_eq!(cuentas[0].saldo_inicial, "100.00");
        assert_eq!(cuentas[0].saldo_actual, "100.00");
        assert_eq!(
            repo.listar_categorias().unwrap().len(),
            11,
            "se reconstruyen las categorías de fábrica desde sus eventos"
        );
        assert!(
            repo.listar_movimientos().unwrap().is_empty(),
            "la Entrega 2 no tenía movimientos"
        );
    }

    /// Repositorio SQLite con el esquema, la siembra canónica y las proyecciones
    /// reconstruidas, listo para recibir eventos.
    fn repositorio_con_fabrica() -> RepositorioEventosSqlite {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::inicializar(&conn).unwrap();
        crate::db::sembrar_categorias_fabrica(&conn).unwrap();
        reconstruir_proyecciones(&conn).unwrap();
        RepositorioEventosSqlite::nuevo(conn)
    }

    #[test]
    fn siembra_y_reconstruccion_no_duplica_categorias() {
        let repo = repositorio_con_fabrica();
        let categorias = repo.listar_categorias().unwrap();

        assert_eq!(categorias.len(), 11, "no debe haber categorías duplicadas");
        let mut claves = std::collections::HashSet::new();
        for categoria in &categorias {
            assert!(
                claves.insert((categoria.tipo.clone(), categoria.nombre.to_lowercase())),
                "categoría duplicada: {} ({})",
                categoria.nombre,
                categoria.tipo
            );
        }
        assert!(
            categorias.iter().any(|c| c.id == "gasto:alimentacion"),
            "debe existir el ID canónico de Alimentación"
        );
        assert!(
            categorias.iter().all(|c| c.es_fabrica),
            "las 11 categorías sembradas son de fábrica"
        );
    }

    #[test]
    fn movimiento_con_id_de_dominio_no_viola_la_fk() {
        let mut repo = repositorio_con_fabrica();
        guardar_evento(
            &mut repo,
            "e-cuenta",
            "CuentaCreada",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        );
        guardar_evento(
            &mut repo,
            "e-mov",
            "MovimientoRegistrado",
            r#"{"id":"m1","cuenta_id":"c1","tipo":"egreso","monto":"25.00","fecha":"2026-10-19","categoria_id":"gasto:alimentacion"}"#,
        );

        let movimientos = repo.listar_movimientos().unwrap();
        assert_eq!(movimientos.len(), 1);
        assert_eq!(movimientos[0].categoria_id, "gasto:alimentacion");
        assert_eq!(
            repo.listar_cuentas().unwrap()[0].saldo_actual,
            "75.00",
            "el movimiento debe afectar el saldo"
        );
    }

    #[test]
    fn reconstruir_traduce_los_ids_historicos_de_categoria() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::inicializar(&conn).unwrap();

        // Simula el libro de una base anterior: eventos con IDs cat-fabrica-*.
        for categoria in dominio::categorias_de_fabrica() {
            let legado = aplicacion::categorias::id_categoria_legado(&categoria);
            let tipo = match categoria.tipo {
                dominio::TipoMovimiento::Gasto => "egreso",
                dominio::TipoMovimiento::Ingreso => "ingreso",
            };
            let datos = format!(
                r#"{{"id":"{legado}","nombre":"{}","tipo":"{tipo}","es_fabrica":true}}"#,
                categoria.nombre
            );
            crate::db::insertar_evento(
                &conn,
                &legado,
                "CategoriaCreada",
                "2026-01-01T00:00:00Z",
                "fabrica",
                &datos,
            )
            .unwrap();
        }
        crate::db::insertar_evento(
            &conn,
            "e-cuenta",
            "CuentaCreada",
            "2026-10-14T12:00:00Z",
            "d1",
            r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#,
        )
        .unwrap();
        crate::db::insertar_evento(
            &conn,
            "e-mov",
            "MovimientoRegistrado",
            "2026-10-19T12:00:00Z",
            "d1",
            r#"{"id":"m1","cuenta_id":"c1","tipo":"egreso","monto":"10.00","fecha":"2026-10-19","categoria_id":"cat-fabrica-egreso-alimentacion"}"#,
        )
        .unwrap();

        reconstruir_proyecciones(&conn).unwrap();
        let repo = RepositorioEventosSqlite::nuevo(conn);

        let categorias = repo.listar_categorias().unwrap();
        assert_eq!(categorias.len(), 11);
        assert!(categorias.iter().any(|c| c.id == "gasto:alimentacion"));
        assert!(
            !categorias.iter().any(|c| c.id.starts_with("cat-fabrica-")),
            "la proyección no debe conservar IDs históricos"
        );

        let movimientos = repo.listar_movimientos().unwrap();
        assert_eq!(movimientos.len(), 1);
        assert_eq!(
            movimientos[0].categoria_id, "gasto:alimentacion",
            "el movimiento histórico debe apuntar al ID canónico"
        );
    }

    #[test]
    fn flujo_completo_con_sqlite_usa_categorias_canonicas() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let ruta = std::env::temp_dir().join(format!("cuentas_claras_categorias_{nanos}.db"));

        {
            let bd = crate::db::BaseDatos::persistente(&ruta).unwrap();
            let repo = RepositorioEventosSqlite::with_conn(bd.conexion());
            let mut app = aplicacion::CasosDeUso::nuevo(
                Box::new(repo),
                Box::new(aplicacion::RelojFijo::nuevo("2026-10-19T12:00:00Z")),
                Box::new(aplicacion::GeneradorSecuencial::nuevo()),
                Box::new(aplicacion::DispositivoFijo::nuevo("disp-1")),
            );

            let categorias = app.listar_categorias().unwrap();
            assert_eq!(categorias.len(), 11);
            assert!(
                categorias
                    .iter()
                    .any(|c| c.id == "gasto:alimentacion" && c.de_fabrica)
            );

            let cuenta = app
                .crear_cuenta(aplicacion::NuevaCuenta {
                    nombre: "Cartera".to_string(),
                    tipo: "efectivo".to_string(),
                    moneda: "USD".to_string(),
                    saldo_inicial: "100.00".to_string(),
                })
                .unwrap();
            app.registrar_movimiento(aplicacion::NuevoMovimiento {
                cuenta_id: cuenta.id.clone(),
                tipo: "gasto".to_string(),
                monto: "25.00".to_string(),
                fecha: "2026-10-19".to_string(),
                categoria_id: "gasto:alimentacion".to_string(),
                nota: None,
            })
            .unwrap();

            let detalle = app.detalle_cuenta(&cuenta.id).unwrap();
            assert_eq!(detalle.movimientos.len(), 1);
            assert_eq!(detalle.cuenta.saldo_actual, "75.00");
        }

        // Reabrir la base demuestra que la proyección persistida quedó con el ID canónico.
        {
            let bd = crate::db::BaseDatos::persistente(&ruta).unwrap();
            let conn = bd.conexion();
            let conn = conn.lock().unwrap();
            let total: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM movimientos WHERE categoria_id = 'gasto:alimentacion'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(total, 1);
        }

        let _ = std::fs::remove_file(&ruta);
        let _ = std::fs::remove_file(ruta.with_extension("db-wal"));
        let _ = std::fs::remove_file(ruta.with_extension("db-shm"));
    }
}
