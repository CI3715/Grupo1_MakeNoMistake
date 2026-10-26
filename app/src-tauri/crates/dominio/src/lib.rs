//! Reglas de negocio. No depende de la base de datos, la red ni el framework.
//!
//! Tipos públicos: [`Moneda`], [`Monto`], [`TipoCuenta`], [`Cuenta`], [`CuentaCreada`],
//! [`TipoMovimiento`], [`EstadoMovimiento`], [`Categoria`], [`Movimiento`], [`Ajuste`],
//! [`ComposicionSaldo`], [`MovimientoRegistrado`], [`MovimientoCorregido`], [`CategoriaCreada`],
//! [`Campo`], [`ErrorDominio`].

use rust_decimal::Decimal;
use std::str::FromStr;

// ──────────────────────────── Errores ────────────────────────────

/// Campos del formulario que pueden originar un error de validación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Campo {
    Nombre,
    Moneda,
    SaldoInicial,
    Cuenta,
    Tipo,
    Monto,
    Fecha,
    Categoria,
    Nota,
}

impl Campo {
    /// Devuelve el identificador textual del campo en formato snake_case.
    pub fn a_texto(&self) -> &'static str {
        match self {
            Campo::Nombre => "nombre",
            Campo::Moneda => "moneda",
            Campo::SaldoInicial => "saldo_inicial",
            Campo::Cuenta => "cuenta",
            Campo::Tipo => "tipo",
            Campo::Monto => "monto",
            Campo::Fecha => "fecha",
            Campo::Categoria => "categoria",
            Campo::Nota => "nota",
        }
    }
}

/// Error de dominio asociado a un campo específico del formulario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDominio {
    pub campo: Campo,
    pub mensaje: String,
}

impl ErrorDominio {
    /// Atajo para construir un error.
    pub fn nuevo(campo: Campo, mensaje: impl Into<String>) -> Self {
        Self {
            campo,
            mensaje: mensaje.into(),
        }
    }
}

// ──────────────────────────── Moneda ─────────────────────────────

/// Moneda soportada por la aplicación.
///
/// El catálogo fijo contiene VES, USD, EUR y USDT, todas con 2 decimales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moneda {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u32,
}

impl Moneda {
    /// Devuelve el catálogo completo de monedas soportadas.
    pub fn catalogo() -> Vec<Moneda> {
        vec![
            Moneda {
                codigo: "VES".into(),
                nombre: "Bolívar".into(),
                simbolo: "Bs.".into(),
                decimales: 2,
            },
            Moneda {
                codigo: "USD".into(),
                nombre: "Dólar estadounidense".into(),
                simbolo: "$".into(),
                decimales: 2,
            },
            Moneda {
                codigo: "EUR".into(),
                nombre: "Euro".into(),
                simbolo: "€".into(),
                decimales: 2,
            },
            Moneda {
                codigo: "USDT".into(),
                nombre: "Tether (dólar digital)".into(),
                simbolo: "USDT".into(),
                decimales: 2,
            },
        ]
    }

    /// Busca una moneda por código (insensible a mayúsculas/minúsculas).
    pub fn por_codigo(codigo: &str) -> Option<Moneda> {
        let codigo_upper = codigo.trim().to_uppercase();
        Self::catalogo()
            .into_iter()
            .find(|m| m.codigo == codigo_upper)
    }
}

// ──────────────────────────── Monto ──────────────────────────────

/// Cantidad de dinero en una moneda específica.
///
/// Los campos son privados para garantizar que todo `Monto` sea válido:
/// la cantidad tiene como máximo los decimales permitidos por su moneda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monto {
    cantidad: Decimal,
    moneda: Moneda,
}

impl Monto {
    /// Crea un monto a partir de un `Decimal` y una `Moneda`.
    ///
    /// No valida decimales; se usa internamente después de haber validado.
    pub(crate) fn new(cantidad: Decimal, moneda: Moneda) -> Self {
        Self { cantidad, moneda }
    }

    /// Cantidad numérica.
    pub fn cantidad(&self) -> Decimal {
        self.cantidad
    }

    /// Moneda del monto.
    pub fn moneda(&self) -> &Moneda {
        &self.moneda
    }

    /// Suma dos montos de la **misma** moneda.
    ///
    /// Devuelve error si las monedas son distintas (no se deben sumar
    /// monedas diferentes — criterio 4).
    pub fn sumar(&self, otro: &Monto) -> Result<Monto, ErrorDominio> {
        if self.moneda.codigo != otro.moneda.codigo {
            return Err(ErrorDominio::nuevo(
                Campo::Moneda,
                format!(
                    "No se pueden sumar montos de monedas distintas ({} y {})",
                    self.moneda.codigo, otro.moneda.codigo
                ),
            ));
        }
        Ok(Monto::new(
            self.cantidad + otro.cantidad,
            self.moneda.clone(),
        ))
    }

    /// Resta dos montos de la **misma** moneda.
    pub fn restar(&self, otro: &Monto) -> Result<Monto, ErrorDominio> {
        if self.moneda.codigo != otro.moneda.codigo {
            return Err(ErrorDominio::nuevo(
                Campo::Moneda,
                format!(
                    "No se pueden restar montos de monedas distintas ({} y {})",
                    self.moneda.codigo, otro.moneda.codigo
                ),
            ));
        }
        Ok(Monto::new(
            self.cantidad - otro.cantidad,
            self.moneda.clone(),
        ))
    }
}

// ──────────────────────────── TipoCuenta ─────────────────────────

/// Tipo de cuenta financiera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoCuenta {
    Efectivo,
    Banco,
    BilleteraDigital,
}

// ──────────────────────────── Cuenta ─────────────────────────────

/// Cuenta financiera del usuario.
///
/// Todos los campos son privados: **no existe forma de cambiar la moneda**
/// una vez creada la cuenta (INV-18, RF-CAM-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cuenta {
    id: String,
    nombre: String,
    tipo: TipoCuenta,
    saldo: Monto,
}

impl Cuenta {
    // ── Getters públicos ──

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn nombre(&self) -> &str {
        &self.nombre
    }

    pub fn tipo(&self) -> TipoCuenta {
        self.tipo
    }

    pub fn saldo(&self) -> &Monto {
        &self.saldo
    }

    // ── Fábrica con validación ──

    /// Crea una cuenta nueva validando **todas** las reglas de negocio.
    ///
    /// Recibe el `id` ya generado (la generación es responsabilidad de la
    /// capa de aplicación) y la lista de `nombres_existentes` para verificar
    /// unicidad sin depender de infraestructura.
    ///
    /// `saldo_inicial` se recibe como texto (e.g. `"1250.50"`) y se
    /// convierte a `Decimal` dentro del dominio.
    ///
    /// Si hay errores, los acumula **todos** y devuelve
    /// `Err(Vec<ErrorDominio>)` para que la pantalla pueda mostrarlos
    /// simultáneamente.
    pub fn crear(
        id: String,
        nombre: &str,
        tipo: TipoCuenta,
        moneda_codigo: &str,
        saldo_inicial: &str,
        nombres_existentes: &[String],
    ) -> Result<(Cuenta, CuentaCreada), Vec<ErrorDominio>> {
        let mut errores: Vec<ErrorDominio> = Vec::new();

        // ── 1. Nombre vacío o solo espacios ──
        let nombre_limpio = nombre.trim().to_string();
        if nombre_limpio.is_empty() {
            errores.push(ErrorDominio::nuevo(
                Campo::Nombre,
                "El nombre no puede estar vacío",
            ));
        }

        // ── 2. Nombre repetido (sin distinguir mayúsculas) ──
        if !nombre_limpio.is_empty() {
            let nombre_lower = nombre_limpio.to_lowercase();
            let repetido = nombres_existentes
                .iter()
                .any(|n| n.trim().to_lowercase() == nombre_lower);
            if repetido {
                errores.push(ErrorDominio::nuevo(
                    Campo::Nombre,
                    "Ya existe una cuenta con ese nombre",
                ));
            }
        }

        // ── 3. Moneda fuera del catálogo ──
        let moneda = Moneda::por_codigo(moneda_codigo);
        if moneda.is_none() {
            errores.push(ErrorDominio::nuevo(
                Campo::Moneda,
                format!(
                    "La moneda '{}' no está en el catálogo. Opciones: VES, USD, EUR, USDT",
                    moneda_codigo
                ),
            ));
        }

        // ── 4 y 5. Saldo: parseo, negativo, decimales ──
        let saldo_decimal = Decimal::from_str(saldo_inicial.trim());
        match &saldo_decimal {
            Err(_) => {
                errores.push(ErrorDominio::nuevo(
                    Campo::SaldoInicial,
                    "El saldo inicial no es un número válido",
                ));
            }
            Ok(d) => {
                // 4. Saldo negativo
                if d.is_sign_negative() && !d.is_zero() {
                    errores.push(ErrorDominio::nuevo(
                        Campo::SaldoInicial,
                        "El saldo inicial no puede ser negativo",
                    ));
                }

                // 5. Más decimales de los que permite la moneda
                if let Some(ref m) = moneda {
                    // normalize() descarta ceros finales: "10.500" vale 10.5 y es válido.
                    let escala = d.normalize().scale();
                    if escala > m.decimales {
                        errores.push(ErrorDominio::nuevo(
                            Campo::SaldoInicial,
                            format!(
                                "El saldo tiene {} decimales, pero {} solo admite {}",
                                escala, m.codigo, m.decimales
                            ),
                        ));
                    }
                }
            }
        }

        // ── Resultado ──
        if !errores.is_empty() {
            return Err(errores);
        }

        // Si llegamos aquí, todo es válido.
        let moneda = moneda.expect("moneda validada");
        let mut cantidad = saldo_decimal.expect("saldo validado");
        if cantidad.is_zero() {
            cantidad = Decimal::ZERO; // evita "-0"
        }
        cantidad.rescale(moneda.decimales); // escala canónica: 300 -> 300.00

        let cuenta = Cuenta {
            id: id.clone(),
            nombre: nombre_limpio.clone(),
            tipo,
            saldo: Monto::new(cantidad, moneda.clone()),
        };

        let evento = CuentaCreada {
            id,
            nombre: nombre_limpio,
            tipo,
            moneda,
            saldo_inicial: cantidad,
        };

        Ok((cuenta, evento))
    }
}

