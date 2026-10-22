//! Funciones de codificación y decodificación JSON para eventos de dominio.
//!
//! Aísla los detalles del formato de serialización (`serde_json`) dentro de la
//! capa de aplicación para mantener el dominio libre de dependencias de formato
//! y permitir que la capa de persistencia opere únicamente con cadenas de texto.

use dominio::{
    CategoriaCreada, CuentaCreada, Moneda, MovimientoCorregido, MovimientoRegistrado, TipoCuenta,
    TipoMovimiento,
};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::str::FromStr;

use crate::errores::ErrorAplicacion;

/// Convierte un valor de [`TipoCuenta`] a su identificador en texto `snake_case`.
pub fn tipo_a_texto(tipo: &TipoCuenta) -> &str {
    match tipo {
        TipoCuenta::Efectivo => "efectivo",
        TipoCuenta::Banco => "banco",
        TipoCuenta::BilleteraDigital => "billetera_digital",
    }
}

/// Convierte una cadena en `snake_case` a su variante correspondiente de [`TipoCuenta`].
pub fn texto_a_tipo(texto: &str) -> Result<TipoCuenta, ErrorAplicacion> {
    match texto.trim() {
        "efectivo" => Ok(TipoCuenta::Efectivo),
        "banco" => Ok(TipoCuenta::Banco),
        "billetera_digital" => Ok(TipoCuenta::BilleteraDigital),
        otro => Err(ErrorAplicacion::Interno(format!(
            "Tipo de cuenta desconocido: {otro}"
        ))),
    }
}

/// Convierte un valor de [`TipoMovimiento`] a su identificador en texto (`"ingreso"` o `"gasto"`).
pub fn tipo_movimiento_a_texto(tipo: &TipoMovimiento) -> &str {
    match tipo {
        TipoMovimiento::Ingreso => "ingreso",
        TipoMovimiento::Gasto => "gasto",
    }
}

/// Convierte una cadena de texto a su variante correspondiente de [`TipoMovimiento`].
pub fn texto_a_tipo_movimiento(texto: &str) -> Result<TipoMovimiento, ErrorAplicacion> {
    match texto.trim().to_lowercase().as_str() {
        "ingreso" => Ok(TipoMovimiento::Ingreso),
        "gasto" | "egreso" => Ok(TipoMovimiento::Gasto),
        otro => Err(ErrorAplicacion::Interno(format!(
            "Tipo de movimiento desconocido: {otro}"
        ))),
    }
}

/// Extrae una propiedad de tipo cadena desde un valor JSON, devolviendo un error interno si no existe.
fn texto(valor: &Value, clave: &str) -> Result<String, ErrorAplicacion> {
    valor
        .get(clave)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ErrorAplicacion::Interno(format!("falta el campo {clave} en el evento")))
}

/// Extrae una propiedad probando una clave principal o una alternativa.
fn texto_alternativo(
    valor: &Value,
    clave_1: &str,
    clave_2: &str,
) -> Result<String, ErrorAplicacion> {
    valor
        .get(clave_1)
        .or_else(|| valor.get(clave_2))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ErrorAplicacion::Interno(format!("falta el campo {clave_1} en el evento")))
}

/// Serializa un evento [`CuentaCreada`] en una cadena JSON con el saldo formateado
/// al número exacto de decimales de la moneda.
pub fn serializar(evento: &CuentaCreada) -> String {
    let saldo_formateado = format!(
        "{:.prec$}",
        evento.saldo_inicial,
        prec = evento.moneda.decimales as usize
    );
    json!({
        "id": evento.id,
        "nombre": evento.nombre,
        "tipo": tipo_a_texto(&evento.tipo),
        "moneda": evento.moneda.codigo,
        "saldo_inicial": saldo_formateado,
    })
    .to_string()
}

/// Deserializa una cadena JSON para reconstruir la estructura [`CuentaCreada`] original.
pub fn deserializar(datos: &str) -> Result<CuentaCreada, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let tipo_texto = texto(&valor, "tipo")?;
    let tipo = texto_a_tipo(&tipo_texto)?;

    let saldo_texto = texto(&valor, "saldo_inicial")?;
    let saldo = Decimal::from_str(&saldo_texto)
        .map_err(|e| ErrorAplicacion::Interno(format!("monto inválido: {e}")))?;

    let moneda_str = texto(&valor, "moneda")?;
    let moneda = Moneda::por_codigo(&moneda_str)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda_str}")))?;

    Ok(CuentaCreada {
        id: texto(&valor, "id")?,
        nombre: texto(&valor, "nombre")?,
        tipo,
        moneda,
        saldo_inicial: saldo,
    })
}

