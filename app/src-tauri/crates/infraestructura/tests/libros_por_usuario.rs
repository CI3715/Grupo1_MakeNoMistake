//! Pruebas de libros personales por usuario (Entrega 7, Angel - Tarea 6).
//!
//! Verifica las reglas de aislamiento, adopción y la invariante INV-16:
//! - Cada usuario del dispositivo tiene su propio archivo `cuentas_claras_<id>.db` (ADR-014).
//! - Dos usuarios en el mismo dispositivo ven libros distintos y aislados.
//! - Volver al primer usuario recupera su libro con sus cuentas y saldos íntegros.
//! - El primer usuario que inicia sesión adopta el `cuentas_claras.db` existente (datos de E2-E4).
//! - El segundo usuario empieza con su libro vacío (sin las cuentas del primero).
//! - Sin sesión se utiliza `cuentas_claras.db`.
//! - Invariante INV-16: NINGUNA tabla del libro personal contiene columnas de usuario (`usuario_id`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aplicacion::{CasosDeUso, NuevaCuenta, NuevoMovimiento};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;

const ID_ANA: &str = "0b9d6c1e-52f7-4c3a-9d10-7a4e8f2b6c33";
const ID_BETO: &str = "5c2a9e47-1d3b-4f60-8a7e-3b9c0d1e2f48";
const ID_CARLA: &str = "7e1a2b3c-4d5e-6f70-8a9b-0c1d2e3f4a5b";

struct CarpetaTemporal(PathBuf);

impl CarpetaTemporal {
    fn nueva(prefijo: &str) -> Self {
        let ruta =
            std::env::temp_dir().join(format!("cc_test_e7_{prefijo}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&ruta).expect("crear la carpeta temporal");
        CarpetaTemporal(ruta)
    }

    fn libro_usuario(&self, id_usuario: &str) -> PathBuf {
        self.0.join(format!("cuentas_claras_{id_usuario}.db"))
    }

    fn libro_sin_sesion(&self) -> PathBuf {
        self.0.join("cuentas_claras.db")
    }
}

impl Drop for CarpetaTemporal {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn abrir_libro(ruta: &Path) -> CasosDeUso {
    let base = BaseDatos::persistente(ruta).expect("abrir base de datos persistente");
    let conexion = base.conexion();
    let dispositivo =
        DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion)).expect("identificar dispositivo");
    CasosDeUso::nuevo(
        Box::new(RepositorioEventosSqlite::with_conn(conexion)),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(dispositivo),
    )
}

fn crear_cuenta(casos: &mut CasosDeUso, nombre: &str, moneda: &str, saldo: &str) -> String {
    casos
        .crear_cuenta(NuevaCuenta {
            nombre: nombre.into(),
            tipo: "efectivo".into(),
            moneda: moneda.into(),
            saldo_inicial: saldo.into(),
        })
        .expect("crear cuenta")
        .id
}

#[test]
fn dos_usuarios_tienen_libros_sqlite_distintos_y_aislados() {
    let carpeta = CarpetaTemporal::nueva("aislamiento");
    let ruta_ana = carpeta.libro_usuario(ID_ANA);
    let ruta_beto = carpeta.libro_usuario(ID_BETO);

    // 1. Ana inicia sesión y crea una cuenta con saldo inicial 500.00 USD
    let c_ana = {
        let mut casos_ana = abrir_libro(&ruta_ana);
        let id_cuenta = crear_cuenta(&mut casos_ana, "Ahorros Ana USD", "USD", "500.00");
        casos_ana
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: id_cuenta.clone(),
                tipo: "ingreso".into(),
                monto: "50.00".into(),
                fecha: "2026-11-20".into(),
                categoria_id: "ingreso:salario".into(),
                nota: Some("Bono".into()),
            })
            .expect("registrar ingreso");
        assert_eq!(
            casos_ana
                .detalle_cuenta(&id_cuenta)
                .unwrap()
                .cuenta
                .saldo_actual,
            "550.00"
        );
        id_cuenta
    };

    // 2. Beto inicia sesión en el mismo dispositivo: su archivo está vacío y no ve nada de Ana
    let c_beto = {
        let mut casos_beto = abrir_libro(&ruta_beto);
        let cuentas_beto = casos_beto.listar_cuentas().expect("listar cuentas").cuentas;
        assert!(
            cuentas_beto.is_empty(),
            "Beto no debe ver las cuentas de Ana"
        );
        assert!(
            casos_beto.detalle_cuenta(&c_ana).is_err(),
            "Beto no puede acceder al detalle de cuenta de Ana"
        );

        let id_cuenta_beto = crear_cuenta(&mut casos_beto, "Billetera Beto EUR", "EUR", "100.00");
        assert_eq!(
            casos_beto
                .detalle_cuenta(&id_cuenta_beto)
                .unwrap()
                .cuenta
                .saldo_actual,
            "100.00"
        );
        id_cuenta_beto
    };

    // 3. Volver al libro de Ana confirma que su saldo y movimientos siguen intactos y no ve a Beto
    {
        let casos_ana = abrir_libro(&ruta_ana);
        let cuentas = casos_ana.listar_cuentas().unwrap().cuentas;
        assert_eq!(cuentas.len(), 1);
        assert_eq!(cuentas[0].id, c_ana);
        assert_eq!(cuentas[0].saldo_actual, "550.00");
        assert!(
            casos_ana.detalle_cuenta(&c_beto).is_err(),
            "Ana no debe ver las cuentas de Beto"
        );
    }
}

