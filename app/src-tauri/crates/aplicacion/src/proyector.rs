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
use crate::categorias::id_categoria_canonico;
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

/// Traduce un evento `CuentaCreada` a la fila inicial que debe escribirse en la proyección `cuentas`.
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
        id: id_categoria_canonico(&categoria.id),
        nombre: categoria.nombre,
        tipo: categoria.tipo,
        es_fabrica: categoria.es_fabrica,
        estado: "activa".to_string(),
    }))
}

/// Traduce un evento de movimiento (`MovimientoRegistrado` o `MovimientoCorregido`)
/// a la fila que debe escribirse en la proyección `movimientos`.
///
/// Para una corrección con datos completos de reemplazo, `reemplaza_a` apunta al
/// movimiento original. Si el evento es un ajuste de dominio que solo contiene
/// la diferencia (`id_ajuste`, `efecto`), devuelve `Ok(None)` para que el ajuste
/// sea procesado mediante `proyectar_correccion_ajuste`.
pub fn proyectar_movimiento(
    evento: &EventoRegistrado,
) -> Result<Option<MovimientoProyectado>, ErrorAplicacion> {
    if evento.tipo != TIPO_MOVIMIENTO_REGISTRADO && evento.tipo != TIPO_MOVIMIENTO_CORREGIDO {
        return Ok(None);
    }

    if evento.tipo == TIPO_MOVIMIENTO_CORREGIDO
        && let Ok(valor) = serde_json::from_str::<serde_json::Value>(&evento.datos)
        && valor.get("id_ajuste").is_some()
        && valor.get("cuenta_id").is_none()
    {
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
        categoria_id: id_categoria_canonico(&movimiento.categoria_id),
        nota: movimiento.nota,
        estado: "vigente".to_string(),
        reemplaza_a,
        corregido_por_evento: None,
    }))
}

///Construye un [`MovimientoProyectado`] a partir de un evento `MovimientoCorregido`
/// de dominio (que almacena `id_ajuste`, `id_movimiento_original` y `monto_correcto`)
/// tomando la cuenta y categoría del movimiento original.
pub fn proyectar_correccion_con_original(
    evento: &EventoRegistrado,
    movimientos_previos: &[MovimientoProyectado],
) -> Result<Option<MovimientoProyectado>, ErrorAplicacion> {
    if evento.tipo != TIPO_MOVIMIENTO_CORREGIDO {
        return Ok(None);
    }
    let valor: serde_json::Value = serde_json::from_str(&evento.datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;
    if valor.get("id_ajuste").is_none() || valor.get("cuenta_id").is_some() {
        return Ok(None);
    }

    let corr = codec::deserializar_correccion(&evento.datos)?;
    let Some(orig) = movimientos_previos
        .iter()
        .find(|m| m.id == corr.id_movimiento_original)
    else {
        return Ok(None);
    };

    let fecha = if evento.momento_registro.len() >= 10 {
        evento.momento_registro[..10].to_string()
    } else {
        orig.fecha.clone()
    };
    let monto_str = format!(
        "{:.prec$}",
        corr.monto_correcto,
        prec = corr.moneda.decimales as usize
    );

    Ok(Some(MovimientoProyectado {
        id: corr.id_ajuste,
        cuenta_id: orig.cuenta_id.clone(),
        tipo: orig.tipo.clone(),
        monto: monto_str,
        fecha,
        categoria_id: orig.categoria_id.clone(),
        nota: None,
        estado: "vigente".to_string(),
        reemplaza_a: Some(orig.id.clone()),
        corregido_por_evento: None,
    }))
}

/// Calcula el saldo actual de una cuenta a partir de su saldo inicial y sus
/// movimientos vigentes, usando aritmética decimal exacta (INV-03) y formateando
/// el resultado con los decimales de la moneda de la cuenta.
///
/// `movimientos_vigentes` es una lista de pares `(tipo, monto)` donde `tipo` es
/// `"ingreso"` (suma) o `"egreso"`/`"gasto"` (resta).
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
            "egreso" | "gasto" => total -= valor,
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

/// Variante con montos en centavos (INTEGER) para la nueva capa de infraestructura.
/// `movimientos_vigentes` es una lista de pares `(tipo, monto_cents)`.
pub fn calcular_saldo_actual_centavos(
    saldo_inicial_cents: i64,
    moneda: &str,
    movimientos_vigentes: &[(String, i64)],
) -> Result<i64, ErrorAplicacion> {
    let mut total: i64 = saldo_inicial_cents;

    for (tipo, monto_cents) in movimientos_vigentes {
        match tipo.as_str() {
            "ingreso" => total += *monto_cents,
            "egreso" | "gasto" => total -= *monto_cents,
            otro => {
                return Err(ErrorAplicacion::Interno(format!(
                    "tipo de movimiento desconocido: {otro}"
                )));
            }
        }
    }

    // Validación de moneda para mantener compatibilidad con la lógica existente
    let _ = Moneda::por_codigo(moneda)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda}")))?;

    Ok(total)
}
