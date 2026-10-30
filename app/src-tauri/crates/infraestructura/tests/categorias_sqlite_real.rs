//! C-02: prueba con el repositorio SQLite REAL (no el de memoria).
//!
//! El repositorio en memoria no detecta dos clases de fallo que sí ocurren con
//! SQLite: categorías duplicadas por dos juegos de identificadores (C-01) y la
//! clave foránea de `movimientos.categoria_id`. Esta prueba abre una base
//! temporal en un archivo, comprueba que existen exactamente las 11 categorías
//! de fábrica sin duplicados y registra un movimiento con cada una. Falla si:
//!   - aparece una categoría duplicada (por id o por tipo y nombre),
//!   - los ids de SQLite no coinciden con los del dominio,
//!   - un movimiento no se puede guardar (clave foránea rota),
//!   - el saldo no coincide con la suma de los movimientos (INV-05).
//!
//! Corre con `cargo test --workspace`, o solo esta con
//! `cargo test -p infraestructura --test categorias_sqlite_real`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aplicacion::{CasosDeUso, NuevaCuenta, NuevoMovimiento, RepositorioEventos};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;

/// Archivo SQLite temporal que se borra (con sus `-wal` y `-shm`) al terminar,
/// aunque la prueba falle.
struct BaseTemporal {
    ruta: PathBuf,
}

impl BaseTemporal {
    fn nueva() -> Self {
        let nombre = format!("cuentas_claras_prueba_{}.db", uuid::Uuid::new_v4());
        BaseTemporal {
            ruta: std::env::temp_dir().join(nombre),
        }
    }
}

impl Drop for BaseTemporal {
    fn drop(&mut self) {
        for sufijo in ["", "-wal", "-shm"] {
            let mut archivo = self.ruta.clone().into_os_string();
            archivo.push(sufijo);
            let _ = std::fs::remove_file(PathBuf::from(archivo));
        }
    }
}

/// Abre la base igual que la aplicación real (`BaseDatos::persistente`, el mismo
/// cableado de `src/lib.rs`) y devuelve los casos de uso más un lector directo
/// de las proyecciones de SQLite.
fn abrir_app(ruta: &Path) -> (CasosDeUso, RepositorioEventosSqlite) {
    let base = BaseDatos::persistente(ruta).expect("abrir la base SQLite temporal");
    let conexion = base.conexion();
    let lector = RepositorioEventosSqlite::with_conn(Arc::clone(&conexion));
    let dispositivo =
        DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion)).expect("identificar el dispositivo");
    let casos = CasosDeUso::nuevo(
        Box::new(RepositorioEventosSqlite::with_conn(Arc::clone(&conexion))),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(dispositivo),
    );
    (casos, lector)
}

/// Comprueba el catálogo de fábrica desde los dos lados: lo que ve la aplicación
/// (casos de uso) y lo que hay realmente en la proyección de SQLite.
fn comprobar_catalogo_de_fabrica(casos: &CasosDeUso, lector: &RepositorioEventosSqlite) {
    let esperadas = dominio::categorias_de_fabrica();
    assert_eq!(esperadas.len(), 11, "el dominio debe definir 11 categorías");
    let ids_dominio: HashSet<&str> = esperadas.iter().map(|c| c.id.as_str()).collect();

    // Lo que ve la aplicación.
    let listadas = casos.listar_categorias().expect("listar categorías");
    let de_fabrica: Vec<_> = listadas.iter().filter(|c| c.de_fabrica).collect();
    assert_eq!(
        de_fabrica.len(),
        11,
        "debe haber exactamente 11 categorías de fábrica, hay {}: {:?}",
        de_fabrica.len(),
        de_fabrica.iter().map(|c| c.id.as_str()).collect::<Vec<_>>()
    );
    let ids: HashSet<&str> = de_fabrica.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 11, "hay ids de categoría repetidos");
    let por_nombre: HashSet<(String, String)> = de_fabrica
        .iter()
        .map(|c| (c.tipo.clone(), c.nombre.trim().to_lowercase()))
        .collect();
    assert_eq!(
        por_nombre.len(),
        11,
        "hay categorías repetidas con el mismo tipo y nombre"
    );
    assert_eq!(
        ids, ids_dominio,
        "los ids de las categorías de fábrica deben ser los del dominio (un solo juego de ids)"
    );

    // Lo que hay realmente en la tabla `categorias` de SQLite.
    let proyectadas = lector
        .listar_categorias()
        .expect("leer la proyección de categorías");
    let fabrica_sqlite: Vec<_> = proyectadas.iter().filter(|c| c.es_fabrica).collect();
    assert_eq!(
        fabrica_sqlite.len(),
        11,
        "la tabla categorias debe tener 11 filas de fábrica, tiene {}",
        fabrica_sqlite.len()
    );
    let ids_sqlite: HashSet<&str> = fabrica_sqlite.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids_sqlite, ids_dominio,
        "los ids guardados en SQLite deben ser los del dominio"
    );
}

