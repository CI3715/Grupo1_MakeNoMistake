//! Conversión exacta entre montos decimales y enteros en **unidades mínimas**.
//!
//! Un monto se guarda como `cantidad × 10^d`, donde `d` son los decimales de su
//! moneda: 12,34 USD (`d = 2`) → `1234`; 0,00012345 BTC (`d = 8`) → `12345`;
//! 1.500 de una moneda sin decimales (`d = 0`) → `1500`. Como los decimales de
//! una moneda no cambian y una cuenta no cambia de moneda (INV-18), todos los
//! montos de una cuenta comparten la misma escala y se suman como enteros sin
//! perder nada. Las cuatro monedas de fábrica tienen `d = 2`, así que una base
//! guardada «en centavos» ya está en unidades mínimas.
//!
//! Las tasas de cambio no son montos: se guardan siempre con escala fija
//! [`ESCALA_TASA`] (`valor × 10^8`), porque admiten hasta 8 decimales sin
//! importar las monedas del par.
//!
//! Ninguna conversión redondea en silencio: un valor con más decimales de los
//! que admite su escala, o que no cabe en un `i64`, es un error (INV-03).

use rust_decimal::Decimal;
use std::str::FromStr;

use crate::errores::ErrorAplicacion;

/// Decimales con los que se guarda el valor de una tasa de cambio.
pub const ESCALA_TASA: u32 = dominio::DECIMALES_MAXIMOS;

/// Convierte un decimal a unidades mínimas de una moneda con `decimales` decimales.
pub fn decimal_a_unidades(valor: Decimal, decimales: u32) -> Result<i64, ErrorAplicacion> {
    if valor.normalize().scale() > decimales {
        return Err(ErrorAplicacion::Interno(format!(
            "el monto {valor} tiene más de {decimales} decimales"
        )));
    }
    let mut escalado = valor;
    escalado.rescale(decimales);
    if escalado.scale() != decimales {
        return Err(ErrorAplicacion::Interno(format!(
            "el monto {valor} es demasiado grande para guardarse"
        )));
    }
    i64::try_from(escalado.mantissa()).map_err(|_| {
        ErrorAplicacion::Interno(format!(
            "el monto {valor} es demasiado grande para guardarse"
        ))
    })
}

/// Convierte un monto en texto decimal (`"12.34"`) a unidades mínimas.
pub fn a_unidades(texto: &str, decimales: u32) -> Result<i64, ErrorAplicacion> {
    let valor = Decimal::from_str(texto.trim())
        .map_err(|e| ErrorAplicacion::Interno(format!("monto inválido '{texto}': {e}")))?;
    decimal_a_unidades(valor, decimales)
}

/// Convierte unidades mínimas a un decimal con la escala de la moneda.
pub fn unidades_a_decimal(unidades: i64, decimales: u32) -> Decimal {
    Decimal::new(unidades, decimales)
}

/// Convierte unidades mínimas al texto con exactamente `decimales` decimales
/// (`1234`, 2 → `"12.34"`; `1500`, 0 → `"1500"`).
pub fn desde_unidades(unidades: i64, decimales: u32) -> String {
    format!(
        "{:.prec$}",
        unidades_a_decimal(unidades, decimales),
        prec = decimales as usize
    )
}

/// Texto canónico del valor de una tasa: sin ceros finales (`"887"`, `"0.0011274"`).
///
/// Ambos adaptadores proyectan este mismo texto, guarden el valor como texto o
/// como entero.
pub fn texto_tasa(valor: Decimal) -> String {
    valor.normalize().to_string()
}

/// Convierte el valor de una tasa a entero con escala [`ESCALA_TASA`].
pub fn tasa_a_entero(texto: &str) -> Result<i64, ErrorAplicacion> {
    a_unidades(texto, ESCALA_TASA)
}

/// Convierte un entero con escala [`ESCALA_TASA`] al texto canónico de la tasa.
pub fn tasa_desde_entero(entero: i64) -> String {
    texto_tasa(unidades_a_decimal(entero, ESCALA_TASA))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_escala_depende_de_los_decimales_de_la_moneda() {
        assert_eq!(a_unidades("12.34", 2).unwrap(), 1234);
        assert_eq!(a_unidades("12.3", 2).unwrap(), 1230);
        assert_eq!(a_unidades("0.00012345", 8).unwrap(), 12345);
        assert_eq!(a_unidades("1500", 0).unwrap(), 1500);
        assert_eq!(a_unidades("-5.5", 1).unwrap(), -55);
    }

    #[test]
    fn ida_y_vuelta_conserva_el_monto_formateado() {
        for (texto, decimales) in [
            ("1000.00", 2),
            ("0.00012345", 8),
            ("1500", 0),
            ("-12.340", 3),
            ("0.00", 2),
        ] {
            let unidades = a_unidades(texto, decimales).unwrap();
            assert_eq!(desde_unidades(unidades, decimales), texto);
        }
    }

    #[test]
    fn nunca_redondea_en_silencio() {
        assert!(a_unidades("12.345", 2).is_err());
        assert!(a_unidades("0.5", 0).is_err());
        assert!(a_unidades("0.000000001", 8).is_err());
        // Los ceros finales no cuentan como decimales.
        assert_eq!(a_unidades("12.3400", 2).unwrap(), 1234);
    }

    #[test]
    fn un_monto_que_no_cabe_en_i64_es_un_error() {
        assert_eq!(
            a_unidades("92233720368.54775807", 8).unwrap(),
            i64::MAX,
            "el máximo con 8 decimales"
        );
        assert!(a_unidades("92233720368.54775808", 8).is_err());
        assert!(a_unidades("99999999999999999999", 2).is_err());
    }

    #[test]
    fn las_tasas_usan_escala_fija_de_ocho_decimales() {
        assert_eq!(tasa_a_entero("887.00").unwrap(), 88_700_000_000);
        assert_eq!(tasa_a_entero("0.00112740").unwrap(), 112_740);
        assert_eq!(tasa_desde_entero(88_700_000_000), "887");
        assert_eq!(tasa_desde_entero(112_740), "0.0011274");
        assert!(tasa_a_entero("1.123456789").is_err());
    }
}
