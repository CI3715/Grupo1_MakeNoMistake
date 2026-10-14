mod cuentas;
mod dto;
mod ping;

use std::sync::{Arc, Mutex};

use aplicacion::{CasosDeUso, RepositorioEnMemoria};
use infraestructura::{DispositivoGenerado, GeneradorUuid, RelojSistema};
use tauri::Manager;

/// Estado compartido de Tauri. El `Arc<Mutex<>>` vive solo aquí (crate `app`),
/// como indica la Guía: dominio y aplicación no lo conocen.
pub type EstadoCasosDeUso = Arc<Mutex<CasosDeUso>>;

/// Arma los casos de uso con sus adaptadores.
/// Por ahora el repositorio es en memoria: las cuentas se pierden al cerrar la app.
fn crear_casos_de_uso() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojSistema),
        Box::new(GeneradorUuid),
        Box::new(DispositivoGenerado::nuevo()),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ping::ping_servidor,
            cuentas::listar_monedas,
            cuentas::listar_cuentas,
            cuentas::crear_cuenta
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let estado: EstadoCasosDeUso = Arc::new(Mutex::new(crear_casos_de_uso()));
            app.manage(estado);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
