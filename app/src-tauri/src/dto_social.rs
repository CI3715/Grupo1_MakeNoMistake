//! DTO de amistades, grupos y gastos compartidos (Entrega 6).
//!
//! Convenciones (las mismas de las entregas anteriores):
//! - Montos y porcentajes viajan como texto con punto decimal ("100.00", "33.33"),
//!   nunca como número; la interfaz solo les da formato. En los mensajes en español
//!   se escriben con coma ("118,00").
//! - Fechas como AAAA-MM-DD. Nombres en snake_case.
//! - Los errores usan `ErroresDto` ({campo, mensaje}) con el campo que falla.
//! - Los campos de entrada tienen valor por defecto para que un dato ausente se
//!   rechace con un mensaje por campo y no con un error técnico.
//! - A las personas se las identifica por su nombre de usuario, nunca por correo.
//! - NADA del libro personal (cuentas, movimientos, saldos, tasas, categorías)
//!   aparece aquí: no viaja al servidor (INV-16). Una prueba lo comprueba.
//!
//! Valores de texto acordados:
//! - `respuesta`: "aceptar" | "rechazar".
//! - `estado` de una invitación: "pendiente" | "aceptada" | "rechazada".
//! - `rol` en un grupo: "administrador" | "miembro".
//! - `regla`: "partes_iguales" | "porcentajes" | "montos_fijos".

// Se usan al conectar los comandos (Entrega 6, tareas de comandos reales).
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ---------- Personas y amistades ----------

/// Búsqueda por nombre de usuario exacto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuscarUsuarioDto {
    #[serde(default)]
    pub usuario: String,
}

/// Lo único que se sabe de otra persona: nunca lleva el correo (RF-SOC-04).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsuarioPublicoDto {
    pub nombre_usuario: String,
    pub nombre_visible: String,
}

/// Invitación de amistad (y cuerpo de POST /amistades): se invita por nombre de usuario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitarAmigoDto {
    #[serde(default)]
    pub usuario: String,
}

/// Responder una invitación de amistad. `respuesta`: "aceptar" | "rechazar".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponderAmistadDto {
    #[serde(default)]
    pub amistad_id: String,
    #[serde(default)]
    pub respuesta: String,
}

/// Un amigo con el identificador de la amistad (sirve para eliminarla).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmigoDto {
    pub amistad_id: String,
    pub usuario: UsuarioPublicoDto,
}

/// Invitación de amistad pendiente. `usuario` es la otra persona: a quien se invitó
/// (en las enviadas) o quien invitó (en las recibidas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitacionAmistadDto {
    pub id: String,
    pub usuario: UsuarioPublicoDto,
    /// "pendiente" | "aceptada" | "rechazada"
    pub estado: String,
}

/// Lista de amistades: amigos e invitaciones pendientes, enviadas y recibidas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListaAmistadesDto {
    pub amigos: Vec<AmigoDto>,
    pub enviadas: Vec<InvitacionAmistadDto>,
    pub recibidas: Vec<InvitacionAmistadDto>,
}

// ---------- Grupos y miembros ----------

/// Datos para crear un grupo. `moneda`: código de 3 letras. `descripcion` es
/// opcional ("" si no hay); `actividad` es la fecha o actividad asociada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuevoGrupoDto {
    #[serde(default)]
    pub nombre: String,
    #[serde(default)]
    pub descripcion: String,
    #[serde(default)]
    pub moneda: String,
    #[serde(default)]
    pub actividad: String,
}

/// Grupo en las listas. `rol`: "administrador" | "miembro".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrupoResumenDto {
    pub id: String,
    pub nombre: String,
    pub descripcion: String,
    pub moneda: String,
    pub actividad: String,
    pub rol: String,
    pub cantidad_miembros: u32,
}

/// Invitación a un grupo que todavía no se aceptó (no da acceso a gastos ni balances).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitacionGrupoDto {
    pub grupo_id: String,
    pub nombre: String,
    pub moneda: String,
    pub actividad: String,
    pub invitado_por: UsuarioPublicoDto,
}

/// Mis grupos y mis invitaciones pendientes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListaGruposDto {
    pub grupos: Vec<GrupoResumenDto>,
    pub invitaciones: Vec<InvitacionGrupoDto>,
}

/// El administrador invita a un amigo al grupo (por nombre de usuario).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitarAlGrupoDto {
    #[serde(default)]
    pub grupo_id: String,
    #[serde(default)]
    pub usuario: String,
}

/// Persona invitada a un grupo que aún no respondió.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitadoDto {
    pub usuario: String,
    pub nombre_visible: String,
    /// "pendiente" | "aceptada" | "rechazada"
    pub estado: String,
}

/// Responder la invitación a un grupo. `respuesta`: "aceptar" | "rechazar".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponderInvitacionGrupoDto {
    #[serde(default)]
    pub grupo_id: String,
    #[serde(default)]
    pub respuesta: String,
}