// ──────────────────────────── CuentaCreada (evento) ──────────────

/// Evento de dominio emitido cuando se crea una cuenta con éxito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuentaCreada {
    pub id: String,
    pub nombre: String,
    pub tipo: TipoCuenta,
    pub moneda: Moneda,
    pub saldo_inicial: Decimal,
}

// ──────────────────────────── TipoMovimiento ──────────────────────

/// Tipo de movimiento financiero (ingreso o gasto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoMovimiento {
    Ingreso,
    Gasto,
}

// ──────────────────────────── EstadoMovimiento ────────────────────

/// Estado de un movimiento financiero en el libro de eventos inmutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoMovimiento {
    Vigente,
    Corregido,
    Ajuste { id_movimiento_original: String },
}

// ──────────────────────────── Categoria ──────────────────────────

/// Categoría de clasificación para ingresos o gastos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Categoria {
    pub id: String,
    pub nombre: String,
    pub tipo: TipoMovimiento,
    pub de_fabrica: bool,
}

// ──────────────────────────── Movimiento ─────────────────────────

/// Movimiento financiero inmutable asociado a una cuenta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movimiento {
    pub id: String,
    pub id_cuenta: String,
    pub tipo: TipoMovimiento,
    pub monto: Monto,
    pub fecha: String,
    pub id_categoria: String,
    pub nota: Option<String>,
    pub estado: EstadoMovimiento,
}

impl Movimiento {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn id_cuenta(&self) -> &str {
        &self.id_cuenta
    }

    pub fn tipo(&self) -> TipoMovimiento {
        self.tipo
    }

    pub fn monto(&self) -> &Monto {
        &self.monto
    }

    pub fn moneda(&self) -> &Moneda {
        self.monto.moneda()
    }

    pub fn fecha(&self) -> &str {
        &self.fecha
    }

    pub fn id_categoria(&self) -> &str {
        &self.id_categoria
    }

    pub fn nota(&self) -> Option<&str> {
        self.nota.as_deref()
    }

    pub fn estado(&self) -> &EstadoMovimiento {
        &self.estado
    }
}

// ──────────────────────────── Ajuste ─────────────────────────────

/// Ajuste correctivo que referencia a un movimiento original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ajuste {
    pub id_ajuste: String,
    pub id_movimiento_original: String,
    pub monto_correcto: Decimal,
    pub efecto: Decimal,
    pub moneda: Moneda,
}

// ──────────────────────────── ComposicionSaldo ───────────────────

/// Desglose y composición del saldo derivado de los movimientos de una cuenta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposicionSaldo {
    pub saldo_inicial: Monto,
    pub total_ingresos: Monto,
    pub total_gastos: Monto,
    pub total_ajustes: Monto,
    pub saldo_actual: Monto,
}

// ──────────────────── Eventos de Movimientos y Categorías ────────

/// Evento emitido cuando se registra un nuevo movimiento financiero.
/// Campos acordados en el acuerdo del equipo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoRegistrado {
    pub id_movimiento: String,
    pub id_cuenta: String,
    pub tipo: TipoMovimiento,
    pub monto: Decimal,
    pub moneda: Moneda,
    pub fecha: String,
    pub id_categoria: String,
    pub nota: Option<String>,
}

/// Evento emitido cuando se corrige un movimiento previo mediante un ajuste.
/// Campos exactos acordados en el acuerdo del equipo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoCorregido {
    pub id_ajuste: String,
    pub id_movimiento_original: String,
    pub monto_correcto: Decimal,
    pub efecto: Decimal,
    pub moneda: Moneda,
}

/// Evento emitido cuando se crea una categoría de usuario.
/// Campos exactos acordados en el acuerdo del equipo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriaCreada {
    pub id_categoria: String,
    pub nombre: String,
    pub tipo: TipoMovimiento,
}

// ──────────────────── Funciones de Dominio ────────────────────────

/// Valida que una cadena represente una fecha gregoriana válida con formato AAAA-MM-DD.
#[allow(clippy::manual_is_multiple_of)]
pub fn validar_fecha(fecha: &str) -> Result<(), ErrorDominio> {
    let err = || ErrorDominio::nuevo(Campo::Fecha, "La fecha debe tener el formato AAAA-MM-DD");
    if fecha.len() != 10 {
        return Err(err());
    }
    let bytes = fecha.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(err());
    }
    let anio: u32 = fecha[0..4].parse().map_err(|_| err())?;
    let mes: u32 = fecha[5..7].parse().map_err(|_| err())?;
    let dia: u32 = fecha[8..10].parse().map_err(|_| err())?;

    if anio < 1000 || !(1..=12).contains(&mes) {
        return Err(ErrorDominio::nuevo(
            Campo::Fecha,
            "Mes o año inválido en la fecha",
        ));
    }

    let es_bisiesto = (anio % 4 == 0 && anio % 100 != 0) || (anio % 400 == 0);
    let max_dias = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if es_bisiesto {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    };

    if dia < 1 || dia > max_dias {
        return Err(ErrorDominio::nuevo(
            Campo::Fecha,
            format!("El día {} no es válido para el mes {}", dia, mes),
        ));
    }

    Ok(())
}

