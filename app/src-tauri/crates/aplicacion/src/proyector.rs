//! ProyecciÃ³n de eventos hacia el modelo de lectura (`cuentas`, `categorias`,
//! `movimientos`, `monedas_propias`, `tasas` y moneda base) y cÃ¡lculo del saldo
//! actual.
//!
//! Concentra la Ãºnica regla que traduce un evento del libro inmutable a una fila
//! de proyecciÃ³n. Al vivir en la capa de aplicaciÃ³n, tanto el adaptador en
//! memoria como el de SQLite comparten exactamente la misma lÃ³gica.

use dominio::Moneda;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::casos_de_uso::TIPO_CUENTA_CREADA;
use crate::categorias::id_categoria_canonico;
use crate::codec;
use crate::errores::ErrorAplicacion;
use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, EventoRegistrado, MonedaProyectada,
    MovimientoProyectado, TasaProyectada,
};

/// Identificador de tipo de los eventos de categorÃ­a.
pub const TIPO_CATEGORIA_CREADA: &str = "CategoriaCreada";
/// Identificador de tipo del evento que registra un movimiento.
pub const TIPO_MOVIMIENTO_REGISTRADO: &str = "MovimientoRegistrado";
/// Identificador de tipo del evento que corrige un movimiento existente.
pub const TIPO_MOVIMIENTO_CORREGIDO: &str = "MovimientoCorregido";
/// Identificador de tipo del evento que registra una tasa de cambio.
pub const TIPO_TASA_REGISTRADA: &str = "TasaRegistrada";
/// Identificador de tipo del evento que elimina una tasa registrada por error.
pub const TIPO_TASA_ELIMINADA: &str = "TasaEliminada";
/// Identificador de tipo del evento que cambia la moneda base.
pub const TIPO_MONEDA_BASE_CAMBIADA: &str = "MonedaBaseCambiada";
/// Identificador de tipo del evento que crea una moneda propia.
pub const TIPO_MONEDA_CREADA: &str = "MonedaCreada";

/// CatÃ¡logo completo de monedas: las cuatro de fÃ¡brica mÃ¡s las propias proyectadas.
pub fn catalogo_completo(propias: &[MonedaProyectada]) -> Vec<Moneda> {
    let mut catalogo = Moneda::catalogo();
    catalogo.extend(
        propias
            .iter()
            .map(|m| Moneda::nueva_propia(&m.codigo, &m.nombre, &m.simbolo, m.decimales)),
    );
    catalogo
}

/// Decimales de una moneda del catÃ¡logo, o error si no existe.
pub fn decimales_de(moneda: &str, catalogo: &[Moneda]) -> Result<u32, ErrorAplicacion> {
    Moneda::por_codigo_en(moneda, catalogo)
        .map(|m| m.decimales)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda}")))
}

