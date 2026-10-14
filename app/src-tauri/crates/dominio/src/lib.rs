//! Reglas de negocio. No depende de la base de datos, la red ni el framework.
//!
//! Tipos públicos: [`Moneda`], [`Monto`], [`TipoCuenta`], [`Cuenta`], [`CuentaCreada`],
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
}

/// Error de dominio asociado a un campo específico del formulario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDominio {
    pub campo: Campo,
    pub mensaje: String,
}

impl ErrorDominio {
    /// Atajo para construir un error.
    fn nuevo(campo: Campo, mensaje: impl Into<String>) -> Self {
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
                nombre: "Bolívar digital".into(),
                simbolo: "Bs.D".into(),
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
                nombre: "Tether".into(),
                simbolo: "₮".into(),
                decimales: 2,
            },
        ]
    }

    /// Busca una moneda por código (insensible a mayúsculas/minúsculas).
    pub fn por_codigo(codigo: &str) -> Option<Moneda> {
        let codigo_upper = codigo.to_uppercase();
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
    fn new(cantidad: Decimal, moneda: Moneda) -> Self {
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
                    let escala = d.scale();
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
        let cantidad = saldo_decimal.expect("saldo validado");

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

    // ── Inmutabilidad de moneda ──

    #[test]
    fn cuenta_no_expone_setter_de_moneda() {
        // Este test es conceptual: la compilación garantiza que no hay
        // forma de modificar la moneda de una cuenta porque los campos
        // son privados y no existe ningún método &mut self que la cambie.
        let (cuenta, _) = Cuenta::crear(
            "id-11".into(),
            "Fija",
            TipoCuenta::Efectivo,
            "USD",
            "0",
            &[],
        )
        .unwrap();

        // Solo podemos leer, no escribir.
        assert_eq!(cuenta.saldo().moneda().codigo, "USD");
    }
}