/// Registra un nuevo movimiento financiero validando todas las reglas de negocio.
///
/// Recibe `hoy` como parámetro para validar que la fecha no sea futura sin leer el reloj.
#[allow(clippy::too_many_arguments)]
pub fn registrar_movimiento(
    id: String,
    id_cuenta: String,
    tipo: TipoMovimiento,
    monto_str: &str,
    moneda_codigo: &str,
    fecha: &str,
    hoy: &str,
    id_categoria: &str,
    nota: Option<String>,
    categorias_existentes: &[Categoria],
) -> Result<(Movimiento, MovimientoRegistrado), Vec<ErrorDominio>> {
    let mut errores: Vec<ErrorDominio> = Vec::new();

    // ── 1. Validación de cuenta ──
    let id_cuenta_limpio = id_cuenta.trim().to_string();
    if id_cuenta_limpio.is_empty() {
        errores.push(ErrorDominio::nuevo(
            Campo::Cuenta,
            "El identificador de cuenta no puede estar vacío",
        ));
    }

    // ── 2. Validación de moneda ──
    let moneda = Moneda::por_codigo(moneda_codigo);
    if moneda.is_none() {
        errores.push(ErrorDominio::nuevo(
            Campo::Moneda,
            format!(
                "La moneda '{}' no está en el catálogo. Opciones: VES, USD, EUR, USDT",
                moneda_codigo
            ),
        ));
    }

    // ── 3. Validación de monto ──
    let monto_decimal = Decimal::from_str(monto_str.trim());
    match &monto_decimal {
        Err(_) => {
            errores.push(ErrorDominio::nuevo(
                Campo::Monto,
                "El monto no es un número válido",
            ));
        }
        Ok(d) => {
            if *d <= Decimal::ZERO {
                errores.push(ErrorDominio::nuevo(
                    Campo::Monto,
                    "El monto debe ser mayor que cero",
                ));
            } else if let Some(ref m) = moneda {
                let escala = d.normalize().scale();
                if escala > m.decimales {
                    errores.push(ErrorDominio::nuevo(
                        Campo::Monto,
                        format!(
                            "El monto tiene {} decimales, pero {} solo admite {}",
                            escala, m.codigo, m.decimales
                        ),
                    ));
                }
            }
        }
    }

    // ── 4. Validación de categoría ──
    let id_cat_limpio = id_categoria.trim();
    if id_cat_limpio.is_empty() {
        errores.push(ErrorDominio::nuevo(
            Campo::Categoria,
            "La categoría no puede estar vacía",
        ));
    } else {
        match categorias_existentes.iter().find(|c| c.id == id_cat_limpio) {
            None => {
                errores.push(ErrorDominio::nuevo(
                    Campo::Categoria,
                    format!("La categoría '{}' no existe", id_cat_limpio),
                ));
            }
            Some(cat) => {
                if cat.tipo != tipo {
                    errores.push(ErrorDominio::nuevo(
                        Campo::Categoria,
                        format!(
                            "La categoría '{}' es de tipo {:?}, pero el movimiento es de tipo {:?}",
                            id_cat_limpio, cat.tipo, tipo
                        ),
                    ));
                }
            }
        }
    }

    // ── 5. Validación de fecha y fecha futura ──
    if let Err(e) = validar_fecha(hoy) {
        errores.push(e);
    } else if let Err(e) = validar_fecha(fecha) {
        errores.push(e);
    } else if fecha > hoy {
        errores.push(ErrorDominio::nuevo(
            Campo::Fecha,
            "La fecha no puede ser futura",
        ));
    }

    if !errores.is_empty() {
        return Err(errores);
    }

    let moneda = moneda.expect("moneda validada");
    let mut cantidad = monto_decimal.expect("monto validado");
    cantidad.rescale(moneda.decimales);

    let nota_limpia = nota.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let movimiento = Movimiento {
        id: id.clone(),
        id_cuenta: id_cuenta_limpio.clone(),
        tipo,
        monto: Monto::new(cantidad, moneda.clone()),
        fecha: fecha.to_string(),
        id_categoria: id_cat_limpio.to_string(),
        nota: nota_limpia.clone(),
        estado: EstadoMovimiento::Vigente,
    };

    let evento = MovimientoRegistrado {
        id_movimiento: id,
        id_cuenta: id_cuenta_limpio,
        tipo,
        monto: cantidad,
        moneda,
        fecha: fecha.to_string(),
        id_categoria: id_cat_limpio.to_string(),
        nota: nota_limpia,
    };

    Ok((movimiento, evento))
}

/// Corrige un movimiento financiero generando un ajuste.
///
/// El valor vigente se calcula a partir del monto original y los ajustes previos.
/// Para un gasto: `vigente = original - Σ(efectos)`.
/// Para un ingreso: `vigente = original + Σ(efectos)`.
///
/// El efecto del nuevo ajuste se calcula como:
/// - Gasto: `vigente - monto_correcto`
/// - Ingreso: `monto_correcto - vigente`
pub fn corregir_movimiento(
    id_ajuste: String,
    id_movimiento_original: &str,
    monto_correcto_str: &str,
    movimientos_existentes: &[Movimiento],
    ajustes_previos: &[Ajuste],
) -> Result<(Ajuste, MovimientoCorregido), Vec<ErrorDominio>> {
    let mut errores: Vec<ErrorDominio> = Vec::new();

    // 1. Buscar movimiento original
    let original = movimientos_existentes
        .iter()
        .find(|m| m.id == id_movimiento_original);

    let original = match original {
        None => {
            errores.push(ErrorDominio::nuevo(
                Campo::Monto,
                format!(
                    "El movimiento original '{}' no existe",
                    id_movimiento_original
                ),
            ));
            return Err(errores);
        }
        Some(m) => m,
    };

    // 2. No se puede corregir un ajuste
    if matches!(original.estado, EstadoMovimiento::Ajuste { .. }) {
        errores.push(ErrorDominio::nuevo(
            Campo::Tipo,
            "No se puede corregir un movimiento que ya es un ajuste",
        ));
        return Err(errores);
    }

    // 3. Validar monto_correcto
    let monto_dec = Decimal::from_str(monto_correcto_str.trim());
    let mut monto_correcto = match monto_dec {
        Err(_) => {
            errores.push(ErrorDominio::nuevo(
                Campo::Monto,
                "El monto correcto no es un número válido",
            ));
            return Err(errores);
        }
        Ok(d) => d,
    };

    if monto_correcto.is_sign_negative() && !monto_correcto.is_zero() {
        errores.push(ErrorDominio::nuevo(
            Campo::Monto,
            "El monto correcto no puede ser negativo",
        ));
    }

    if monto_correcto.is_zero() {
        monto_correcto = Decimal::ZERO;
    }

    let moneda = original.monto.moneda();
    let escala = monto_correcto.normalize().scale();
    if escala > moneda.decimales {
        errores.push(ErrorDominio::nuevo(
            Campo::Monto,
            format!(
                "El monto correcto tiene {} decimales, pero {} solo admite {}",
                escala, moneda.codigo, moneda.decimales
            ),
        ));
    }

    if !errores.is_empty() {
        return Err(errores);
    }

    monto_correcto.rescale(moneda.decimales);

    // 4. Calcular valor vigente acumulado
    let ajustes_este_mov: Vec<&Ajuste> = ajustes_previos
        .iter()
        .filter(|a| a.id_movimiento_original == original.id)
        .collect();

    let suma_efectos: Decimal = ajustes_este_mov.iter().map(|a| a.efecto).sum();

    let mut original_cant = original.monto.cantidad();
    original_cant.rescale(moneda.decimales);

    let vigente = match original.tipo {
        TipoMovimiento::Gasto => original_cant - suma_efectos,
        TipoMovimiento::Ingreso => original_cant + suma_efectos,
    };

    // Verificación de redundancia: el monto_correcto del último ajuste debe ser igual al vigente
    if let Some(ultimo_ajuste) = ajustes_este_mov.last() {
        debug_assert_eq!(
            ultimo_ajuste.monto_correcto, vigente,
            "Invariante violada: monto_correcto del último ajuste no coincide con el vigente recalculado"
        );
    }

    // 5. Rechazar si el monto es igual al vigente
    if monto_correcto == vigente {
        errores.push(ErrorDominio::nuevo(
            Campo::Monto,
            "El monto correcto no puede ser igual al monto vigente",
        ));
        return Err(errores);
    }

    // 6. Calcular efecto
    let mut efecto = match original.tipo {
        TipoMovimiento::Gasto => vigente - monto_correcto,
        TipoMovimiento::Ingreso => monto_correcto - vigente,
    };
    efecto.rescale(moneda.decimales);

    let ajuste = Ajuste {
        id_ajuste: id_ajuste.clone(),
        id_movimiento_original: original.id.clone(),
        monto_correcto,
        efecto,
        moneda: moneda.clone(),
    };

    let evento = MovimientoCorregido {
        id_ajuste,
        id_movimiento_original: original.id.clone(),
        monto_correcto,
        efecto,
        moneda: moneda.clone(),
    };

    Ok((ajuste, evento))
}

