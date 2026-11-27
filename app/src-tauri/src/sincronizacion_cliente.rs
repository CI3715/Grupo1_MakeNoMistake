//! Cliente de los cambios del servidor (Entrega 7).
//!
//! Pide al servidor los eventos de gasto de grupo del usuario posteriores a un cursor.
//! Este archivo solo ve la petición y la respuesta: no sabe nada del registro personal
//! del usuario. Quien guarda lo recibido es otro módulo. La red real vive en
//! `ClienteHttp`, en `sesion.rs`, igual que en los demás clientes. Lo único que viaja
//! es el cursor (INV-16, RES-02).

use serde::Deserialize;

use crate::dto::ErroresDto;
use crate::sesion::Almacen;
use crate::social_cliente::{ClienteSocial, Peticion, leer, llamar};

pub const TIPO_GASTO_DE_GRUPO: &str = "gasto_de_grupo";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrupoDelEvento {
    pub id: String,
    pub nombre: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GastoDelEvento {
    pub id: String,
    pub concepto: String,
}

/// Un evento de GET /cambios. Los datos de la parte solo vienen en `gasto_de_grupo`:
/// un tipo que esta versión no conoce trae al menos id, posicion y tipo, y se ignora.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EventoGastoDeGrupo {
    pub id: String,
    pub posicion: i64,
    pub tipo: String,
    #[serde(default)]
    pub grupo: Option<GrupoDelEvento>,
    #[serde(default)]
    pub gasto: Option<GastoDelEvento>,
    #[serde(default)]
    pub fecha: Option<String>,
    #[serde(default)]
    pub moneda: Option<String>,
    #[serde(default)]
    pub monto: Option<String>,
    #[serde(default)]
    pub incluye_redondeo: Option<bool>,
}

/// La respuesta de GET /cambios: un lote de a lo sumo 100 eventos.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaginaCambios {
    pub eventos: Vec<EventoGastoDeGrupo>,
    pub cursor: i64,
    pub hay_mas: bool,
}

/// Pide los cambios posteriores a `desde`. Sin sesión, con la sesión vencida, sin
/// conexión o con un 401 se comporta igual que los demás comandos (ver `llamar`).
pub async fn pedir_cambios<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    desde: i64,
) -> Result<PaginaCambios, ErroresDto> {
    let peticion = Peticion::get(&format!("/cambios?desde={desde}"));
    let respuesta = llamar(cliente, almacen, ahora, &peticion, &[200]).await?;
    leer(&respuesta)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const PAGINA: &str = r#"{
        "eventos": [
            {"id": "e-100", "posicion": 1, "tipo": "gasto_de_grupo",
             "grupo": {"id": "g1", "nombre": "Viaje a Mérida"},
             "gasto": {"id": "x1", "concepto": "Hotel"},
             "fecha": "2026-11-20", "moneda": "USD", "monto": "33.33",
             "incluye_redondeo": false},
            {"id": "e-101", "posicion": 2, "tipo": "tipo_futuro", "algo": {"nuevo": true}}
        ],
        "cursor": 2,
        "hay_mas": false
    }"#;

    #[test]
    fn una_pagina_con_un_tipo_desconocido_se_lee_sin_fallar() {
        let pagina: PaginaCambios = serde_json::from_str(PAGINA).unwrap();
        assert_eq!(pagina.eventos.len(), 2);
        assert_eq!(pagina.cursor, 2);
        assert!(!pagina.hay_mas);
        let parte = &pagina.eventos[0];
        assert_eq!(parte.tipo, TIPO_GASTO_DE_GRUPO);
        assert_eq!(parte.monto.as_deref(), Some("33.33"));
        assert_eq!(parte.grupo.as_ref().unwrap().nombre, "Viaje a Mérida");
        let ajeno = &pagina.eventos[1];
        assert_eq!(ajeno.tipo, "tipo_futuro");
        assert!(ajeno.grupo.is_none() && ajeno.monto.is_none());
    }

    #[test]
    fn una_pagina_vacia_conserva_el_cursor() {
        let pagina: PaginaCambios =
            serde_json::from_str(r#"{"eventos": [], "cursor": 7, "hay_mas": false}"#).unwrap();
        assert!(pagina.eventos.is_empty());
        assert_eq!(pagina.cursor, 7);
    }
}
