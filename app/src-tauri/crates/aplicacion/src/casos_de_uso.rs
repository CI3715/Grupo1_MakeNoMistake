//! Casos de uso para la gestión de cuentas y consulta del catálogo de monedas.
//!
//! Coordina la validación de reglas de negocio delegando en `dominio`, la
//! generación y persistencia de eventos inmutables mediante los puertos de salida,
//! y la proyección de lecturas y subtotales por moneda.

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

use dominio::{Campo, Cuenta, ErrorDominio, Moneda, TipoCuenta};

use crate::codec;
use crate::errores::{ErrorAplicacion, ErrorValidacion};
use crate::puertos::{
    CuentaProyectada, DispositivoOrigen, EventoRegistrado, GeneradorIdentificadores, Reloj,
    RepositorioEventos,
};
use crate::vistas::{CuentaListada, ListaCuentas, MonedaDto, Subtotal};

/// Identificador de tipo para los eventos de creación de cuenta.
pub const TIPO_CUENTA_CREADA: &str = "CuentaCreada";

/// Datos de entrada requeridos para registrar una nueva cuenta.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaCuenta {
    /// Nombre de la cuenta (obligatorio y único sin distinguir mayúsculas).
    pub nombre: String,
    /// Tipo de cuenta (`"efectivo"`, `"banco"` o `"billetera_digital"`).
    pub tipo: String,
    /// Código de la moneda del catálogo (`"VES"`, `"USD"`, `"EUR"`, `"USDT"`).
    pub moneda: String,
    /// Saldo inicial expresado como cadena decimal (por ejemplo, `"1250.50"`).
    pub saldo_inicial: String,
}

/// Servicio de aplicación que agrupa los casos de uso e inyecta los puertos requeridos.
pub struct CasosDeUso {
    repositorio: Box<dyn RepositorioEventos>,
    reloj: Box<dyn Reloj>,
    ids: Box<dyn GeneradorIdentificadores>,
    dispositivo: Box<dyn DispositivoOrigen>,
}

impl CasosDeUso {
    /// Inicializa el servicio de casos de uso con los adaptadores proporcionados.
    pub fn nuevo(
        repositorio: Box<dyn RepositorioEventos>,
        reloj: Box<dyn Reloj>,
        ids: Box<dyn GeneradorIdentificadores>,
        dispositivo: Box<dyn DispositivoOrigen>,
    ) -> CasosDeUso {
        CasosDeUso {
            repositorio,
            reloj,
            ids,
            dispositivo,
        }
    }

    /// Devuelve el catálogo de monedas soportadas por el sistema.
    pub fn listar_monedas(&self) -> Vec<MonedaDto> {
        Moneda::catalogo()
            .into_iter()
            .map(|m| MonedaDto {
                codigo: m.codigo,
                nombre: m.nombre,
                simbolo: m.simbolo,
                decimales: m.decimales as u8,
            })
            .collect()
    }

    /// Lee el listado de cuentas directamente desde la proyección materializada
    /// y calcula los subtotales agrupados por cada moneda sin mezclar divisas distintas.
    ///
    /// No reproyecta eventos: la proyección ya se mantiene al día en la misma
    /// transacción que guarda cada evento.
    pub fn listar_cuentas(&self) -> Result<ListaCuentas, ErrorAplicacion> {
        let proyectadas = self.repositorio.listar_cuentas()?;

        let mut cuentas = Vec::new();
        for c in &proyectadas {
            cuentas.push(CuentaListada {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                tipo: c.tipo.clone(),
                moneda: c.moneda.clone(),
                saldo_actual: c.saldo_actual.clone(),
            });
        }

        let subtotales = calcular_subtotales(&proyectadas)?;
        Ok(ListaCuentas {
            cuentas,
            subtotales,
        })
    }