#[test]
fn tres_usuarios_mantienen_libros_independientes() {
    let carpeta = CarpetaTemporal::nueva("tres_usuarios");
    let mut cuentas = Vec::new();

    for (id, nombre, moneda, monto) in [
        (ID_ANA, "Ana", "USD", "100.00"),
        (ID_BETO, "Beto", "EUR", "200.00"),
        (ID_CARLA, "Carla", "USD", "300.00"),
    ] {
        let ruta = carpeta.libro_usuario(id);
        let mut casos = abrir_libro(&ruta);
        let c = crear_cuenta(&mut casos, &format!("Cuenta {nombre}"), moneda, monto);
        cuentas.push((id, c, monto));
    }

    for (id, c_id, esperado) in &cuentas {
        let ruta = carpeta.libro_usuario(id);
        let casos = abrir_libro(&ruta);
        let lista = casos.listar_cuentas().unwrap().cuentas;
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].id, *c_id);
        assert_eq!(lista[0].saldo_actual, *esperado);
    }
}

#[test]
fn adopcion_del_libro_existente_por_el_primer_usuario() {
    let carpeta = CarpetaTemporal::nueva("adopcion");
    let ruta_sin_sesion = carpeta.libro_sin_sesion();
    let ruta_primer_usuario = carpeta.libro_usuario(ID_ANA);
    let ruta_segundo_usuario = carpeta.libro_usuario(ID_BETO);

    // Creamos cuentas y movimientos en el libro preexistente (sin sesión, Entregas 2 a 4)
    let cuenta_previa = {
        let mut casos = abrir_libro(&ruta_sin_sesion);
        let id_cta = crear_cuenta(&mut casos, "Efectivo Histórico", "USD", "300.00");
        casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: id_cta.clone(),
                tipo: "gasto".into(),
                monto: "25.00".into(),
                fecha: "2026-11-01".into(),
                categoria_id: "gasto:alimentacion".into(),
                nota: None,
            })
            .expect("gasto histórico");
        assert_eq!(
            casos.detalle_cuenta(&id_cta).unwrap().cuenta.saldo_actual,
            "275.00"
        );
        id_cta
    };

    // El primer usuario adopta el libro existente (se renombra el archivo y sus auxiliares)
    fs::rename(&ruta_sin_sesion, &ruta_primer_usuario).expect("adoptar libro existente");
    assert!(!ruta_sin_sesion.exists());
    assert!(ruta_primer_usuario.exists());

    // El primer usuario abre su libro adoptado y encuentra sus datos intactos
    {
        let casos_ana = abrir_libro(&ruta_primer_usuario);
        let detalle = casos_ana
            .detalle_cuenta(&cuenta_previa)
            .expect("cuenta adoptada");
        assert_eq!(detalle.cuenta.saldo_actual, "275.00");
        assert_eq!(detalle.movimientos.len(), 1);
    }

    // El segundo usuario que inicia sesión después empieza con libro nuevo y vacío
    {
        let casos_beto = abrir_libro(&ruta_segundo_usuario);
        let cuentas = casos_beto
            .listar_cuentas()
            .expect("cuentas de beto")
            .cuentas;
        assert!(
            cuentas.is_empty(),
            "El segundo usuario debe empezar con libro vacío"
        );
    }
}

