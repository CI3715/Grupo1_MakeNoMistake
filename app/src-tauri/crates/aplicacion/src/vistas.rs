//! Modelos de transferencia y lectura (DTOs) devueltos por los casos de uso.
//!
//! Los valores monetarios se representan como cadenas de texto formateadas con
//! punto decimal y con la escala exacta de su moneda para evitar conversiones
//! en coma flotante en las capas superiores.

use serde::Serialize;

/// Representación de una cuenta financiera para su visualización en listados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CuentaListada {
    /// Identificador único de la cuenta.
    pub id: String,
    /// Nombre asignado a la cuenta.
    pub nombre: String,
    /// Categoría de la cuenta (`"efectivo"`, `"banco"`, `"billetera_digital"`).
    pub tipo: String,
    /// Código de la moneda asociada (`"VES"`, `"USD"`, `"EUR"`, `"USDT"`).
    pub moneda: String,
    /// Saldo actual formateado con punto decimal según los decimales de la moneda.
    pub saldo_actual: String,
}

/// Suma total de saldos agrupada para una moneda específica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subtotal {
    /// Código de la moneda a la que corresponde el subtotal.
    pub moneda: String,
    /// Monto total acumulado formateado como texto decimal.
    pub total: String,
}

/// Resultado de consultar las cuentas registradas y sus respectivos subtotales por moneda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListaCuentas {
    /// Lista de cuentas ordenadas cronológicamente según su orden de creación.
    pub cuentas: Vec<CuentaListada>,
    /// Subtotales calculados de forma independiente por cada moneda presente.
    pub subtotales: Vec<Subtotal>,
}

/// Datos descriptivos de una moneda soportada por el catálogo del sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MonedaDto {
    /// Código identificador de la moneda (por ejemplo, `"USD"`).
    pub codigo: String,
    /// Nombre completo de la moneda.
    pub nombre: String,
    /// Símbolo monetario de visualización.
    pub simbolo: String,
    /// Número de posiciones decimales permitidas.
    pub decimales: u8,
}
