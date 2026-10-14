//! Funciones de codificación y decodificación JSON para eventos de dominio.
//!
//! Aísla los detalles del formato de serialización (`serde_json`) dentro de la
//! capa de aplicación para mantener el dominio libre de dependencias de formato
//! y permitir que la capa de persistencia opere únicamente con cadenas de texto.

use dominio::{CuentaCreada, Moneda, TipoCuenta};
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
    match texto {
        "efectivo" => Ok(TipoCuenta::Efectivo),
        "banco" => Ok(TipoCuenta::Banco),
        "billetera_digital" => Ok(TipoCuenta::BilleteraDigital),
        _ => Err(ErrorAplicacion::Interno(format!(
            "Tipo de cuenta desconocido: {texto}"
        ))),
    }
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

/// Extrae una propiedad de tipo cadena desde un valor JSON, devolviendo un error interno si no existe.
fn texto(valor: &Value, clave: &str) -> Result<String, ErrorAplicacion> {
    valor
        .get(clave)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ErrorAplicacion::Interno(format!("falta el campo {clave} en el evento")))
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
    }
}
