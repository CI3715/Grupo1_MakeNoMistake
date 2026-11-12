//! Tipos de error de la capa de aplicación.
//!
//! Separa los errores de validación de entrada (asociados a campos específicos)
//! de los fallos de almacenamiento o errores internos de procesamiento.

use crate::puertos::ErrorRepositorio;
use thiserror::Error;

/// Error de validación vinculado a un campo específico de los datos de entrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorValidacion {
    /// Nombre del campo afectado (`"nombre"`, `"tipo"`, `"moneda"`, `"saldo_inicial"`), o `None` si es general.
    pub campo: Option<String>,
    /// Descripción legible de la causa del fallo de validación.
    pub mensaje: String,
}

/// Categorías de error que pueden ocurrir al ejecutar un caso de uso.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorAplicacion {
    /// Uno o más campos de entrada no superaron las reglas de validación.
    #[error("Hay datos que corregir antes de continuar.")]
    Validacion(Vec<ErrorValidacion>),

    /// Error originado al consultar o escribir en el repositorio de eventos.
    #[error("No se pudo leer o guardar en el almacenamiento del dispositivo. Intenta de nuevo.")]
    Almacenamiento(#[from] ErrorRepositorio),

    /// Error inesperado al serializar, deserializar o reconstruir el estado interno.
    #[error("Ocurrió un problema interno. Intenta de nuevo.")]
    Interno(String),
}
