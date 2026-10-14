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