/// Crea una nueva categoría validando nombre no vacío y unicidad sin distinguir mayúsculas dentro del mismo tipo.
pub fn crear_categoria(
    id_categoria: String,
    nombre: &str,
    tipo: TipoMovimiento,
    categorias_existentes: &[Categoria],
) -> Result<(Categoria, CategoriaCreada), Vec<ErrorDominio>> {
    let mut errores: Vec<ErrorDominio> = Vec::new();

    let nombre_limpio = nombre.trim().to_string();
    if nombre_limpio.is_empty() {
        errores.push(ErrorDominio::nuevo(
            Campo::Nombre,
            "El nombre de la categoría no puede estar vacío",
        ));
    } else {
        let nombre_lower = nombre_limpio.to_lowercase();
        let repetido = categorias_existentes
            .iter()
            .any(|c| c.tipo == tipo && c.nombre.trim().to_lowercase() == nombre_lower);
        if repetido {
            errores.push(ErrorDominio::nuevo(
                Campo::Nombre,
                "Ya existe una categoría con ese nombre para este tipo de movimiento",
            ));
        }
    }

    if !errores.is_empty() {
        return Err(errores);
    }

    let categoria = Categoria {
        id: id_categoria.clone(),
        nombre: nombre_limpio.clone(),
        tipo,
        de_fabrica: false,
    };

    let evento = CategoriaCreada {
        id_categoria,
        nombre: nombre_limpio,
        tipo,
    };

    Ok((categoria, evento))
}

/// Devuelve el catálogo de categorías predefinidas de fábrica (7 de gasto y 4 de ingreso).
pub fn categorias_de_fabrica() -> Vec<Categoria> {
    vec![
        Categoria {
            id: "gasto:alimentacion".into(),
            nombre: "Alimentación".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:transporte".into(),
            nombre: "Transporte".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:servicios".into(),
            nombre: "Servicios".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:salud".into(),
            nombre: "Salud".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:educacion".into(),
            nombre: "Educación".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:entretenimiento".into(),
            nombre: "Entretenimiento".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "gasto:otros".into(),
            nombre: "Otros gastos".into(),
            tipo: TipoMovimiento::Gasto,
            de_fabrica: true,
        },
        Categoria {
            id: "ingreso:salario".into(),
            nombre: "Salario".into(),
            tipo: TipoMovimiento::Ingreso,
            de_fabrica: true,
        },
        Categoria {
            id: "ingreso:ventas".into(),
            nombre: "Ventas".into(),
            tipo: TipoMovimiento::Ingreso,
            de_fabrica: true,
        },
        Categoria {
            id: "ingreso:regalos".into(),
            nombre: "Regalos".into(),
            tipo: TipoMovimiento::Ingreso,
            de_fabrica: true,
        },
        Categoria {
            id: "ingreso:otros".into(),
            nombre: "Otros ingresos".into(),
            tipo: TipoMovimiento::Ingreso,
            de_fabrica: true,
        },
    ]
}

/// Calcula el saldo y desglose de composición a partir de una cuenta, sus movimientos y ajustes.
///
/// Invariante: `saldo_actual = saldo_inicial + total_ingresos - total_gastos + total_ajustes`.
pub fn calcular_saldo_y_composicion(
    cuenta: &Cuenta,
    movimientos: &[Movimiento],
    ajustes: &[Ajuste],
) -> Result<ComposicionSaldo, Vec<ErrorDominio>> {
    let moneda = cuenta.saldo().moneda();

    for m in movimientos {
        if m.monto.moneda().codigo != moneda.codigo {
            return Err(vec![ErrorDominio::nuevo(
                Campo::Moneda,
                format!(
                    "El movimiento {} tiene moneda distinta ({}) a la de la cuenta ({})",
                    m.id,
                    m.monto.moneda().codigo,
                    moneda.codigo
                ),
            )]);
        }
    }

    for a in ajustes {
        if a.moneda.codigo != moneda.codigo {
            return Err(vec![ErrorDominio::nuevo(
                Campo::Moneda,
                format!(
                    "El ajuste {} tiene moneda distinta ({}) a la de la cuenta ({})",
                    a.id_ajuste, a.moneda.codigo, moneda.codigo
                ),
            )]);
        }
    }

    let mut total_ingresos_dec = Decimal::ZERO;
    let mut total_gastos_dec = Decimal::ZERO;

    for m in movimientos {
        if matches!(m.estado, EstadoMovimiento::Ajuste { .. }) {
            continue;
        }
        match m.tipo {
            TipoMovimiento::Ingreso => {
                total_ingresos_dec += m.monto.cantidad();
            }
            TipoMovimiento::Gasto => {
                total_gastos_dec += m.monto.cantidad();
            }
        }
    }

    let mut total_ajustes_dec: Decimal = ajustes.iter().map(|a| a.efecto).sum();

    total_ingresos_dec.rescale(moneda.decimales);
    total_gastos_dec.rescale(moneda.decimales);
    total_ajustes_dec.rescale(moneda.decimales);

    let mut saldo_actual_dec =
        cuenta.saldo().cantidad() + total_ingresos_dec - total_gastos_dec + total_ajustes_dec;
    saldo_actual_dec.rescale(moneda.decimales);

    Ok(ComposicionSaldo {
        saldo_inicial: cuenta.saldo().clone(),
        total_ingresos: Monto::new(total_ingresos_dec, moneda.clone()),
        total_gastos: Monto::new(total_gastos_dec, moneda.clone()),
        total_ajustes: Monto::new(total_ajustes_dec, moneda.clone()),
        saldo_actual: Monto::new(saldo_actual_dec, moneda.clone()),
    })
}

/// Función pura que simula el saldo resultante tras aplicar un gasto hipotético.
/// Ángel la utiliza para previsualizar movimientos interactivos en la interfaz.
pub fn saldo_resultante(
    saldo_actual: &Monto,
    gasto_hipotetico: &Monto,
) -> Result<Monto, ErrorDominio> {
    saldo_actual.restar(gasto_hipotetico)
}

