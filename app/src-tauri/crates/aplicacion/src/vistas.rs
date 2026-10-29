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
    /// Indica si la moneda pertenece al catálogo de fábrica.
    pub de_fabrica: bool,
}

/// Representación de una categoría de ingresos o gastos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CategoriaListada {
    /// Identificador único de la categoría (por ejemplo, `"gasto:alimentacion"` o UUID).
    pub id: String,
    /// Nombre visible de la categoría.
    pub nombre: String,
    /// Tipo de movimiento al que aplica (`"ingreso"` o `"gasto"`).
    pub tipo: String,
    /// Indica si la categoría pertenece al catálogo predefinido del sistema.
    pub de_fabrica: bool,
}

/// Resumen de categoría embebido dentro de cada movimiento listado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CategoriaResumen {
    /// Identificador único de la categoría.
    pub id: String,
    /// Nombre visible de la categoría.
    pub nombre: String,
}

/// Representación de un movimiento o ajuste dentro del historial de una cuenta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MovimientoListado {
    /// Identificador único del movimiento o ajuste.
    pub id: String,
    /// Identificador de la cuenta a la que pertenece.
    pub cuenta_id: String,
    /// Tipo de operación (`"ingreso"` o `"gasto"`).
    pub tipo: String,
    /// Monto absoluto como texto positivo con punto decimal (ej. `"500.00"`).
    pub monto: String,
    /// Monto con signo explícito según su efecto en el saldo (ej. `"+800.00"`, `"-500.00"`, `"+450.00"`).
    pub monto_con_signo: String,
    /// Valor vigente del movimiento tras aplicar sus ajustes (`None` cuando el registro es un ajuste).
    pub monto_vigente: Option<String>,
    /// Fecha del movimiento en formato `"AAAA-MM-DD"`.
    pub fecha: String,
    /// Datos de la categoría asociada.
    pub categoria: CategoriaResumen,
    /// Nota descriptiva opcional (cadena vacía si no se especificó).
    pub nota: String,
    /// Estado del registro: `"vigente"`, `"corregido"` o `"ajuste"`.
    pub estado: String,
    /// Identificador del movimiento original cuando este registro es un `"ajuste"`.
    pub corrige_a: Option<String>,
    /// Lista de identificadores de los ajustes asociados cuando este registro es un movimiento original.
    pub ajustes: Vec<String>,
}

/// Desglose de los componentes que integran el saldo actual de una cuenta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComposicionVista {
    /// Saldo con el que se abrió la cuenta.
    pub saldo_inicial: String,
    /// Suma total de los ingresos originales registrados.
    pub total_ingresos: String,
    /// Suma total de los gastos originales registrados.
    pub total_gastos: String,
    /// Suma algebraica neta de los efectos de todos los ajustes aplicados.
    pub total_ajustes: String,
}

/// Vista detallada de una cuenta con su saldo actual, composición y movimientos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetalleCuentaVista {
    /// Datos generales de la cuenta con su saldo actual derivado.
    pub cuenta: CuentaListada,
    /// Desglose de saldo inicial, ingresos, gastos y ajustes.
    pub composicion: ComposicionVista,
    /// Lista de movimientos ordenados del más reciente al más antiguo.
    pub movimientos: Vec<MovimientoListado>,
}

/// Resultado de simular el efecto de un movimiento antes de registrarlo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrevisualizacionMovimiento {
    /// Saldo que tendría la cuenta si el movimiento se confirmara.
    pub saldo_resultante: String,
    /// Indica si la operación es un gasto que deja el saldo de la cuenta menor que cero.
    pub deja_saldo_negativo: bool,
}

/// Representación de una tasa de cambio en listados e historial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TasaListada {
    /// Identificador único de la tasa.
    pub id: String,
    /// Código de la moneda de origen.
    pub moneda_origen: String,
    /// Código de la moneda de destino.
    pub moneda_destino: String,
    /// Valor decimal formateado como texto.
    pub valor: String,
    /// Fecha de la tasa (AAAA-MM-DD).
    pub fecha: String,
    /// Fuente de la tasa (ej. "BCV", "P2P").
    pub fuente: String,
}

/// Representación de una cuenta en la vista de patrimonio consolidado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CuentaPatrimonioVista {
    /// Identificador de la cuenta.
    pub id: String,
    /// Nombre visible.
    pub nombre: String,
    /// Moneda de la cuenta.
    pub moneda: String,
    /// Saldo de la cuenta a la fecha de consulta formateado con sus decimales.
    pub saldo: String,
}

/// Tasa de cambio empleada durante la consolidación de patrimonio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TasaUsadaVista {
    /// Moneda origen de la tasa.
    pub origen: String,
    /// Moneda destino de la tasa.
    pub destino: String,
    /// Valor decimal de la tasa como texto.
    pub valor: String,
    /// Fecha de vigencia de la tasa (AAAA-MM-DD).
    pub fecha: String,
    /// Fuente declarada.
    pub fuente: String,
    /// Indica si se aplicó como tasa inversa (1 / valor).
    pub es_inversa: bool,
}

/// Par de monedas para el cual falta una tasa aplicable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParFaltanteVista {
    /// Moneda de origen que requiere conversión.
    pub origen: String,
    /// Moneda de destino (moneda base).
    pub destino: String,
}

/// Vista consolidada del patrimonio del usuario a una fecha de consulta (RF-CAM-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatrimonioVista {
    /// Moneda base en la que se calcula el total consolidado.
    pub moneda_base: String,
    /// Fecha a la que se realiza la consulta (AAAA-MM-DD).
    pub fecha_consulta: String,
    /// Saldos de las cuentas activas a la fecha de consulta.
    pub cuentas: Vec<CuentaPatrimonioVista>,
    /// Subtotales independientes por cada moneda presente.
    pub subtotales: Vec<Subtotal>,
    /// Total consolidado redondeado a los decimales de la moneda base, o `None` si falta alguna tasa.
    pub total: Option<String>,
    /// Tasas de cambio utilizadas en la consolidación.
    pub tasas_usadas: Vec<TasaUsadaVista>,
    /// Pares de monedas faltantes que impiden calcular el total consolidado.
    pub faltantes: Vec<ParFaltanteVista>,
}