/// Contenido de un evento `CategoriaCreada`, ya validado para la proyección.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriaDatos {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub es_fabrica: bool,
}

/// Deserializa y valida la carga de un evento `CategoriaCreada` para la proyección.
pub fn deserializar_categoria(datos: &str) -> Result<CategoriaDatos, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let tipo_raw = texto(&valor, "tipo")?;
    let tipo = match tipo_raw.as_str() {
        "ingreso" => "ingreso".to_string(),
        "egreso" | "gasto" => "egreso".to_string(),
        otro => {
            return Err(ErrorAplicacion::Interno(format!(
                "tipo de categoría desconocido: {otro}"
            )));
        }
    };

    Ok(CategoriaDatos {
        id: texto_alternativo(&valor, "id", "id_categoria")?,
        nombre: texto(&valor, "nombre")?,
        tipo,
        es_fabrica: valor
            .get("es_fabrica")
            .or_else(|| valor.get("de_fabrica"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// Contenido de un evento de movimiento (`MovimientoRegistrado` o
/// `MovimientoCorregido`), ya validado para la proyección.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoDatos {
    pub id: String,
    pub cuenta_id: String,
    pub tipo: String,
    pub monto: String,
    pub fecha: String,
    pub categoria_id: String,
    pub nota: Option<String>,
    /// Presente en `MovimientoCorregido`: el movimiento original que se corrige.
    pub movimiento_original_id: Option<String>,
}

/// Deserializa y valida la carga de un evento de movimiento para la proyección.
pub fn deserializar_movimiento(datos: &str) -> Result<MovimientoDatos, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let tipo_raw = texto(&valor, "tipo")?;
    let tipo = match tipo_raw.as_str() {
        "ingreso" => "ingreso".to_string(),
        "egreso" | "gasto" => "egreso".to_string(),
        otro => {
            return Err(ErrorAplicacion::Interno(format!(
                "tipo de movimiento desconocido: {otro}"
            )));
        }
    };

    let monto = texto_alternativo(&valor, "monto", "monto_correcto")?;
    let decimal = Decimal::from_str(&monto)
        .map_err(|e| ErrorAplicacion::Interno(format!("monto inválido: {e}")))?;

    let orig_id = valor
        .get("movimiento_original_id")
        .or_else(|| valor.get("id_movimiento_original"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if orig_id.is_none() && decimal <= Decimal::new(0, 0) {
        return Err(ErrorAplicacion::Interno(
            "el monto del movimiento debe ser positivo".to_string(),
        ));
    }

    Ok(MovimientoDatos {
        id: texto_alternativo(&valor, "id", "id_movimiento")
            .or_else(|_| texto(&valor, "id_ajuste"))?,
        cuenta_id: texto_alternativo(&valor, "cuenta_id", "id_cuenta")?,
        tipo,
        monto,
        fecha: texto(&valor, "fecha")?,
        categoria_id: texto_alternativo(&valor, "categoria_id", "id_categoria")?,
        nota: valor
            .get("nota")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        movimiento_original_id: orig_id,
    })
}

/// Serializa un evento [`MovimientoRegistrado`] en una cadena JSON compatible con
/// el dominio y con las proyecciones de almacenamiento.
pub fn serializar_movimiento(evento: &MovimientoRegistrado) -> String {
    let monto_formateado = format!(
        "{:.prec$}",
        evento.monto,
        prec = evento.moneda.decimales as usize
    );
    let tipo_dom = tipo_movimiento_a_texto(&evento.tipo);
    let tipo_proy = match evento.tipo {
        TipoMovimiento::Ingreso => "ingreso",
        TipoMovimiento::Gasto => "egreso",
    };
    let mut obj = serde_json::Map::new();
    obj.insert("id".into(), Value::String(evento.id_movimiento.clone()));
    obj.insert(
        "id_movimiento".into(),
        Value::String(evento.id_movimiento.clone()),
    );
    obj.insert("cuenta_id".into(), Value::String(evento.id_cuenta.clone()));
    obj.insert("id_cuenta".into(), Value::String(evento.id_cuenta.clone()));
    obj.insert("tipo".into(), Value::String(tipo_proy.into()));
    obj.insert("tipo_dominio".into(), Value::String(tipo_dom.into()));
    obj.insert("monto".into(), Value::String(monto_formateado));
    obj.insert("moneda".into(), Value::String(evento.moneda.codigo.clone()));
    obj.insert("fecha".into(), Value::String(evento.fecha.clone()));
    obj.insert(
        "categoria_id".into(),
        Value::String(evento.id_categoria.clone()),
    );
    obj.insert(
        "id_categoria".into(),
        Value::String(evento.id_categoria.clone()),
    );
    if let Some(ref n) = evento.nota {
        obj.insert("nota".into(), Value::String(n.clone()));
    }
    Value::Object(obj).to_string()
}

/// Deserializa una cadena JSON para reconstruir un [`MovimientoRegistrado`] de dominio.
pub fn deserializar_evento_movimiento(
    datos: &str,
) -> Result<MovimientoRegistrado, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let tipo_str = valor
        .get("tipo_dominio")
        .or_else(|| valor.get("tipo"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorAplicacion::Interno("falta el campo tipo en el evento".to_string()))?;
    let tipo = texto_a_tipo_movimiento(tipo_str)?;

    let moneda_str = valor
        .get("moneda")
        .and_then(|v| v.as_str())
        .unwrap_or("USD");
    let moneda = Moneda::por_codigo(moneda_str)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda_str}")))?;

    let monto_str = texto(&valor, "monto")?;
    let mut monto = Decimal::from_str(&monto_str)
        .map_err(|e| ErrorAplicacion::Interno(format!("monto inválido: {e}")))?;
    monto.rescale(moneda.decimales);

    let nota = valor
        .get("nota")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(MovimientoRegistrado {
        id_movimiento: texto_alternativo(&valor, "id_movimiento", "id")?,
        id_cuenta: texto_alternativo(&valor, "id_cuenta", "cuenta_id")?,
        tipo,
        monto,
        moneda,
        fecha: texto(&valor, "fecha")?,
        id_categoria: texto_alternativo(&valor, "id_categoria", "categoria_id")?,
        nota,
    })
}

/// Serializa un evento [`MovimientoCorregido`] en una cadena JSON.
pub fn serializar_correccion(evento: &MovimientoCorregido) -> String {
    let prec = evento.moneda.decimales as usize;
    let monto_correcto = format!("{:.prec$}", evento.monto_correcto, prec = prec);
    let efecto = format!("{:.prec$}", evento.efecto, prec = prec);
    json!({
        "id_ajuste": evento.id_ajuste,
        "id_movimiento_original": evento.id_movimiento_original,
        "monto_correcto": monto_correcto,
        "efecto": efecto,
        "moneda": evento.moneda.codigo,
    })
    .to_string()
}

/// Deserializa una cadena JSON para reconstruir un [`MovimientoCorregido`] de dominio.
pub fn deserializar_correccion(datos: &str) -> Result<MovimientoCorregido, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let moneda_str = valor
        .get("moneda")
        .and_then(|v| v.as_str())
        .unwrap_or("USD");
    let moneda = Moneda::por_codigo(moneda_str)
        .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda_str}")))?;

    let monto_str = texto_alternativo(&valor, "monto_correcto", "monto")?;
    let mut monto_correcto = Decimal::from_str(&monto_str)
        .map_err(|e| ErrorAplicacion::Interno(format!("monto_correcto inválido: {e}")))?;
    monto_correcto.rescale(moneda.decimales);

    let efecto_str = valor
        .get("efecto")
        .and_then(|v| v.as_str())
        .unwrap_or("0.00");
    let mut efecto = Decimal::from_str(efecto_str.trim().trim_start_matches('+'))
        .map_err(|e| ErrorAplicacion::Interno(format!("efecto inválido: {e}")))?;
    efecto.rescale(moneda.decimales);

    Ok(MovimientoCorregido {
        id_ajuste: texto_alternativo(&valor, "id_ajuste", "id")?,
        id_movimiento_original: texto_alternativo(
            &valor,
            "id_movimiento_original",
            "movimiento_original_id",
        )?,
        monto_correcto,
        efecto,
        moneda,
    })
}

/// Serializa un evento [`CategoriaCreada`] en una cadena JSON compatible con el
/// dominio y con la proyección de almacenamiento.
pub fn serializar_categoria(evento: &CategoriaCreada) -> String {
    let tipo_proy = match evento.tipo {
        TipoMovimiento::Ingreso => "ingreso",
        TipoMovimiento::Gasto => "egreso",
    };
    json!({
        "id": evento.id_categoria,
        "id_categoria": evento.id_categoria,
        "nombre": evento.nombre,
        "tipo": tipo_proy,
        "es_fabrica": false,
    })
    .to_string()
}

/// Deserializa una cadena JSON para reconstruir una [`CategoriaCreada`] de dominio.
pub fn deserializar_evento_categoria(datos: &str) -> Result<CategoriaCreada, ErrorAplicacion> {
    let valor: Value = serde_json::from_str(datos)
        .map_err(|e| ErrorAplicacion::Interno(format!("evento con JSON inválido: {e}")))?;

    let tipo = texto_a_tipo_movimiento(&texto(&valor, "tipo")?)?;
    Ok(CategoriaCreada {
        id_categoria: texto_alternativo(&valor, "id_categoria", "id")?,
        nombre: texto(&valor, "nombre")?,
        tipo,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use rust_decimal_macros::dec;

    fn evento_ejemplo() -> CuentaCreada {
        CuentaCreada {
            id: "id-1".to_string(),
            nombre: "Cartera".to_string(),
            tipo: TipoCuenta::Efectivo,
            moneda: Moneda::por_codigo("VES").unwrap(),
            saldo_inicial: dec!(1250.50),
        }
    }

    #[test]
    fn ida_y_vuelta_conserva_el_evento() {
        let original = evento_ejemplo();
        let recuperado = deserializar(&serializar(&original)).unwrap();
        assert_eq!(recuperado, original);
    }

    #[test]
    fn el_json_lleva_el_monto_formateado_como_texto() {
        let json = serializar(&evento_ejemplo());
        assert!(json.contains("\"saldo_inicial\":\"1250.50\""));
        assert!(json.contains("\"moneda\":\"VES\""));
    }

    #[test]
    fn ida_y_vuelta_conserva_movimiento_correccion_y_categoria() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let mov = MovimientoRegistrado {
            id_movimiento: "m-1".to_string(),
            id_cuenta: "c-1".to_string(),
            tipo: TipoMovimiento::Gasto,
            monto: dec!(50.00),
            moneda: usd.clone(),
            fecha: "2026-10-19".to_string(),
            id_categoria: "gasto:alimentacion".to_string(),
            nota: Some("Cena".to_string()),
        };
        let json_mov = serializar_movimiento(&mov);
        assert_eq!(deserializar_evento_movimiento(&json_mov).unwrap(), mov);
        assert_eq!(deserializar_movimiento(&json_mov).unwrap().id, "m-1");

        let corr = MovimientoCorregido {
            id_ajuste: "a-1".to_string(),
            id_movimiento_original: "m-1".to_string(),
            monto_correcto: dec!(40.00),
            efecto: dec!(10.00),
            moneda: usd,
        };
        assert_eq!(
            deserializar_correccion(&serializar_correccion(&corr)).unwrap(),
            corr
        );

        let cat = CategoriaCreada {
            id_categoria: "cat-1".to_string(),
            nombre: "Mascotas".to_string(),
            tipo: TipoMovimiento::Gasto,
        };
        let json_cat = serializar_categoria(&cat);
        assert_eq!(deserializar_evento_categoria(&json_cat).unwrap(), cat);
        assert_eq!(deserializar_categoria(&json_cat).unwrap().id, "cat-1");
    }

    #[test]
    fn errores_de_deserializacion_son_internos() {
        assert!(matches!(
            deserializar("no es json"),
            Err(ErrorAplicacion::Interno(_))
        ));
        assert!(matches!(
            deserializar("{}"),
            Err(ErrorAplicacion::Interno(_))
        ));
        let tipo_malo =
            r#"{"id":"1","nombre":"A","tipo":"tarjeta","moneda":"VES","saldo_inicial":"1.00"}"#;
        assert!(matches!(
            deserializar(tipo_malo),
            Err(ErrorAplicacion::Interno(_))
        ));
        let monto_malo =
            r#"{"id":"1","nombre":"A","tipo":"banco","moneda":"VES","saldo_inicial":"abc"}"#;
        assert!(matches!(
            deserializar(monto_malo),
            Err(ErrorAplicacion::Interno(_))
        ));
        let moneda_mala =
            r#"{"id":"1","nombre":"A","tipo":"banco","moneda":"XYZ","saldo_inicial":"1.00"}"#;
        assert!(matches!(
            deserializar(moneda_mala),
            Err(ErrorAplicacion::Interno(_))
        ));
        assert!(matches!(
            deserializar_movimiento("no es json"),
            Err(ErrorAplicacion::Interno(_))
        ));
        assert!(matches!(
            deserializar_correccion("no es json"),
            Err(ErrorAplicacion::Interno(_))
        ));
        assert!(matches!(
            deserializar_categoria("no es json"),
            Err(ErrorAplicacion::Interno(_))
        ));
    }
}
