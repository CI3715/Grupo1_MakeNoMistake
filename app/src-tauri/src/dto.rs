use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct MonedaDto {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u8,
}

#[derive(Serialize)]
pub struct CuentaDto {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub moneda: String,
    pub saldo_actual: String,
}

#[derive(Serialize)]
pub struct SubtotalDto {
    pub moneda: String,
    pub total: String,
}

#[derive(Serialize)]
pub struct ListaCuentasDto {
    pub cuentas: Vec<CuentaDto>,
    pub subtotales: Vec<SubtotalDto>,
}

#[derive(Deserialize)]
pub struct NuevaCuentaDto {
    pub nombre: String,
    pub tipo: String,
    pub moneda: String,
    pub saldo_inicial: String,
}

#[derive(Serialize)]
pub struct ErrorCampo {
    pub campo: Option<String>,
    pub mensaje: String,
}

#[derive(Serialize)]
pub struct ErroresDto {
    pub errores: Vec<ErrorCampo>,
}

// ---------- Conversiones desde la capa de aplicación ----------

impl From<aplicacion::MonedaDto> for MonedaDto {
    fn from(m: aplicacion::MonedaDto) -> Self {
        MonedaDto {
            codigo: m.codigo,
            nombre: m.nombre,
            simbolo: m.simbolo,
            decimales: m.decimales,
        }
    }
}

impl From<aplicacion::CuentaListada> for CuentaDto {
    fn from(c: aplicacion::CuentaListada) -> Self {
        CuentaDto {
            id: c.id,
            nombre: c.nombre,
            tipo: c.tipo,
            moneda: c.moneda,
            saldo_actual: c.saldo_actual,
        }
    }
}

impl From<aplicacion::ListaCuentas> for ListaCuentasDto {
    fn from(l: aplicacion::ListaCuentas) -> Self {
        ListaCuentasDto {
            cuentas: l.cuentas.into_iter().map(CuentaDto::from).collect(),
            subtotales: l
                .subtotales
                .into_iter()
                .map(|s| SubtotalDto {
                    moneda: s.moneda,
                    total: s.total,
                })
                .collect(),
        }
    }
}

impl From<NuevaCuentaDto> for aplicacion::NuevaCuenta {
    fn from(d: NuevaCuentaDto) -> Self {
        aplicacion::NuevaCuenta {
            nombre: d.nombre,
            tipo: d.tipo,
            moneda: d.moneda,
            saldo_inicial: d.saldo_inicial,
        }
    }
}

/// Errores de validación (por campo) o un único mensaje general para fallos
/// internos o de almacenamiento. El detalle técnico nunca llega a la pantalla.
impl From<aplicacion::ErrorAplicacion> for ErroresDto {
    fn from(e: aplicacion::ErrorAplicacion) -> Self {
        let errores = match e {
            aplicacion::ErrorAplicacion::Validacion(lista) => lista
                .into_iter()
                .map(|v| ErrorCampo {
                    campo: v.campo,
                    mensaje: v.mensaje,
                })
                .collect(),
            otro => vec![ErrorCampo {
                campo: None,
                mensaje: otro.to_string(),
            }],
        };
        ErroresDto { errores }
    }
}
