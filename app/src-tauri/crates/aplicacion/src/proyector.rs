//! Proyección de eventos hacia los modelos de lectura.
//!
//! Traduce eventos del libro inmutable hacia las proyecciones materializadas
//! de cuentas, manteniendo los saldos derivados sincronizados con los movimientos
//! y ajustes registrados.

use std::collections::HashSet;
use std::str::FromStr;

use dominio::{Moneda, TipoMovimiento};
use rust_decimal::Decimal;

use crate::casos_de_uso::{
    TIPO_CUENTA_CREADA, TIPO_MOVIMIENTO_CORREGIDO, TIPO_MOVIMIENTO_REGISTRADO,
};
use crate::codec;
use crate::errores::ErrorAplicacion;
use crate::puertos::{CuentaProyectada, EventoRegistrado};

/// Traduce un evento `CuentaCreada` a la fila inicial que debe escribirse en la proyección `cuentas`.
///
/// Devuelve `Ok(None)` para eventos que no crean una cuenta nueva. Un evento `CuentaCreada`
/// con carga inválida devuelve error para evitar desincronizar la proyección.
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

/// Recalcula la proyección de todas las cuentas a partir de la secuencia completa de eventos,
/// aplicando ingresos, gastos y ajustes de forma idempotente (INV-05, INV-13).
///
/// Si un evento de movimiento o corrección tiene un JSON inválido (por ejemplo, en pruebas de contrato
/// con eventos sintéticos), se ignora sin abortar la proyección de las cuentas existentes.
pub fn proyectar_todas_las_cuentas(
    eventos: &[EventoRegistrado],
) -> Result<Vec<CuentaProyectada>, ErrorAplicacion> {
    let mut vistos = HashSet::new();
    let mut cuentas: Vec<CuentaProyectada> = Vec::new();
    let mut saldos: Vec<(Decimal, u32)> = Vec::new();
    let mut cuenta_de_movimiento: Vec<(String, usize)> = Vec::new();

    for e in eventos {
        if !vistos.insert(e.id.clone()) {
            continue;
        }
        match e.tipo.as_str() {
            TIPO_CUENTA_CREADA => {
                if let Some(c) = proyectar_cuenta(e)? {
                    let decs = Moneda::por_codigo(&c.moneda)
                        .map(|m| m.decimales)
                        .unwrap_or(2);
                    let inicial = Decimal::from_str(&c.saldo_actual).unwrap_or(Decimal::ZERO);
                    saldos.push((inicial, decs));
                    cuentas.push(c);
                }
            }
            TIPO_MOVIMIENTO_REGISTRADO => {
                if let Ok(mov) = codec::deserializar_movimiento(&e.datos)
                    && let Some(idx) = cuentas.iter().position(|c| c.id == mov.id_cuenta)
                {
                    match mov.tipo {
                        TipoMovimiento::Ingreso => saldos[idx].0 += mov.monto,
                        TipoMovimiento::Gasto => saldos[idx].0 -= mov.monto,
                    }
                    let decs = saldos[idx].1;
                    cuentas[idx].saldo_actual =
                        format!("{:.prec$}", saldos[idx].0, prec = decs as usize);
                    cuenta_de_movimiento.push((mov.id_movimiento, idx));
                }
            }
            TIPO_MOVIMIENTO_CORREGIDO => {
                if let Ok(corr) = codec::deserializar_correccion(&e.datos)
                    && let Some((_, idx)) = cuenta_de_movimiento
                        .iter()
                        .find(|(id_m, _)| *id_m == corr.id_movimiento_original)
                {
                    saldos[*idx].0 += corr.efecto;
                    let decs = saldos[*idx].1;
                    cuentas[*idx].saldo_actual =
                        format!("{:.prec$}", saldos[*idx].0, prec = decs as usize);
                }
            }
            _ => {}
        }
    }

    Ok(cuentas)
}