// ──────────────────────────── Tests ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── Catálogo de monedas ──

    #[test]
    fn catalogo_tiene_cuatro_monedas() {
        let catalogo = Moneda::catalogo();
        assert_eq!(catalogo.len(), 4);
        let codigos: Vec<&str> = catalogo.iter().map(|m| m.codigo.as_str()).collect();
        assert!(codigos.contains(&"VES"));
        assert!(codigos.contains(&"USD"));
        assert!(codigos.contains(&"EUR"));
        assert!(codigos.contains(&"USDT"));
    }

    #[test]
    fn todas_las_monedas_tienen_dos_decimales() {
        for m in Moneda::catalogo() {
            assert_eq!(
                m.decimales, 2,
                "La moneda {} no tiene 2 decimales",
                m.codigo
            );
        }
    }

    #[test]
    fn catalogo_coincide_con_el_enunciado() {
        let esperado = [
            ("VES", "Bolívar", "Bs."),
            ("USD", "Dólar estadounidense", "$"),
            ("EUR", "Euro", "€"),
            ("USDT", "Tether (dólar digital)", "USDT"),
        ];
        let catalogo = Moneda::catalogo();
        for (codigo, nombre, simbolo) in esperado {
            let m = catalogo
                .iter()
                .find(|m| m.codigo == codigo)
                .expect("moneda del catálogo");
            assert_eq!(m.nombre, nombre);
            assert_eq!(m.simbolo, simbolo);
        }
    }

    #[test]
    fn por_codigo_encuentra_moneda() {
        let usd = Moneda::por_codigo("usd").expect("debe encontrar USD");
        assert_eq!(usd.codigo, "USD");
        assert_eq!(usd.simbolo, "$");
    }

    #[test]
    fn por_codigo_inexistente_devuelve_none() {
        assert!(Moneda::por_codigo("BTC").is_none());
    }

    // ── Monto: suma y resta ──

    #[test]
    fn sumar_misma_moneda() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let a = Monto::new(dec!(10.50), usd.clone());
        let b = Monto::new(dec!(4.25), usd);
        let resultado = a.sumar(&b).expect("misma moneda, debe funcionar");
        assert_eq!(resultado.cantidad(), dec!(14.75));
    }

    #[test]
    fn sumar_monedas_distintas_falla() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let eur = Moneda::por_codigo("EUR").unwrap();
        let a = Monto::new(dec!(10), usd);
        let b = Monto::new(dec!(5), eur);
        let err = a.sumar(&b).expect_err("monedas distintas, debe fallar");
        assert_eq!(err.campo, Campo::Moneda);
    }

    #[test]
    fn restar_misma_moneda() {
        let ves = Moneda::por_codigo("VES").unwrap();
        let a = Monto::new(dec!(100.00), ves.clone());
        let b = Monto::new(dec!(30.50), ves);
        let resultado = a.restar(&b).expect("misma moneda, debe funcionar");
        assert_eq!(resultado.cantidad(), dec!(69.50));
    }

    #[test]
    fn restar_monedas_distintas_falla() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let usdt = Moneda::por_codigo("USDT").unwrap();
        let a = Monto::new(dec!(50), usd);
        let b = Monto::new(dec!(20), usdt);
        let err = a.restar(&b).expect_err("monedas distintas, debe fallar");
        assert_eq!(err.campo, Campo::Moneda);
    }

    // ── Cuenta::crear — caso exitoso ──

    #[test]
    fn crear_cuenta_valida() {
        let (cuenta, evento) = Cuenta::crear(
            "id-1".into(),
            "Mi Efectivo",
            TipoCuenta::Efectivo,
            "VES",
            "1250.50",
            &[],
        )
        .expect("debe crear sin errores");

        assert_eq!(cuenta.id(), "id-1");
        assert_eq!(cuenta.nombre(), "Mi Efectivo");
        assert_eq!(cuenta.tipo(), TipoCuenta::Efectivo);
        assert_eq!(cuenta.saldo().cantidad(), dec!(1250.50));
        assert_eq!(cuenta.saldo().moneda().codigo, "VES");

        assert_eq!(evento.id, "id-1");
        assert_eq!(evento.moneda.codigo, "VES");
        assert_eq!(evento.saldo_inicial, dec!(1250.50));
    }

    #[test]
    fn crear_cuenta_con_saldo_cero() {
        let resultado = Cuenta::crear("id-2".into(), "Banco", TipoCuenta::Banco, "USD", "0", &[]);
        assert!(resultado.is_ok());
    }

    // ── Cuenta::crear — validación: nombre vacío ──

    #[test]
    fn nombre_vacio_falla() {
        let err = Cuenta::crear("id-3".into(), "", TipoCuenta::Efectivo, "USD", "100", &[])
            .expect_err("nombre vacío debe fallar");

        assert!(err.iter().any(|e| e.campo == Campo::Nombre));
    }

    #[test]
    fn nombre_solo_espacios_falla() {
        let err = Cuenta::crear(
            "id-4".into(),
            "   ",
            TipoCuenta::Efectivo,
            "USD",
            "100",
            &[],
        )
        .expect_err("nombre de espacios debe fallar");

        assert!(err.iter().any(|e| e.campo == Campo::Nombre));
    }

    // ── Cuenta::crear — validación: nombre repetido ──

    #[test]
    fn nombre_repetido_falla() {
        let existentes = vec!["Mi Cuenta".to_string()];
        let err = Cuenta::crear(
            "id-5".into(),
            "mi cuenta",
            TipoCuenta::Banco,
            "EUR",
            "50",
            &existentes,
        )
        .expect_err("nombre repetido debe fallar");

        assert!(
            err.iter()
                .any(|e| e.campo == Campo::Nombre && e.mensaje.contains("Ya existe"))
        );
    }

    // ── Cuenta::crear — validación: moneda inválida ──

    #[test]
    fn moneda_invalida_falla() {
        let err = Cuenta::crear(
            "id-6".into(),
            "Cripto",
            TipoCuenta::BilleteraDigital,
            "BTC",
            "1.00",
            &[],
        )
        .expect_err("moneda inválida debe fallar");

        assert!(err.iter().any(|e| e.campo == Campo::Moneda));
    }

    // ── Cuenta::crear — validación: saldo negativo ──

    #[test]
    fn saldo_negativo_falla() {
        let err = Cuenta::crear(
            "id-7".into(),
            "Efectivo",
            TipoCuenta::Efectivo,
            "USD",
            "-10.00",
            &[],
        )
        .expect_err("saldo negativo debe fallar");

        assert!(
            err.iter()
                .any(|e| e.campo == Campo::SaldoInicial && e.mensaje.contains("negativo"))
        );
    }

    // ── Cuenta::crear — validación: demasiados decimales ──

    #[test]
    fn saldo_con_demasiados_decimales_falla() {
        let err = Cuenta::crear(
            "id-8".into(),
            "Ahorro",
            TipoCuenta::Banco,
            "EUR",
            "100.123",
            &[],
        )
        .expect_err("demasiados decimales debe fallar");

        assert!(
            err.iter()
                .any(|e| e.campo == Campo::SaldoInicial && e.mensaje.contains("decimales"))
        );
    }

    // ── Cuenta::crear — validación: saldo no numérico ──

    #[test]
    fn saldo_no_numerico_falla() {
        let err = Cuenta::crear(
            "id-9".into(),
            "Test",
            TipoCuenta::Efectivo,
            "USD",
            "abc",
            &[],
        )
        .expect_err("saldo no numérico debe fallar");

        assert!(err.iter().any(|e| e.campo == Campo::SaldoInicial));
    }

    // ── Cuenta::crear — acumulación de errores ──

    #[test]
    fn multiples_errores_se_acumulan() {
        let err = Cuenta::crear(
            "id-10".into(),
            "", // nombre vacío
            TipoCuenta::Efectivo,
            "BTC",    // moneda inválida
            "-5.999", // negativo + demasiados decimales (pero moneda inválida, no se chequea decimales)
            &[],
        )
        .expect_err("múltiples errores deben acumularse");

        // Al menos: nombre vacío, moneda inválida, saldo negativo
        assert!(
            err.len() >= 2,
            "debe haber al menos 2 errores, hay {}",
            err.len()
        );
        assert!(err.iter().any(|e| e.campo == Campo::Nombre));
        assert!(err.iter().any(|e| e.campo == Campo::Moneda));
    }

    // ── INV-18: la moneda de una cuenta no puede cambiar ──

    /// Código del dominio sin el módulo de pruebas.
    fn codigo_de_produccion() -> &'static str {
        let fuente = include_str!("lib.rs");
        &fuente[..fuente.find("#[cfg(test)]").expect("módulo de pruebas")]
    }

    /// Texto del bloque `impl Cuenta { ... }` (sin llaves anidadas mal balanceadas).
    fn impl_de_cuenta() -> String {
        let src = codigo_de_produccion();
        let inicio = src.find("impl Cuenta {").expect("impl Cuenta");
        let mut prof = 0i32;
        let mut fin = inicio;
        for (i, c) in src[inicio..].char_indices() {
            match c {
                '{' => prof += 1,
                '}' => {
                    prof -= 1;
                    if prof == 0 {
                        fin = inicio + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        src[inicio..=fin].to_string()
    }

    #[test]
    fn inv18_cuenta_no_tiene_metodos_mutables() {
        let bloque = impl_de_cuenta();
        assert!(
            !bloque.contains("&mut self") && !bloque.contains("mut self"),
            "Cuenta no puede exponer operaciones que modifiquen su estado (INV-18)"
        );
    }

    #[test]
    fn inv18_campos_de_cuenta_y_monto_son_privados() {
        let src = codigo_de_produccion();
        for tipo in ["pub struct Cuenta {", "pub struct Monto {"] {
            let inicio = src.find(tipo).expect("struct") + tipo.len();
            let cuerpo = &src[inicio..inicio + src[inicio..].find('}').unwrap()];
            assert!(
                !cuerpo.contains("pub "),
                "{tipo} no debe tener campos públicos"
            );
        }
    }

    #[test]
    fn inv18_la_moneda_del_saldo_es_la_del_evento_y_no_cambia() {
        let (cuenta, evento) = Cuenta::crear(
            "id-11".into(),
            "Fija",
            TipoCuenta::Efectivo,
            "USD",
            "0",
            &[],
        )
        .unwrap();
        let copia = cuenta.clone();
        assert_eq!(cuenta.saldo().moneda().codigo, "USD");
        assert_eq!(evento.moneda.codigo, "USD");
        assert_eq!(copia.saldo().moneda(), cuenta.saldo().moneda());
    }

    // ── INV-01: monto = par indivisible cantidad + moneda ──

    #[test]
    fn montos_con_igual_cantidad_y_distinta_moneda_no_son_iguales() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let eur = Moneda::por_codigo("EUR").unwrap();
        assert_ne!(Monto::new(dec!(10), usd.clone()), Monto::new(dec!(10), eur));
        assert_eq!(
            Monto::new(dec!(10), usd.clone()),
            Monto::new(dec!(10.00), usd)
        );
    }

    #[test]
    fn monto_de_cuenta_conserva_su_moneda_en_las_operaciones() {
        let (cuenta, evento) =
            Cuenta::crear("id-m".into(), "Dólares", TipoCuenta::Banco, "usd", "5", &[]).unwrap();
        assert_eq!(cuenta.saldo().moneda(), &evento.moneda);
        let suma = cuenta.saldo().sumar(cuenta.saldo()).unwrap();
        assert_eq!(suma.moneda().codigo, "USD");
        assert_eq!(suma.cantidad(), dec!(10));
    }

    // ── Normalización ──

    #[test]
    fn por_codigo_ignora_espacios() {
        assert_eq!(Moneda::por_codigo(" usd ").unwrap().codigo, "USD");
    }

    #[test]
    fn saldo_con_ceros_finales_es_valido() {
        let (cuenta, _) = Cuenta::crear(
            "id-z".into(),
            "Ceros",
            TipoCuenta::Banco,
            "USD",
            "10.500",
            &[],
        )
        .unwrap();
        assert_eq!(cuenta.saldo().cantidad(), dec!(10.50));
        assert_eq!(cuenta.saldo().cantidad().scale(), 2);
    }

    #[test]
    fn saldo_entero_se_guarda_con_los_decimales_de_la_moneda() {
        let (_, evento) = Cuenta::crear(
            "id-e".into(),
            "Entero",
            TipoCuenta::Efectivo,
            "VES",
            "300",
            &[],
        )
        .unwrap();
        assert_eq!(evento.saldo_inicial.to_string(), "300.00");
    }

    #[test]
    fn saldo_cero_negativo_se_normaliza() {
        let (_, evento) = Cuenta::crear(
            "id-n".into(),
            "Cero",
            TipoCuenta::Efectivo,
            "VES",
            "-0.00",
            &[],
        )
        .unwrap();
        assert_eq!(evento.saldo_inicial.to_string(), "0.00");
    }

    #[test]
    fn diez_con_quinientos_cincuenta_y_cinco_milesimas_se_rechaza() {
        let err = Cuenta::crear(
            "id-d".into(),
            "Dec",
            TipoCuenta::Banco,
            "USD",
            "10.555",
            &[],
        )
        .unwrap_err();
        assert!(err.iter().any(|e| e.campo == Campo::SaldoInicial));
    }

    // ── Pruebas de Fecha ──

    #[test]
    fn fecha_valida_pasa() {
        assert!(validar_fecha("2026-10-19").is_ok());
        assert!(validar_fecha("2024-02-29").is_ok()); // año bisiesto
        assert!(validar_fecha("2026-04-30").is_ok());
    }

    #[test]
    fn fecha_invalida_falla() {
        assert!(validar_fecha("2026-02-29").is_err()); // 2026 no es bisiesto
        assert!(validar_fecha("2026-02-30").is_err());
        assert!(validar_fecha("2026-04-31").is_err());
        assert!(validar_fecha("2026-13-01").is_err());
        assert!(validar_fecha("fecha-invalida").is_err());
        assert!(validar_fecha("2026/10/19").is_err());
    }

    // ── Pruebas de Categorías ──

    #[test]
    fn categorias_de_fabrica_tiene_once_elementos() {
        let cats = categorias_de_fabrica();
        assert_eq!(cats.len(), 11);
        let gastos = cats
            .iter()
            .filter(|c| c.tipo == TipoMovimiento::Gasto)
            .count();
        let ingresos = cats
            .iter()
            .filter(|c| c.tipo == TipoMovimiento::Ingreso)
            .count();
        assert_eq!(gastos, 7);
        assert_eq!(ingresos, 4);
        assert!(cats.iter().all(|c| c.de_fabrica));
        assert!(cats.iter().any(|c| c.id == "gasto:alimentacion"));
    }

    #[test]
    fn crear_categoria_valida() {
        let cats = categorias_de_fabrica();
        let (cat, ev) = crear_categoria("cat-1".into(), "Mascotas", TipoMovimiento::Gasto, &cats)
            .expect("debe crear categoría válida");

        assert_eq!(cat.nombre, "Mascotas");
        assert_eq!(cat.tipo, TipoMovimiento::Gasto);
        assert!(!cat.de_fabrica);
        assert_eq!(ev.id_categoria, "cat-1");
        assert_eq!(ev.nombre, "Mascotas");
        assert_eq!(ev.tipo, TipoMovimiento::Gasto);
    }

    #[test]
    fn crear_categoria_nombre_vacio_falla() {
        let err = crear_categoria("cat-2".into(), "   ", TipoMovimiento::Gasto, &[])
            .expect_err("nombre vacío debe fallar");
        assert!(err.iter().any(|e| e.campo == Campo::Nombre));
    }

    #[test]
    fn crear_categoria_repetida_sin_distinguir_mayusculas_falla() {
        let cats = categorias_de_fabrica();
        let err = crear_categoria("cat-3".into(), "alimentación", TipoMovimiento::Gasto, &cats)
            .expect_err("categoría repetida debe fallar");
        assert!(err.iter().any(|e| e.campo == Campo::Nombre));
    }

    #[test]
    fn crear_categoria_mismo_nombre_distinto_tipo_pasa() {
        let cats = categorias_de_fabrica();
        // "Otros" existe en Ingreso, permitimos crear "Otros" en Gasto
        let res = crear_categoria("cat-4".into(), "Otros", TipoMovimiento::Gasto, &cats);
        assert!(res.is_ok());
    }

    // ── Pruebas de Registrar Movimiento ──

    #[test]
    fn registrar_movimiento_valido() {
        let cats = categorias_de_fabrica();
        let (mov, ev) = registrar_movimiento(
            "mov-1".into(),
            "cuenta-1".into(),
            TipoMovimiento::Gasto,
            "45.50",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            Some("Almuerzo".into()),
            &cats,
        )
        .expect("debe registrar movimiento");

        assert_eq!(mov.id(), "mov-1");
        assert_eq!(mov.monto().cantidad(), dec!(45.50));
        assert_eq!(mov.monto().moneda().codigo, "USD");
        assert_eq!(mov.fecha(), "2026-10-19");
        assert_eq!(mov.estado(), &EstadoMovimiento::Vigente);
        assert_eq!(mov.nota(), Some("Almuerzo"));

        assert_eq!(ev.id_movimiento, "mov-1");
        assert_eq!(ev.id_cuenta, "cuenta-1");
        assert_eq!(ev.monto, dec!(45.50));
        assert_eq!(ev.moneda.codigo, "USD");
    }

    #[test]
    fn registrar_movimiento_monto_cero_o_negativo_falla() {
        let cats = categorias_de_fabrica();
        let err_cero = registrar_movimiento(
            "mov-2".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "0.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(err_cero.iter().any(|e| e.campo == Campo::Monto));

        let err_neg = registrar_movimiento(
            "mov-3".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "-15.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(err_neg.iter().any(|e| e.campo == Campo::Monto));
    }

    #[test]
    fn registrar_movimiento_demasiados_decimales_falla() {
        let cats = categorias_de_fabrica();
        let err = registrar_movimiento(
            "mov-4".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "10.123",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(err.iter().any(|e| e.campo == Campo::Monto));
    }

    #[test]
    fn registrar_movimiento_categoria_inexistente_o_incompatible_falla() {
        let cats = categorias_de_fabrica();
        let err_inex = registrar_movimiento(
            "mov-5".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "10.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "categoria_fantasma",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(err_inex.iter().any(|e| e.campo == Campo::Categoria));

        let err_incomp = registrar_movimiento(
            "mov-6".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "10.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "ingreso:salario",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(err_incomp.iter().any(|e| e.campo == Campo::Categoria));
    }

    #[test]
    fn registrar_movimiento_fecha_futura_falla() {
        let cats = categorias_de_fabrica();
        let err = registrar_movimiento(
            "mov-7".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "10.00",
            "USD",
            "2026-10-25",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.campo == Campo::Fecha && e.mensaje.contains("futura"))
        );
    }

    // ── Pruebas de Corregir Movimiento ──

    #[test]
    fn corregir_movimiento_secuencia_acordada_con_ajuste_mas_cuatrocientos_cincuenta() {
        let cats = categorias_de_fabrica();
        // Caso Ingreso: 1.000,00 -> 1.800,00 -> 1.300,00 -> 1.750,00 (ajuste final de +450,00)
        let (mov_orig, _) = registrar_movimiento(
            "mov-orig".into(),
            "c1".into(),
            TipoMovimiento::Ingreso,
            "1000.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "ingreso:salario",
            None,
            &cats,
        )
        .unwrap();

        let movimientos = vec![mov_orig];
        let mut ajustes: Vec<Ajuste> = Vec::new();

        // 1ª corrección: a 1.800,00. Vigente = 1.000,00. Efecto = 1800 - 1000 = +800,00
        let (ajuste_1, ev_1) = corregir_movimiento(
            "ajuste-1".into(),
            "mov-orig",
            "1800.00",
            &movimientos,
            &ajustes,
        )
        .expect("primera corrección");
        assert_eq!(ajuste_1.efecto, dec!(800.00));
        assert_eq!(ajuste_1.monto_correcto, dec!(1800.00));
        assert_eq!(ev_1.efecto, dec!(800.00));
        ajustes.push(ajuste_1);

        // 2ª corrección: a 1.300,00. Vigente = 1.800,00. Efecto = 1300 - 1800 = -500,00
        let (ajuste_2, ev_2) = corregir_movimiento(
            "ajuste-2".into(),
            "mov-orig",
            "1300.00",
            &movimientos,
            &ajustes,
        )
        .expect("segunda corrección");
        assert_eq!(ajuste_2.efecto, dec!(-500.00));
        assert_eq!(ajuste_2.monto_correcto, dec!(1300.00));
        assert_eq!(ev_2.efecto, dec!(-500.00));
        ajustes.push(ajuste_2);

        // 3ª corrección: a 1.750,00. Vigente = 1.300,00. Efecto = 1750 - 1300 = +450,00
        let (ajuste_3, ev_3) = corregir_movimiento(
            "ajuste-3".into(),
            "mov-orig",
            "1750.00",
            &movimientos,
            &ajustes,
        )
        .expect("tercera corrección con ajuste de +450,00");
        assert_eq!(ajuste_3.efecto, dec!(450.00));
        assert_eq!(ajuste_3.monto_correcto, dec!(1750.00));
        assert_eq!(ev_3.efecto, dec!(450.00));
        ajustes.push(ajuste_3);

        // Invariante de redundancia verificada: monto_correcto del último ajuste coincide con el vigente
        assert_eq!(ajustes.last().unwrap().monto_correcto, dec!(1750.00));
    }

    #[test]
    fn corregir_gasto_disminucion_aumenta_saldo() {
        let cats = categorias_de_fabrica();
        // Gasto original de 500, corregido a 50 -> efecto = 500 - 50 = +450 (devuelve 450 a la cuenta)
        let (mov_orig, _) = registrar_movimiento(
            "gasto-1".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "500.00",
            "VES",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap();

        let (ajuste, ev) =
            corregir_movimiento("ajuste-g".into(), "gasto-1", "50.00", &[mov_orig], &[])
                .expect("corrección de gasto");

        assert_eq!(ajuste.efecto, dec!(450.00));
        assert_eq!(ev.efecto, dec!(450.00));
        assert_eq!(ajuste.monto_correcto, dec!(50.00));
    }

    #[test]
    fn corregir_movimiento_monto_cero_es_valido() {
        let cats = categorias_de_fabrica();
        let (mov_orig, _) = registrar_movimiento(
            "mov-cero".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "100.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:transporte",
            None,
            &cats,
        )
        .unwrap();

        let (ajuste, _) = corregir_movimiento("aj-0".into(), "mov-cero", "0.00", &[mov_orig], &[])
            .expect("monto correcto 0 es válido");

        assert_eq!(ajuste.monto_correcto, dec!(0.00));
        assert_eq!(ajuste.efecto, dec!(100.00));
    }

    #[test]
    fn corregir_movimiento_igual_al_vigente_falla() {
        let cats = categorias_de_fabrica();
        let (mov_orig, _) = registrar_movimiento(
            "mov-eq".into(),
            "c1".into(),
            TipoMovimiento::Gasto,
            "100.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:transporte",
            None,
            &cats,
        )
        .unwrap();

        let err = corregir_movimiento("aj-eq".into(), "mov-eq", "100.00", &[mov_orig], &[])
            .expect_err("monto igual al vigente debe fallar");

        assert!(err.iter().any(|e| e.campo == Campo::Monto));
    }

    // ── Pruebas de Saldo y Composición ──

    #[test]
    fn calcular_saldo_y_composicion_correcto() {
        let (cuenta, _) = Cuenta::crear(
            "cta-1".into(),
            "Principal",
            TipoCuenta::Banco,
            "USD",
            "1000.00",
            &[],
        )
        .unwrap();

        let cats = categorias_de_fabrica();
        let (ingreso, _) = registrar_movimiento(
            "m1".into(),
            "cta-1".into(),
            TipoMovimiento::Ingreso,
            "500.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "ingreso:salario",
            None,
            &cats,
        )
        .unwrap();

        let (gasto, _) = registrar_movimiento(
            "m2".into(),
            "cta-1".into(),
            TipoMovimiento::Gasto,
            "200.00",
            "USD",
            "2026-10-19",
            "2026-10-19",
            "gasto:alimentacion",
            None,
            &cats,
        )
        .unwrap();

        let (ajuste, _) = corregir_movimiento(
            "aj1".into(),
            "m2",
            "150.00", // gasto bajó 50 -> efecto +50
            std::slice::from_ref(&gasto),
            &[],
        )
        .unwrap();

        let comp = calcular_saldo_y_composicion(&cuenta, &[ingreso, gasto], &[ajuste])
            .expect("cálculo de saldo");

        assert_eq!(comp.saldo_inicial.cantidad(), dec!(1000.00));
        assert_eq!(comp.total_ingresos.cantidad(), dec!(500.00));
        assert_eq!(comp.total_gastos.cantidad(), dec!(200.00));
        assert_eq!(comp.total_ajustes.cantidad(), dec!(50.00));
        // saldo_actual = 1000 + 500 - 200 + 50 = 1350
        assert_eq!(comp.saldo_actual.cantidad(), dec!(1350.00));
    }

    #[test]
    fn saldo_resultante_resta_gasto_hipotetico() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let saldo_actual = Monto::new(dec!(500.00), usd.clone());
        let gasto_hipotetico = Monto::new(dec!(150.00), usd.clone());

        let res = saldo_resultante(&saldo_actual, &gasto_hipotetico).unwrap();
        assert_eq!(res.cantidad(), dec!(350.00));
        assert_eq!(res.moneda().codigo, "USD");
    }

    #[test]
    fn saldo_resultante_puede_quedar_negativo() {
        let usd = Moneda::por_codigo("USD").unwrap();
        let saldo_actual = Monto::new(dec!(100.00), usd.clone());
        let gasto_mayor = Monto::new(dec!(250.00), usd);

        let res = saldo_resultante(&saldo_actual, &gasto_mayor).unwrap();
        assert_eq!(res.cantidad(), dec!(-150.00));
    }

    // ── Pruebas de Propiedades (Generador Propio Determinista) ──

    struct GeneradorRng {
        estado: u64,
    }

    impl GeneradorRng {
        fn con_semilla(semilla: u64) -> Self {
            Self { estado: semilla }
        }

        fn siguiente_u32(&mut self) -> u32 {
            self.estado = self
                .estado
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            (self.estado >> 32) as u32
        }

        fn rango(&mut self, min: i64, max: i64) -> i64 {
            let span = (max - min + 1) as u64;
            min + (self.siguiente_u32() as u64 % span) as i64
        }
    }

    #[test]
    fn propiedad_conservacion_del_saldo_para_secuencias_arbitrarias() {
        let mut rng = GeneradorRng::con_semilla(20261019);
        let usd = Moneda::por_codigo("USD").unwrap();

        for _ in 0..100 {
            let saldo_inicial_centavos = rng.rango(0, 100_000);
            let saldo_inicial = Decimal::new(saldo_inicial_centavos, 2);
            let (cuenta, _) = Cuenta::crear(
                "cta-prop".into(),
                "Prop",
                TipoCuenta::Banco,
                "USD",
                &saldo_inicial.to_string(),
                &[],
            )
            .unwrap();

            let mut movimientos = Vec::new();
            let mut ajustes = Vec::new();
            let mut suma_ingresos = Decimal::ZERO;
            let mut suma_gastos = Decimal::ZERO;
            let mut suma_efectos = Decimal::ZERO;

            let n_movs = rng.rango(1, 10);
            for i in 0..n_movs {
                let centavos = rng.rango(1, 50_000);
                let monto = Decimal::new(centavos, 2);
                let es_ingreso = rng.rango(0, 1) == 1;
                let tipo = if es_ingreso {
                    suma_ingresos += monto;
                    TipoMovimiento::Ingreso
                } else {
                    suma_gastos += monto;
                    TipoMovimiento::Gasto
                };

                let m = Movimiento {
                    id: format!("m-{}", i),
                    id_cuenta: "cta-prop".into(),
                    tipo,
                    monto: Monto::new(monto, usd.clone()),
                    fecha: "2026-10-19".into(),
                    id_categoria: "cat".into(),
                    nota: None,
                    estado: EstadoMovimiento::Vigente,
                };
                movimientos.push(m);
            }

            // Aplicar ajustes aleatorios sobre algunos movimientos
            let n_ajustes = rng.rango(0, 5);
            for j in 0..n_ajustes {
                let idx = rng.rango(0, movimientos.len() as i64 - 1) as usize;
                let orig = &movimientos[idx];
                let centavos_corr = rng.rango(0, 60_000);
                let corr = Decimal::new(centavos_corr, 2);
                let efecto = match orig.tipo {
                    TipoMovimiento::Gasto => orig.monto.cantidad() - corr,
                    TipoMovimiento::Ingreso => corr - orig.monto.cantidad(),
                };
                suma_efectos += efecto;
                ajustes.push(Ajuste {
                    id_ajuste: format!("aj-{}", j),
                    id_movimiento_original: orig.id.clone(),
                    monto_correcto: corr,
                    efecto,
                    moneda: usd.clone(),
                });
            }

            let comp = calcular_saldo_y_composicion(&cuenta, &movimientos, &ajustes)
                .expect("cálculo de saldo en propiedad");

            let saldo_esperado = saldo_inicial + suma_ingresos - suma_gastos + suma_efectos;
            assert_eq!(comp.saldo_actual.cantidad(), saldo_esperado);
            assert_eq!(comp.total_ingresos.cantidad(), suma_ingresos);
            assert_eq!(comp.total_gastos.cantidad(), suma_gastos);
            assert_eq!(comp.total_ajustes.cantidad(), suma_efectos);
        }
    }

    #[test]
    fn propiedad_aritmetica_decimal_nunca_pierde_centavos() {
        let mut rng = GeneradorRng::con_semilla(987654321);
        let usd = Moneda::por_codigo("USD").unwrap();

        for _ in 0..200 {
            let c1 = rng.rango(-500_000, 500_000);
            let c2 = rng.rango(-500_000, 500_000);
            let m1 = Monto::new(Decimal::new(c1, 2), usd.clone());
            let m2 = Monto::new(Decimal::new(c2, 2), usd.clone());

            let suma = m1.sumar(&m2).unwrap();
            let esperado_suma = Decimal::new(c1 + c2, 2);
            assert_eq!(suma.cantidad(), esperado_suma);

            let resta = m1.restar(&m2).unwrap();
            let esperado_resta = Decimal::new(c1 - c2, 2);
            assert_eq!(resta.cantidad(), esperado_resta);
        }
    }

    #[test]
    fn propiedad_coherencia_entre_efectos_acumulados_y_montos_vigentes() {
        let mut rng = GeneradorRng::con_semilla(456789123);
        let usd = Moneda::por_codigo("USD").unwrap();

        for _ in 0..100 {
            let saldo_inicial_centavos = rng.rango(0, 100_000);
            let saldo_inicial = Decimal::new(saldo_inicial_centavos, 2);
            let (cuenta, _) = Cuenta::crear(
                "cta-coh".into(),
                "Coh",
                TipoCuenta::Banco,
                "USD",
                &saldo_inicial.to_string(),
                &[],
            )
            .unwrap();

            let mut movimientos = Vec::new();
            let mut ajustes: Vec<Ajuste> = Vec::new();

            // 1. Crear entre 3 y 8 movimientos arbitrarios
            let n_movs = rng.rango(3, 8);
            for i in 0..n_movs {
                let centavos = rng.rango(1, 30_000);
                let monto = Decimal::new(centavos, 2);
                let es_ingreso = rng.rango(0, 1) == 1;
                let tipo = if es_ingreso {
                    TipoMovimiento::Ingreso
                } else {
                    TipoMovimiento::Gasto
                };

                let m = Movimiento {
                    id: format!("m-coh-{}", i),
                    id_cuenta: "cta-coh".into(),
                    tipo,
                    monto: Monto::new(monto, usd.clone()),
                    fecha: "2026-10-19".into(),
                    id_categoria: "cat".into(),
                    nota: None,
                    estado: EstadoMovimiento::Vigente,
                };
                movimientos.push(m);
            }

            // 2. Realizar múltiples rondas de correcciones
            let n_corrs = rng.rango(1, 10);
            for j in 0..n_corrs {
                let idx = rng.rango(0, movimientos.len() as i64 - 1) as usize;
                let orig = &movimientos[idx];

                // Calcular el vigente actual antes de corregir
                let ajustes_este_mov: Vec<&Ajuste> = ajustes
                    .iter()
                    .filter(|a| a.id_movimiento_original == orig.id)
                    .collect();
                let suma_efectos_prev: Decimal = ajustes_este_mov.iter().map(|a| a.efecto).sum();
                let vigente_previo = match orig.tipo {
                    TipoMovimiento::Gasto => orig.monto.cantidad() - suma_efectos_prev,
                    TipoMovimiento::Ingreso => orig.monto.cantidad() + suma_efectos_prev,
                };

                let centavos_corr = rng.rango(0, 40_000);
                let nuevo_corr = Decimal::new(centavos_corr, 2);
                if nuevo_corr == vigente_previo {
                    continue;
                }

                let (ajuste, _) = corregir_movimiento(
                    format!("aj-coh-{}", j),
                    &orig.id,
                    &nuevo_corr.to_string(),
                    &movimientos,
                    &ajustes,
                )
                .expect("corrección válida en propiedad");

                ajustes.push(ajuste);

                // Exigencia 1: tras cada corrección, recalcular el vigente desde todos los ajustes y exigir que sea igual al monto_correcto
                let ajustes_act: Vec<&Ajuste> = ajustes
                    .iter()
                    .filter(|a| a.id_movimiento_original == orig.id)
                    .collect();
                let suma_efectos_act: Decimal = ajustes_act.iter().map(|a| a.efecto).sum();
                let vigente_recalculado = match orig.tipo {
                    TipoMovimiento::Gasto => orig.monto.cantidad() - suma_efectos_act,
                    TipoMovimiento::Ingreso => orig.monto.cantidad() + suma_efectos_act,
                };
                assert_eq!(
                    vigente_recalculado, nuevo_corr,
                    "El valor vigente recalculado debe ser exactamente igual al monto_correcto"
                );
            }

            // Exigencia 2: comprobar que el saldo calculado con los efectos coincide con el calculado con los montos vigentes finales
            let comp = calcular_saldo_y_composicion(&cuenta, &movimientos, &ajustes)
                .expect("cálculo con efectos");
            let saldo_con_efectos = comp.saldo_actual.cantidad();

            // Saldo calculado directamente con los montos vigentes finales de cada movimiento
            let mut suma_ingresos_vigentes = Decimal::ZERO;
            let mut suma_gastos_vigentes = Decimal::ZERO;

            for m in &movimientos {
                let ajustes_m: Vec<&Ajuste> = ajustes
                    .iter()
                    .filter(|a| a.id_movimiento_original == m.id)
                    .collect();
                let monto_vigente = if let Some(ultimo) = ajustes_m.last() {
                    ultimo.monto_correcto
                } else {
                    m.monto.cantidad()
                };

                match m.tipo {
                    TipoMovimiento::Ingreso => suma_ingresos_vigentes += monto_vigente,
                    TipoMovimiento::Gasto => suma_gastos_vigentes += monto_vigente,
                }
            }

            let saldo_con_montos_vigentes =
                saldo_inicial + suma_ingresos_vigentes - suma_gastos_vigentes;

            assert_eq!(
                saldo_con_efectos, saldo_con_montos_vigentes,
                "El saldo derivado de los eventos/efectos debe coincidir exactamente con el saldo calculado con los montos vigentes"
            );
        }
    }
}
