//! DTO de la sincronización con el servidor (Entrega 7).
//!
//! Mismas convenciones que `dto_social.rs`: montos como texto con punto decimal,
//! fechas AAAA-MM-DD, nombres en snake_case y errores con `ErroresDto`.
//! Nada del libro personal viaja al servidor: estos DTO solo van entre la app y la
//! interfaz. La cuenta elegida para cada grupo vive solo en el dispositivo.

use serde::{Deserialize, Serialize};

/// Una parte de gasto de grupo que bajó del servidor y aún no tiene cuenta donde aplicarse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParteGrupoPendienteDto {
    pub evento_id: String,
    pub grupo_id: String,
    pub grupo_nombre: String,
    pub concepto: String,
    pub monto: String,
    pub moneda: String,
    pub fecha: String,
}

/// Un gasto de grupo en la cola local. `estado` es "pendiente" (esperando envío)
/// o "rechazado" (el servidor lo rechazó; `motivo` dice por qué).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GastoEnColaDto {
    pub clave: String,
    pub grupo_id: String,
    pub grupo_nombre: String,
    pub concepto: String,
    pub monto: String,
    pub moneda: String,
    pub fecha: String,
    pub estado: String,
    pub motivo: Option<String>,
}

/// Todo lo que la pantalla de sincronización necesita en una sola llamada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoSincronizacionDto {
    /// Fecha y hora de la última sincronización exitosa; `None` si aún no hubo ninguna.
    pub ultima_sincronizacion: Option<String>,
    pub sincronizando: bool,
    pub partes_pendientes: Vec<ParteGrupoPendienteDto>,
    pub cola: Vec<GastoEnColaDto>,
}

/// Una cuenta personal que se puede elegir para un grupo (activa y en su moneda).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuentaParaGrupoDto {
    pub id: String,
    pub nombre: String,
    pub moneda: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConsultaCuentasGrupoDto {
    pub grupo_id: String,
}

/// Las cuentas elegibles de un grupo y cuál está elegida hoy (si hay alguna).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuentasParaGrupoDto {
    pub grupo_id: String,
    pub cuenta_elegida: Option<String>,
    pub cuentas: Vec<CuentaParaGrupoDto>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ElegirCuentaGrupoDto {
    pub grupo_id: String,
    pub cuenta_id: String,
}
