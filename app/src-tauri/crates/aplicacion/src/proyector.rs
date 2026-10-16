//! Proyección de eventos hacia el modelo de lectura `cuentas`.
//!
//! Concentra la única regla que traduce un evento del libro inmutable a una fila
//! de la proyección. Al vivir en la capa de aplicación, tanto el adaptador en
//! memoria como el de SQLite comparten exactamente la misma lógica.

use crate::casos_de_uso::TIPO_CUENTA_CREADA;
use crate::codec;
use crate::errores::ErrorAplicacion;
use crate::puertos::{CuentaProyectada, EventoRegistrado};

/// Traduce un evento a la fila que debe escribirse en la proyección `cuentas`.
///
/// Devuelve `Ok(None)` para eventos que no afectan a esta proyección (por
/// ejemplo, tipos desconocidos para esta entrega). Un evento `CuentaCreada`
/// con carga inválida es un error: la proyección no puede quedar desincronizada.
pub fn proyectar_cuenta(
    evento: &EventoRegistrado,
) -> Result<Option<CuentaProyectada>, ErrorAplicacion> {
    if evento.tipo != TIPO_CUENTA_CREADA {
        return Ok(None);
    }
    let cuenta = codec::deserializar(&evento.datos)?;
    let saldo = format!(
        "{:.prec$}",
        cuenta.saldo_inicial,
        prec = cuenta.moneda.decimales as usize
    );
    Ok(Some(CuentaProyectada {
        id: cuenta.id,
        nombre: cuenta.nombre,
        tipo: codec::tipo_a_texto(&cuenta.tipo).to_string(),
        moneda: cuenta.moneda.codigo,
        saldo_actual: saldo,
        estado: "activa".to_string(),
    }))
}