/// Traduce un evento `CuentaCreada` a la fila inicial que debe escribirse en la proyecciÃ³n `cuentas`.
///
/// Devuelve `Ok(None)` para eventos que no afectan a esta proyecciÃ³n. Un evento
/// `CuentaCreada` con carga invÃ¡lida es un error: la proyecciÃ³n no puede quedar
/// desincronizada.
///
/// La cuenta nace con `saldo_actual == saldo_inicial`; los movimientos posteriores
/// ajustan `saldo_actual` sin tocar `saldo_inicial` (D-11).
///
/// `catalogo` son las monedas conocidas al proyectar el evento: la cuenta puede
/// usar una moneda propia creada antes que ella.
pub fn proyectar_cuenta(
    evento: &EventoRegistrado,
    catalogo: &[Moneda],
) -> Result<Option<CuentaProyectada>, ErrorAplicacion> {
    if evento.tipo != TIPO_CUENTA_CREADA {
        return Ok(None);
    }
    let cuenta = codec::deserializar(&evento.datos, catalogo)?;
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

/// Traduce un evento `CategoriaCreada` a la fila de la proyecciÃ³n `categorias`.
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
/// a la fila que debe escribirse en la proyecciÃ³n `movimientos`.
///
/// Para una correcciÃ³n con datos completos de reemplazo, `reemplaza_a` apunta al
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
/// tomando la cuenta y categorÃ­a del movimiento original.
pub fn proyectar_correccion_con_original(
    evento: &EventoRegistrado,
    movimientos_previos: &[MovimientoProyectado],
    catalogo: &[Moneda],
) -> Result<Option<MovimientoProyectado>, ErrorAplicacion> {
    if evento.tipo != TIPO_MOVIMIENTO_CORREGIDO {
        return Ok(None);
    }
    let valor: serde_json::Value = serde_json::from_str(&evento.datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON invÃ¡lido: {e}")))?;
    if valor.get("id_ajuste").is_none() || valor.get("cuenta_id").is_some() {
        return Ok(None);
    }

    let corr = codec::deserializar_correccion(&evento.datos, catalogo)?;
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
/// movimientos vigentes, usando aritmÃ©tica decimal exacta (INV-03) y formateando
/// el resultado con los decimales de la moneda de la cuenta.
///
/// `movimientos_vigentes` es una lista de pares `(tipo, monto)` donde `tipo` es
/// `"ingreso"` (suma) o `"egreso"`/`"gasto"` (resta).
pub fn calcular_saldo_actual(
    saldo_inicial: &str,
    decimales: u32,
    movimientos_vigentes: &[(String, String)],
) -> Result<String, ErrorAplicacion> {
    let mut total = Decimal::from_str(saldo_inicial)
        .map_err(|e| ErrorAplicacion::Interno(format!("saldo inicial invÃ¡lido: {e}")))?;

    for (tipo, monto) in movimientos_vigentes {
        let valor = Decimal::from_str(monto)
            .map_err(|e| ErrorAplicacion::Interno(format!("monto invÃ¡lido: {e}")))?;
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

    Ok(format!("{:.prec$}", total, prec = decimales as usize))
}

/// Variante con montos en **unidades mÃ­nimas** (enteros con la escala de la
/// moneda de la cuenta), para el adaptador de SQLite.
///
/// Todos los montos de una cuenta estÃ¡n en la misma moneda y, por lo tanto, en
/// la misma escala: la suma entera es exacta. Un desbordamiento es un error, no
/// un resultado incorrecto.
pub fn calcular_saldo_actual_unidades(
    saldo_inicial: i64,
    movimientos_vigentes: &[(String, i64)],
) -> Result<i64, ErrorAplicacion> {
    let mut total = saldo_inicial;
    for (tipo, monto) in movimientos_vigentes {
        let siguiente = match tipo.as_str() {
            "ingreso" => total.checked_add(*monto),
            "egreso" | "gasto" => total.checked_sub(*monto),
            otro => {
                return Err(ErrorAplicacion::Interno(format!(
                    "tipo de movimiento desconocido: {otro}"
                )));
            }
        };
        total = siguiente.ok_or_else(|| {
            ErrorAplicacion::Interno("el saldo es demasiado grande para guardarse".to_string())
        })?;
    }
    Ok(total)
}

/// Traduce un evento `MonedaCreada` a la fila de la proyecciÃ³n `monedas_propias`.
///
/// Rechaza un cÃ³digo de fÃ¡brica: esas monedas no se pueden redefinir. La
/// unicidad frente a otras monedas propias la comprueba cada adaptador.
pub fn proyectar_moneda(
    evento: &EventoRegistrado,
) -> Result<Option<MonedaProyectada>, ErrorAplicacion> {
    if evento.tipo != TIPO_MONEDA_CREADA {
        return Ok(None);
    }
    let moneda = codec::deserializar_moneda_creada(&evento.datos)?;
    if Moneda::por_codigo(&moneda.codigo).is_some() {
        return Err(ErrorAplicacion::Interno(format!(
            "{} es una moneda de fÃ¡brica y no se puede redefinir",
            moneda.codigo
        )));
    }
    Ok(Some(MonedaProyectada {
        codigo: moneda.codigo,
        nombre: moneda.nombre,
        simbolo: moneda.simbolo,
        decimales: moneda.decimales,
    }))
}

/// Traduce un evento `TasaRegistrada` a la fila del historial de tasas.
///
/// Las dos monedas del par deben existir en `catalogo`.
pub fn proyectar_tasa(
    evento: &EventoRegistrado,
    catalogo: &[Moneda],
) -> Result<Option<TasaProyectada>, ErrorAplicacion> {
    if evento.tipo != TIPO_TASA_REGISTRADA {
        return Ok(None);
    }
    let tasa = codec::deserializar_tasa_registrada(&evento.datos)?;
    decimales_de(&tasa.moneda_origen, catalogo)?;
    decimales_de(&tasa.moneda_destino, catalogo)?;
    Ok(Some(TasaProyectada {
        id: tasa.id,
        moneda_origen: tasa.moneda_origen,
        moneda_destino: tasa.moneda_destino,
        valor: crate::unidades::texto_tasa(tasa.valor),
        fecha: tasa.fecha,
        fuente: tasa.fuente,
        estado: "vigente".to_string(),
    }))
}

/// Identificador de la tasa que elimina un evento `TasaEliminada`.
///
/// El adaptador marca esa tasa como `"eliminada"`; si no existe o ya estaba
/// eliminada, el evento se rechaza.
pub fn tasa_eliminada(evento: &EventoRegistrado) -> Result<Option<String>, ErrorAplicacion> {
    if evento.tipo != TIPO_TASA_ELIMINADA {
        return Ok(None);
    }
    codec::deserializar_tasa_eliminada(&evento.datos).map(Some)
}

/// Nueva moneda base que fija un evento `MonedaBaseCambiada`; debe estar en `catalogo`.
pub fn moneda_base_cambiada(
    evento: &EventoRegistrado,
    catalogo: &[Moneda],
) -> Result<Option<String>, ErrorAplicacion> {
    if evento.tipo != TIPO_MONEDA_BASE_CAMBIADA {
        return Ok(None);
    }
    let moneda = codec::deserializar_moneda_base_cambiada(&evento.datos)?;
    decimales_de(&moneda, catalogo)?;
    Ok(Some(moneda))
}