// ---------- Gastos compartidos y división ----------

/// Un participante de un gasto. `valor`: el porcentaje ("50") en la regla de
/// porcentajes o el monto ("40.00") en la de montos fijos; vacío en partes iguales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipanteDto {
    #[serde(default)]
    pub usuario: String,
    #[serde(default)]
    pub valor: String,
}

/// Gasto nuevo de un grupo (y cuerpo de POST /grupos/{id}/gastos). `monto`: texto
/// positivo con dos decimales; `fecha`: AAAA-MM-DD; `pagador`: nombre de usuario de
/// un miembro; `regla`: "partes_iguales" | "porcentajes" | "montos_fijos".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuevoGastoGrupoDto {
    #[serde(default)]
    pub grupo_id: String,
    #[serde(default)]
    pub concepto: String,
    #[serde(default)]
    pub monto: String,
    #[serde(default)]
    pub fecha: String,
    #[serde(default)]
    pub pagador: String,
    #[serde(default)]
    pub regla: String,
    #[serde(default)]
    pub participantes: Vec<ParticipanteDto>,
}

/// Parte de un participante en un gasto. `incluye_redondeo` es verdadero si recibió
/// un céntimo del residuo (la interfaz muestra «incluye 0,01 de redondeo»).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParteDto {
    pub usuario: String,
    pub monto: String,
    pub incluye_redondeo: bool,
}

/// Un gasto del grupo ya registrado, con la división que calculó el servidor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GastoGrupoDto {
    pub id: String,
    pub concepto: String,
    pub monto: String,
    pub fecha: String,
    pub pagador: String,
    pub regla: String,
    pub partes: Vec<ParteDto>,
}

/// Datos para previsualizar la división sin enviar nada al servidor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrevisualizarDivisionDto {
    #[serde(default)]
    pub monto: String,
    #[serde(default)]
    pub regla: String,
    #[serde(default)]
    pub participantes: Vec<ParticipanteDto>,
}

/// Vista previa de la división. Si los porcentajes o montos no cuadran, `cuadra` es
/// falso y `diferencia` trae el mensaje exacto («Los porcentajes suman 95; faltan 5.»).
/// Cuando no cuadra, `partes` va vacío.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrevisualizacionDivisionDto {
    pub partes: Vec<ParteDto>,
    /// Suma de los céntimos repartidos como residuo ("0.01"; "0.00" si no hubo).
    pub residuo: String,
    pub cuadra: bool,
    pub diferencia: Option<String>,
}

// ---------- Detalle del grupo ----------

/// Miembro aceptado del grupo. `rol`: "administrador" | "miembro".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiembroDto {
    pub usuario: String,
    pub nombre_visible: String,
    pub rol: String,
}

/// Balance de un miembro: lo que pagó menos lo que le corresponde. Positivo: le
/// deben; negativo ("-48.33"): debe. La suma de todos es exactamente "0.00".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceDto {
    pub usuario: String,
    pub pago: String,
    pub le_corresponde: String,
    pub balance: String,
}

