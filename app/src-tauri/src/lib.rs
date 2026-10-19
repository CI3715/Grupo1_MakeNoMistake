mod cuentas;
mod ping;

use std::sync::Arc;

use aplicacion::{CasosDeUso, RepositorioEventos as _};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ping::ping_servidor,
            cuentas::crear_cuenta,
            cuentas::listar_cuentas,
            cuentas::listar_monedas,
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
            // conexión: repositorio de eventos/proyecciones, reloj, identificadores
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
            app.manage(cuentas::ServicioCuentas::nuevo(casos));
            app.manage(base_datos);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
