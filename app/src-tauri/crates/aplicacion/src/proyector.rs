//! Proyección de eventos hacia el modelo de lectura (`cuentas`, `categorias` y
//! `movimientos`) y cálculo del saldo actual.
//!
//! Concentra la única regla que traduce un evento del libro inmutable a una fila
//! de proyección. Al vivir en la capa de aplicación, tanto el adaptador en
//! memoria como el de SQLite comparten exactamente la misma lógica.

use dominio::Moneda;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::casos_de_uso::TIPO_CUENTA_CREADA;
use crate::codec;
use crate::errores::ErrorAplicacion;
use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, EventoRegistrado, MovimientoProyectado,
};

/// Identificador de tipo de los eventos de categoría.
pub const TIPO_CATEGORIA_CREADA: &str = "CategoriaCreada";
/// Identificador de tipo del evento que registra un movimiento.
pub const TIPO_MOVIMIENTO_REGISTRADO: &str = "MovimientoRegistrado";
/// Identificador de tipo del evento que corrige un movimiento existente.
pub const TIPO_MOVIMIENTO_CORREGIDO: &str = "MovimientoCorregido";

/// Traduce un evento a la fila que debe escribirse en la proyección `cuentas`.
///
/// Devuelve `Ok(None)` para eventos que no afectan a esta proyección. Un evento
/// `CuentaCreada` con carga inválida es un error: la proyección no puede quedar
/// desincronizada.
///
/// La cuenta nace con `saldo_actual == saldo_inicial`; los movimientos posteriores
/// ajustan `saldo_actual` sin tocar `saldo_inicial` (D-11).
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
        saldo_inicial: saldo.clone(),
        saldo_actual: saldo,
        estado: "activa".to_string(),
    }))
}

/// Traduce un evento `CategoriaCreada` a la fila de la proyección `categorias`.
///
/// Devuelve `Ok(None)` para cualquier otro tipo de evento.
pub fn proyectar_categoria(
    evento: &EventoRegistrado,
) -> Result<Option<CategoriaProyectada>, ErrorAplicacion> {
    if evento.tipo != TIPO_CATEGORIA_CREADA {
        return Ok(None);
    }
    let categoria = codec::deserializar_categoria(&evento.datos)?;
    Ok(Some(CategoriaProyectada {
        id: categoria.id,
        nombre: categoria.nombre,
        tipo: categoria.tipo,
        es_fabrica: categoria.es_fabrica,
        estado: "activa".to_string(),
    }))
}

/// Traduce un evento de movimiento (`MovimientoRegistrado` o `MovimientoCorregido`)
/// a la fila que debe escribirse en la proyección `movimientos`.
///
/// Para una corrección, `reemplaza_a` apunta al movimiento original; marcar ese
/// original como `corregido` es responsabilidad del repositorio, que registra
/// también el evento de la corrección.
pub fn proyectar_movimiento(
    evento: &EventoRegistrado,
) -> Result<Option<MovimientoProyectado>, ErrorAplicacion> {
    if evento.tipo != TIPO_MOVIMIENTO_REGISTRADO && evento.tipo != TIPO_MOVIMIENTO_CORREGIDO {
        return Ok(None);
    }
    let movimiento = codec::deserializar_movimiento(&evento.datos)?;

    let reemplaza_a = if evento.tipo == TIPO_MOVIMIENTO_CORREGIDO {
        match movimiento.movimiento_original_id {
            Some(original) => Some(original),
            None => {
                return Err(ErrorAplicacion::Interno(
                    "MovimientoCorregido sin movimiento_original_id".to_string(),
                ));
            }
        }
    } else {
        None
    };

    Ok(Some(MovimientoProyectado {
        id: movimiento.id,
        cuenta_id: movimiento.cuenta_id,
        tipo: movimiento.tipo,
        monto: movimiento.monto,
        fecha: movimiento.fecha,
        categoria_id: movimiento.categoria_id,
        nota: movimiento.nota,
        estado: "vigente".to_string(),
        reemplaza_a,
        corregido_por_evento: None,
    }))
}

/// Calcula el saldo actual de una cuenta a partir de su saldo inicial y sus
/// movimientos vigentes, usando aritmética decimal exacta (INV-03) y formateando
/// el resultado con los decimales de la moneda de la cuenta.
///
/// `movimientos_vigentes` es una lista de pares `(tipo, monto)` donde `tipo` es
/// `"ingreso"` (suma) o `"egreso"` (resta).
pub fn calcular_saldo_actual(
    saldo_inicial: &str,
    moneda: &str,
    movimientos_vigentes: &[(String, String)],
) -> Result<String, ErrorAplicacion> {
    let mut total = Decimal::from_str(saldo_inicial)
        .map_err(|e| ErrorAplicacion::Interno(format!("saldo inicial inválido: {e}")))?;

    for (tipo, monto) in movimientos_vigentes {
        let valor = Decimal::from_str(monto)
            .map_err(|e| ErrorAplicacion::Interno(format!("monto inválido: {e}")))?;
        match tipo.as_str() {
            "ingreso" => total += valor,
            "egreso" => total -= valor,
            otro => {
                return Err(ErrorAplicacion::Interno(format!(
                    "tipo de movimiento desconocido: {otro}"
                )));
            }
        }
    }

    let decimales = Moneda::por_codigo(moneda)
        .map(|m| m.decimales)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda}")))?;
    Ok(format!("{:.prec$}", total, prec = decimales as usize))
}