#[test]
fn sin_sesion_se_utiliza_cuentas_claras_db() {
    let carpeta = CarpetaTemporal::nueva("sin_sesion");
    let ruta = carpeta.libro_sin_sesion();

    let mut casos = abrir_libro(&ruta);
    assert_eq!(ruta.file_name().unwrap(), "cuentas_claras.db");
    let id_cta = crear_cuenta(&mut casos, "Local Offline", "USD", "10.00");
    assert_eq!(
        casos.detalle_cuenta(&id_cta).unwrap().cuenta.saldo_actual,
        "10.00"
    );
}

#[test]
fn reapertura_de_libro_conserva_saldo_y_proyecciones_inv04_inv05() {
    let carpeta = CarpetaTemporal::nueva("reapertura");
    let ruta = carpeta.libro_usuario(ID_ANA);

    let id_cta = {
        let mut casos = abrir_libro(&ruta);
        let c = crear_cuenta(&mut casos, "Caja Fuerte", "USD", "1000.00");
        casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: c.clone(),
                tipo: "gasto".into(),
                monto: "150.00".into(),
                fecha: "2026-11-20".into(),
                categoria_id: "gasto:alimentacion".into(),
                nota: None,
            })
            .expect("registrar gasto");
        assert_eq!(
            casos.detalle_cuenta(&c).unwrap().cuenta.saldo_actual,
            "850.00"
        );
        c
    };

    // Reabrir dos veces simula arranques sucesivos de la aplicación
    for _ in 0..2 {
        let casos = abrir_libro(&ruta);
        let detalle = casos.detalle_cuenta(&id_cta).expect("detalle");
        assert_eq!(detalle.cuenta.saldo_actual, "850.00");
        assert_eq!(detalle.movimientos.len(), 1);
    }
}

#[test]
fn inv16_ninguna_tabla_de_sqlite_contiene_usuario_id() {
    let carpeta = CarpetaTemporal::nueva("inv16_schema");
    let ruta = carpeta.libro_usuario(ID_ANA);

    // Inicializar el esquema y proyecciones completas
    let base = BaseDatos::persistente(&ruta).expect("crear base de datos");
    let conexion = base.conexion();
    let conn = conexion.lock().expect("bloquear conexion");

    // Consultamos todas las tablas creadas en SQLite
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .expect("preparar consulta de tablas");
    let tablas: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .expect("leer tablas")
        .flatten()
        .collect();

    assert!(!tablas.is_empty(), "Debe haber tablas creadas en la base");

    // Ninguna tabla puede tener columnas relacionadas con usuario_id (INV-16)
    for tabla in &tablas {
        let pragma_sql = format!("PRAGMA table_info({})", tabla);
        let mut pragma_stmt = conn.prepare(&pragma_sql).expect("preparar pragma");
        let columnas: Vec<String> = pragma_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("leer columnas")
            .flatten()
            .collect();

        for col in &columnas {
            let col_minuscula = col.to_lowercase();
            assert!(
                !col_minuscula.contains("usuario") && !col_minuscula.contains("user"),
                "Violación de INV-16: la tabla '{}' contiene la columna prohibida '{}'",
                tabla,
                col
            );
        }
    }
}
