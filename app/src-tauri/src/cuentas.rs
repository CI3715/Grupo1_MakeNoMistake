//! Comandos de Tauri para la gestión de cuentas.
//!
//! Traducen las invocaciones de la interfaz hacia los casos de uso de la capa de
//! aplicación. La interfaz no conoce ni la base de datos ni las reglas de negocio:
//! solo envía datos y recibe modelos de lectura ya formateados.

use std::sync::Mutex;

use aplicacion::{
    CasosDeUso, CuentaListada, ErrorAplicacion, ListaCuentas, MonedaDto, NuevaCuenta,
};
use serde::Serialize;
use tauri::State;

/// Estado compartido de la aplicación: los casos de uso protegidos por un mutex
/// para poder usarse desde los distintos hilos que atienden los comandos.
pub struct ServicioCuentas {
    casos: Mutex<CasosDeUso>,
}

impl ServicioCuentas {
    /// Envuelve los casos de uso ya cableados con sus adaptadores reales.
    pub fn nuevo(casos: CasosDeUso) -> Self {
        ServicioCuentas {
            casos: Mutex::new(casos),
        }
    }
}

/// Error devuelto por un comando, serializable hacia la interfaz.
///
/// Nunca expone detalles internos: los fallos de almacenamiento o internos se
/// resumen en una categoría para que la interfaz muestre un mensaje genérico.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "tipo")]
pub enum ErrorComando {
    /// La entrada no superó las validaciones; se detalla cada campo afectado.
    Validacion { errores: Vec<ErrorCampo> },
    /// Falló la lectura o escritura del almacenamiento local.
    Almacenamiento,
    /// Ocurrió un problema interno inesperado.
    Interno,
}

/// Detalle de un error de validación asociado (o no) a un campo del formulario.
#[derive(Debug, Serialize)]
pub struct ErrorCampo {
    /// Nombre del campo afectado, o `None` si el error es general.
    pub campo: Option<String>,
    /// Descripción legible del problema.
    pub mensaje: String,
}

impl From<ErrorAplicacion> for ErrorComando {
    fn from(error: ErrorAplicacion) -> Self {
        match error {
            ErrorAplicacion::Validacion(errores) => ErrorComando::Validacion {
                errores: errores
                    .into_iter()
                    .map(|e| ErrorCampo {
                        campo: e.campo,
                        mensaje: e.mensaje,
                    })
                    .collect(),
            },
            ErrorAplicacion::Almacenamiento(_) => ErrorComando::Almacenamiento,
            ErrorAplicacion::Interno(_) => ErrorComando::Interno,
        }
    }
}

/// Devuelve el catálogo de monedas soportadas (VES, USD, EUR, USDT).
#[tauri::command]
pub fn listar_monedas(servicio: State<'_, ServicioCuentas>) -> Vec<MonedaDto> {
    casos(&servicio).listar_monedas()
}

/// Devuelve las cuentas registradas y sus subtotales por moneda.
#[tauri::command]
pub fn listar_cuentas(servicio: State<'_, ServicioCuentas>) -> Result<ListaCuentas, ErrorComando> {
    casos(&servicio)
        .listar_cuentas()
        .map_err(ErrorComando::from)
}

/// Registra una nueva cuenta y devuelve su representación lista para mostrar.
#[tauri::command]
pub fn crear_cuenta(
    servicio: State<'_, ServicioCuentas>,
    entrada: NuevaCuenta,
) -> Result<CuentaListada, ErrorComando> {
    let mut casos = servicio
        .casos
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner());
    casos.crear_cuenta(entrada).map_err(ErrorComando::from)
}

/// Toma prestado el servicio recuperando el estado aunque el mutex estuviera envenenado.
fn casos<'a>(servicio: &'a State<'_, ServicioCuentas>) -> std::sync::MutexGuard<'a, CasosDeUso> {
    servicio
        .casos
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner())
}