    /// Valida los datos de entrada contra las reglas del dominio y, si son válidos,
    /// genera y persiste el evento `CuentaCreada` correspondiente.
    ///
    /// Si una o más validaciones fallan, acumula y devuelve todos los errores
    /// detectados sin persistir cambios ni consumir identificadores.
    pub fn crear_cuenta(&mut self, entrada: NuevaCuenta) -> Result<CuentaListada, ErrorAplicacion> {
        let proyectadas = self.repositorio.listar_cuentas()?;
        let nombres_existentes: Vec<String> =
            proyectadas.iter().map(|c| c.nombre.clone()).collect();

        let tipo_resultado = codec::texto_a_tipo(&entrada.tipo);
        let tipo_para_validar = tipo_resultado.clone().unwrap_or(TipoCuenta::Efectivo);

        // Ejecuta la validación de reglas de dominio antes de generar identificadores.
        let validacion_dominio = Cuenta::crear(
            String::new(),
            &entrada.nombre,
            tipo_para_validar,
            &entrada.moneda,
            &entrada.saldo_inicial,
            &nombres_existentes,
        );

        let mut errores_validacion = match validacion_dominio {
            Ok(_) => Vec::new(),
            Err(errs) => convertir_errores(errs),
        };

        if tipo_resultado.is_err() {
            let pos = errores_validacion
                .iter()
                .position(|e| e.campo.as_deref() != Some("nombre"))
                .unwrap_or(errores_validacion.len());
            errores_validacion.insert(
                pos,
                ErrorValidacion {
                    campo: Some("tipo".to_string()),
                    mensaje: "El tipo de cuenta no es válido.".to_string(),
                },
            );
        }

        if !errores_validacion.is_empty() {
            return Err(ErrorAplicacion::Validacion(errores_validacion));
        }

        // Construye la entidad y sella el evento con los metadatos de los puertos.
        let id_cuenta = self.ids.nuevo_id();
        let tipo = tipo_resultado.unwrap_or(TipoCuenta::Efectivo);
        let (_cuenta, evento_dominio) = Cuenta::crear(
            id_cuenta,
            &entrada.nombre,
            tipo,
            &entrada.moneda,
            &entrada.saldo_inicial,
            &nombres_existentes,
        )
        .map_err(|errs| ErrorAplicacion::Validacion(convertir_errores(errs)))?;

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_CUENTA_CREADA.to_string(),
            momento_registro: self.reloj.ahora_utc(),
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar(&evento_dominio),
        };
        self.repositorio.guardar(&evento)?;

        Ok(CuentaListada {
            id: evento_dominio.id.clone(),
            nombre: evento_dominio.nombre.clone(),
            tipo: codec::tipo_a_texto(&evento_dominio.tipo).to_string(),
            moneda: evento_dominio.moneda.codigo.clone(),
            saldo_actual: formatear_monto(
                evento_dominio.saldo_inicial,
                evento_dominio.moneda.decimales,
            ),
        })
    }
}

/// Formatea una cantidad decimal con el número exacto de decimales definidos para su moneda.
fn formatear_monto(cantidad: Decimal, decimales: u32) -> String {
    format!("{:.prec$}", cantidad, prec = decimales as usize)
}

/// Calcula la suma exacta de saldos por cada moneda presente, conservando el orden de aparición.
///
/// Los saldos llegan ya formateados desde la proyección; se reconstruyen como
/// decimales exactos a partir de su texto, nunca como punto flotante (INV-03).
fn calcular_subtotales(cuentas: &[CuentaProyectada]) -> Result<Vec<Subtotal>, ErrorAplicacion> {
    let mut monedas_orden: Vec<String> = Vec::new();
    let mut sumas: HashMap<String, Decimal> = HashMap::new();

    for c in cuentas {
        if !sumas.contains_key(&c.moneda) {
            monedas_orden.push(c.moneda.clone());
        }
        let monto = Decimal::from_str(&c.saldo_actual)
            .map_err(|e| ErrorAplicacion::Interno(format!("saldo inválido: {e}")))?;
        let actual = sumas
            .entry(c.moneda.clone())
            .or_insert_with(|| Decimal::new(0, 0));
        *actual += monto;
    }

    let mut subtotales = Vec::new();
    for moneda in monedas_orden {
        let decimales = Moneda::por_codigo(&moneda)
            .map(|m| m.decimales)
            .ok_or_else(|| ErrorAplicacion::Interno(format!("Moneda desconocida: {moneda}")))?;
        subtotales.push(Subtotal {
            moneda: moneda.clone(),
            total: formatear_monto(sumas[&moneda], decimales),
        });
    }
    Ok(subtotales)
}

/// Transforma la lista de errores de dominio al tipo [`ErrorValidacion`] de la capa de aplicación.
fn convertir_errores(errores: Vec<ErrorDominio>) -> Vec<ErrorValidacion> {
    errores
        .into_iter()
        .map(|e| ErrorValidacion {
            campo: Some(match e.campo {
                Campo::Nombre => "nombre".to_string(),
                Campo::Moneda => "moneda".to_string(),
                Campo::SaldoInicial => "saldo_inicial".to_string(),
            }),
            mensaje: e.mensaje,
        })
        .collect()
}