/// Detalle de un grupo: miembros, gastos (del más reciente al más antiguo) y balances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrupoDetalleDto {
    pub grupo: GrupoResumenDto,
    pub miembros: Vec<MiembroDto>,
    pub gastos: Vec<GastoGrupoDto>,
    pub balances: Vec<BalanceDto>,
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::{Value, json};

    fn claves(v: &Value) -> Vec<String> {
        let mut c: Vec<String> = v.as_object().expect("objeto").keys().cloned().collect();
        c.sort();
        c
    }

    fn todas_las_claves(v: &Value, fuera: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    fuera.push(k.clone());
                    todas_las_claves(x, fuera);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| todas_las_claves(x, fuera)),
            _ => {}
        }
    }

    fn usuario(n: &str) -> UsuarioPublicoDto {
        UsuarioPublicoDto {
            nombre_usuario: n.to_string(),
            nombre_visible: n.to_uppercase(),
        }
    }

    fn resumen() -> GrupoResumenDto {
        GrupoResumenDto {
            id: "g1".into(),
            nombre: "Viaje a Mérida".into(),
            descripcion: String::new(),
            moneda: "USD".into(),
            actividad: "marzo".into(),
            rol: "administrador".into(),
            cantidad_miembros: 3,
        }
    }

    fn parte(u: &str, m: &str, r: bool) -> ParteDto {
        ParteDto {
            usuario: u.into(),
            monto: m.into(),
            incluye_redondeo: r,
        }
    }

    fn detalle() -> GrupoDetalleDto {
        GrupoDetalleDto {
            grupo: resumen(),
            miembros: vec![MiembroDto {
                usuario: "ana".into(),
                nombre_visible: "Ana".into(),
                rol: "administrador".into(),
            }],
            gastos: vec![GastoGrupoDto {
                id: "x1".into(),
                concepto: "Cena".into(),
                monto: "100.00".into(),
                fecha: "2026-11-12".into(),
                pagador: "ana".into(),
                regla: "partes_iguales".into(),
                partes: vec![
                    parte("ana", "33.34", true),
                    parte("beto", "33.33", false),
                    parte("carla", "33.33", false),
                ],
            }],
            balances: vec![BalanceDto {
                usuario: "ana".into(),
                pago: "100.00".into(),
                le_corresponde: "33.34".into(),
                balance: "66.66".into(),
            }],
        }
    }

    #[test]
    fn el_detalle_del_grupo_tiene_los_campos_acordados() {
        let v = serde_json::to_value(detalle()).unwrap();
        assert_eq!(claves(&v), vec!["balances", "gastos", "grupo", "miembros"]);
        assert_eq!(
            claves(&v["gastos"][0]),
            vec![
                "concepto", "fecha", "id", "monto", "pagador", "partes", "regla"
            ]
        );
        assert_eq!(
            claves(&v["gastos"][0]["partes"][0]),
            vec!["incluye_redondeo", "monto", "usuario"]
        );
        assert_eq!(
            claves(&v["balances"][0]),
            vec!["balance", "le_corresponde", "pago", "usuario"]
        );
    }

    #[test]
    fn el_usuario_publico_nunca_lleva_correo() {
        let v = serde_json::to_value(usuario("beto")).unwrap();
        assert_eq!(claves(&v), vec!["nombre_usuario", "nombre_visible"]);
    }

    #[test]
    fn los_montos_viajan_como_texto() {
        let v = serde_json::to_value(detalle()).unwrap();
        assert!(v["gastos"][0]["monto"].is_string());
        assert!(v["gastos"][0]["partes"][0]["monto"].is_string());
        assert!(v["balances"][0]["balance"].is_string());
    }

    #[test]
    fn los_datos_de_entrada_ausentes_no_fallan_al_leerse() {
        let g: NuevoGastoGrupoDto = serde_json::from_value(json!({})).unwrap();
        assert!(g.monto.is_empty() && g.regla.is_empty() && g.participantes.is_empty());
        let p: ParticipanteDto = serde_json::from_value(json!({"usuario": "ana"})).unwrap();
        assert!(p.valor.is_empty());
        let n: NuevoGrupoDto = serde_json::from_value(json!({"nombre": "Viaje"})).unwrap();
        assert!(n.moneda.is_empty() && n.descripcion.is_empty());
        let r: ResponderAmistadDto = serde_json::from_value(json!({})).unwrap();
        assert!(r.amistad_id.is_empty() && r.respuesta.is_empty());
    }

    #[test]
    fn ningun_dto_lleva_datos_del_libro_personal() {
        let muestras: Vec<Value> = vec![
            serde_json::to_value(detalle()).unwrap(),
            serde_json::to_value(ListaAmistadesDto {
                amigos: vec![AmigoDto {
                    amistad_id: "a1".into(),
                    usuario: usuario("beto"),
                }],
                enviadas: vec![],
                recibidas: vec![InvitacionAmistadDto {
                    id: "i1".into(),
                    usuario: usuario("carla"),
                    estado: "pendiente".into(),
                }],
            })
            .unwrap(),
            serde_json::to_value(ListaGruposDto {
                grupos: vec![resumen()],
                invitaciones: vec![InvitacionGrupoDto {
                    grupo_id: "g1".into(),
                    nombre: "Viaje".into(),
                    moneda: "USD".into(),
                    actividad: "marzo".into(),
                    invitado_por: usuario("ana"),
                }],
            })
            .unwrap(),
            serde_json::to_value(PrevisualizacionDivisionDto {
                partes: vec![parte("ana", "33.34", true)],
                residuo: "0.01".into(),
                cuadra: true,
                diferencia: None,
            })
            .unwrap(),
            serde_json::to_value(NuevoGastoGrupoDto {
                grupo_id: "g1".into(),
                concepto: "Cena".into(),
                monto: "100.00".into(),
                fecha: "2026-11-12".into(),
                pagador: "ana".into(),
                regla: "partes_iguales".into(),
                participantes: vec![ParticipanteDto {
                    usuario: "ana".into(),
                    valor: String::new(),
                }],
            })
            .unwrap(),
            serde_json::to_value(NuevoGrupoDto {
                nombre: "Viaje".into(),
                descripcion: String::new(),
                moneda: "USD".into(),
                actividad: "marzo".into(),
            })
            .unwrap(),
        ];
        let prohibidas = [
            "cuenta",
            "saldo",
            "movimiento",
            "tasa",
            "categoria",
            "correo",
        ];
        for m in &muestras {
            let mut claves = Vec::new();
            todas_las_claves(m, &mut claves);
            for c in claves {
                assert!(
                    !prohibidas.iter().any(|p| c.contains(p)),
                    "el campo `{c}` no puede viajar al servidor (INV-16)"
                );
            }
        }
    }
}
