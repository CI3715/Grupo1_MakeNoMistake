//! Vista previa de la división de un gasto (Entrega 6). No usa la red: llama a la
//! división exacta del dominio y traduce el resultado al DTO de la interfaz.
//!
//! - Si los datos son válidos y cuadran: partes con su marca de redondeo.
//! - Si porcentajes o montos no cuadran: `Ok` con `cuadra = false` y el mensaje
//!   exacto en `diferencia` (la interfaz lo muestra en vivo mientras se escribe).
//! - Si un dato es inválido (monto vacío, regla desconocida...): errores por campo.

use dominio::division::{self, Participante};

use crate::dto::{ErrorCampo, ErroresDto};
use crate::dto_social::{ParteDto, PrevisualizacionDivisionDto, PrevisualizarDivisionDto};

pub fn previsualizar(
    datos: &PrevisualizarDivisionDto,
) -> Result<PrevisualizacionDivisionDto, ErroresDto> {
    let participantes: Vec<Participante> = datos
        .participantes
        .iter()
        .map(|p| Participante {
            usuario: p.usuario.clone(),
            valor: p.valor.clone(),
        })
        .collect();

    match division::dividir(&datos.monto, &datos.regla, &participantes) {
        Ok(d) => Ok(PrevisualizacionDivisionDto {
            partes: d
                .partes
                .into_iter()
                .map(|p| ParteDto {
                    usuario: p.usuario,
                    monto: p.monto,
                    incluye_redondeo: p.incluye_redondeo,
                })
                .collect(),
            residuo: d.residuo,
            cuadra: true,
            diferencia: None,
        }),
        Err(errores) => {
            if let Some(e) = errores.iter().find(|e| e.no_cuadra) {
                return Ok(PrevisualizacionDivisionDto {
                    partes: Vec::new(),
                    residuo: "0.00".to_string(),
                    cuadra: false,
                    diferencia: Some(e.mensaje.clone()),
                });
            }
            Err(ErroresDto {
                errores: errores
                    .into_iter()
                    .map(|e| ErrorCampo {
                        campo: Some(e.campo),
                        mensaje: e.mensaje,
                    })
                    .collect(),
            })
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dto_social::ParticipanteDto;

    fn datos(monto: &str, regla: &str, gente: &[(&str, &str)]) -> PrevisualizarDivisionDto {
        PrevisualizarDivisionDto {
            monto: monto.to_string(),
            regla: regla.to_string(),
            participantes: gente
                .iter()
                .map(|(u, v)| ParticipanteDto {
                    usuario: (*u).to_string(),
                    valor: (*v).to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn cien_en_partes_iguales_marca_la_parte_con_redondeo() {
        let d = datos(
            "100.00",
            "partes_iguales",
            &[("ana", ""), ("beto", ""), ("carla", "")],
        );
        let Ok(v) = previsualizar(&d) else {
            panic!("debía cuadrar");
        };
        let montos: Vec<_> = v.partes.iter().map(|p| p.monto.as_str()).collect();
        let marcas: Vec<_> = v.partes.iter().map(|p| p.incluye_redondeo).collect();
        assert_eq!(montos, ["33.34", "33.33", "33.33"]);
        assert_eq!(marcas, [true, false, false]);
        assert_eq!(v.residuo, "0.01");
        assert!(v.cuadra && v.diferencia.is_none());
    }

    #[test]
    fn porcentajes_que_no_cuadran_dan_la_diferencia_sin_ser_un_error() {
        let d = datos(
            "100.00",
            "porcentajes",
            &[("ana", "50"), ("beto", "25"), ("carla", "20")],
        );
        let Ok(v) = previsualizar(&d) else {
            panic!("la diferencia no es un error");
        };
        assert!(!v.cuadra && v.partes.is_empty());
        assert_eq!(
            v.diferencia.as_deref(),
            Some("Los porcentajes suman 95; faltan 5.")
        );
    }

    #[test]
    fn montos_que_no_cuadran_dan_la_diferencia_con_coma() {
        let d = datos(
            "120.00",
            "montos_fijos",
            &[("ana", "58.00"), ("beto", "60.00")],
        );
        let Ok(v) = previsualizar(&d) else {
            panic!("la diferencia no es un error");
        };
        assert_eq!(
            v.diferencia.as_deref(),
            Some("Los montos suman 118,00; faltan 2,00.")
        );
    }

    #[test]
    fn un_dato_invalido_llega_como_error_por_campo() {
        let d = datos("", "partes_iguales", &[]);
        let Err(e) = previsualizar(&d) else {
            panic!("se esperaban errores");
        };
        let campos: Vec<_> = e
            .errores
            .iter()
            .filter_map(|c| c.campo.as_deref())
            .collect();
        assert_eq!(campos, ["monto", "participantes"]);
    }

    #[test]
    fn los_montos_con_exceso_de_decimales_se_rechazan() {
        let d = datos("10.001", "partes_iguales", &[("ana", "")]);
        assert!(previsualizar(&d).is_err());
    }
}
