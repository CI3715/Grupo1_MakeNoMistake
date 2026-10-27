mod cuentas;
mod dto;
mod movimientos;
mod ping;
mod tasas;

use std::sync::{Arc, Mutex};

use aplicacion::{CasosDeUso, RepositorioEventos as _};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;
use tauri::Manager;

/// Estado compartido de Tauri. El `Arc<Mutex<>>` vive solo aquí (crate `app`),
/// como indica la Guía: dominio y aplicación no lo conocen.
pub type EstadoCasosDeUso = Arc<Mutex<CasosDeUso>>;

/// Arma los casos de uso para las pruebas de los comandos: repositorio en
/// memoria (no toca el disco) y un dispositivo fijo, sin base de datos.
#[cfg(test)]
fn crear_casos_de_uso() -> CasosDeUso {
    use aplicacion::{DispositivoOrigen, RepositorioEnMemoria};

    struct DispositivoDePrueba;

    impl DispositivoOrigen for DispositivoDePrueba {
        fn id_dispositivo(&self) -> String {
            "dispositivo-de-prueba".to_string()
        }
    }

    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(DispositivoDePrueba),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ping::ping_servidor,
            cuentas::listar_monedas,
            cuentas::listar_cuentas,
            cuentas::crear_cuenta,
            movimientos::registrar_movimiento,
            movimientos::corregir_movimiento,
            movimientos::previsualizar_movimiento,
            movimientos::listar_categorias,
            movimientos::crear_categoria,
            movimientos::detalle_cuenta,
            tasas::registrar_tasa,
            tasas::eliminar_tasa,
            tasas::listar_tasas,
            tasas::cambiar_moneda_base,
            tasas::crear_moneda,
            tasas::consultar_patrimonio
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Base de datos local persistente (SQLite embebido).
            // El archivo vive en el directorio de datos de la aplicación y
            // sobrevive al cierre y reapertura.
            let carpeta = app.path().app_data_dir()?;
            std::fs::create_dir_all(&carpeta)?;
            let ruta = carpeta.join("cuentas_claras.db");
            let base_datos = BaseDatos::persistente(&ruta)?;
            log::info!("Base de datos local lista en {}", ruta.display());

            // Cablea los casos de uso con sus adaptadores reales sobre la misma
            // conexión: repositorio de eventos y proyección, reloj, identificadores
            // y el identificador persistente de este dispositivo.
            let conexion = base_datos.conexion();
            let mut repositorio = RepositorioEventosSqlite::with_conn(Arc::clone(&conexion));

            // Reconstruye las proyecciones desde el libro de eventos al abrir.
            // Así una base de la Entrega 2 (que no tenía las proyecciones de
            // categorías y movimientos) se abre sin perder cuentas, y cualquier
            // desincronización de la caché se corrige sola. El libro de eventos
            // nunca se toca.
            repositorio.reconstruir()?;
            log::info!("Proyecciones reconstruidas desde el libro de eventos");

            let dispositivo = DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion))?;
            let casos = CasosDeUso::nuevo(
                Box::new(repositorio),
                Box::new(RelojSistema::nuevo()),
                Box::new(GeneradorUuid::nuevo()),
                Box::new(dispositivo),
            );

            let estado: EstadoCasosDeUso = Arc::new(Mutex::new(casos));
            app.manage(estado);
            app.manage(base_datos);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