#[test]
fn once_categorias_sin_duplicados_y_un_movimiento_con_cada_una() {
    let temporal = BaseTemporal::nueva();
    let (mut casos, lector) = abrir_app(&temporal.ruta);

    comprobar_catalogo_de_fabrica(&casos, &lector);

    let cuenta = casos
        .crear_cuenta(NuevaCuenta {
            nombre: "Cuenta de prueba".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "1000.00".to_string(),
        })
        .expect("crear la cuenta de prueba");

    // Un movimiento con CADA categoría de fábrica: gastos de 10,00 e ingresos de 5,00.
    let categorias: Vec<_> = casos
        .listar_categorias()
        .expect("listar categorías")
        .into_iter()
        .filter(|c| c.de_fabrica)
        .collect();
    let mut gastos = 0;
    let mut ingresos = 0;
    for categoria in &categorias {
        let monto = if categoria.tipo == "gasto" {
            gastos += 1;
            "10.00"
        } else {
            ingresos += 1;
            "5.00"
        };
        casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.id.clone(),
                tipo: categoria.tipo.clone(),
                monto: monto.to_string(),
                fecha: String::new(), // vacía: usa la fecha de hoy del reloj
                categoria_id: categoria.id.clone(),
                nota: None,
            })
            .unwrap_or_else(|e| {
                panic!(
                    "no se pudo registrar un movimiento con la categoría '{}' ({}): {e:?}",
                    categoria.nombre, categoria.id
                )
            });
    }
    assert_eq!(
        (gastos, ingresos),
        (7, 4),
        "deben ser 7 de gasto y 4 de ingreso"
    );

    // INV-05: saldo = saldo inicial + ingresos - gastos = 1000 + 4*5 - 7*10.
    let detalle = casos
        .detalle_cuenta(&cuenta.id)
        .expect("detalle de la cuenta");
    assert_eq!(detalle.cuenta.saldo_actual, "950.00");
    assert_eq!(detalle.movimientos.len(), 11);

    // Cada categoría de fábrica quedó usada exactamente una vez, en la base real.
    let usadas: HashSet<String> = detalle
        .movimientos
        .iter()
        .map(|m| m.categoria.id.clone())
        .collect();
    let esperadas: HashSet<String> = dominio::categorias_de_fabrica()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(usadas, esperadas);

    let guardados = lector
        .listar_movimientos()
        .expect("leer la proyección de movimientos");
    assert_eq!(
        guardados.len(),
        11,
        "SQLite debe guardar los 11 movimientos"
    );
}

#[test]
fn reabrir_la_base_no_duplica_categorias_ni_pierde_movimientos() {
    let temporal = BaseTemporal::nueva();

    // Primera apertura: una cuenta y un gasto con una categoría de fábrica.
    let cuenta_id = {
        let (mut casos, lector) = abrir_app(&temporal.ruta);
        comprobar_catalogo_de_fabrica(&casos, &lector);
        let cuenta = casos
            .crear_cuenta(NuevaCuenta {
                nombre: "Cuenta de prueba".to_string(),
                tipo: "banco".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "1000.00".to_string(),
            })
            .expect("crear la cuenta de prueba");
        casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.id.clone(),
                tipo: "gasto".to_string(),
                monto: "10.00".to_string(),
                fecha: String::new(),
                categoria_id: "gasto:alimentacion".to_string(),
                nota: Some("Mercado".to_string()),
            })
            .expect("registrar el gasto");
        cuenta.id
    };

    // Cerrar y reabrir varias veces: las categorías no se duplican y los datos
    // siguen idénticos (persistencia, RNF-03).
    for _ in 0..3 {
        let (casos, lector) = abrir_app(&temporal.ruta);
        comprobar_catalogo_de_fabrica(&casos, &lector);

        let detalle = casos
            .detalle_cuenta(&cuenta_id)
            .expect("detalle tras reabrir");
        assert_eq!(detalle.cuenta.saldo_actual, "990.00");
        assert_eq!(detalle.movimientos.len(), 1);
        assert_eq!(detalle.movimientos[0].categoria.id, "gasto:alimentacion");
    }
}
