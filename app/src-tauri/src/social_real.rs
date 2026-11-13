//! Comandos reales de amistades y grupos (Entrega 6), contra el JSON que devuelve
//! el servidor en la rama `entrega6A`.
//!
//! Los usa `social.rs`: cada comando de Tauri llama a su versión `*_real`. El
//! servidor devuelve el detalle del grupo y los gastos con la forma de los DTO;
//! las amistades y la respuesta a invitaciones sí necesitan traducción.
//!
//! Aquí se traduce la forma del servidor a la de los DTO que usa la interfaz:
//! - GET /amistades: lista plana -> amigos, enviadas y recibidas.
//! - POST /amistades/{id}/respuesta: `{aceptar: bool}` en vez de `respuesta`.
//! - Los estados `invitado`, `aceptado`... se normalizan a pendiente/aceptada/rechazada.
//! - GET /grupos/{id}: campos del grupo planos -> `{grupo, miembros, gastos, balances}`.

#![allow(dead_code)]

use serde::Deserialize;
use serde_json::json;
use tauri::Manager;

use crate::dto::{ErrorCampo, ErroresDto};
use crate::dto_social::{
    AmigoDto, BuscarUsuarioDto, GastoGrupoDto, GrupoDetalleDto, GrupoResumenDto,
    InvitacionAmistadDto, InvitadoDto, InvitarAlGrupoDto, InvitarAmigoDto, ListaAmistadesDto,
    ListaGruposDto, NuevoGastoGrupoDto, NuevoGrupoDto, ResponderAmistadDto,
    ResponderInvitacionGrupoDto, UsuarioPublicoDto,
};
use crate::sesion::{Almacen, AlmacenSistema, ClienteHttp, ahora_iso, sin_campo};
use crate::social_cliente::{ClienteSocial, Peticion, codificar, leer, llamar};

// ---------- Lo que responde el servidor ----------

#[derive(Deserialize)]
struct AmistadServidor {
    id: String,
    #[serde(default)]
    estado: String,
    #[serde(default)]
    otro_nombre_usuario: String,
    #[serde(default)]
    otro_nombre_visible: String,
    #[serde(default)]
    enviada_por_mi: bool,
}

#[derive(Deserialize)]
struct InvitadoServidor {
    usuario: String,
    #[serde(default)]
    nombre_visible: String,
    #[serde(default)]
    estado: String,
}

// ---------- Traducciones ----------

fn falla(campo: &str, mensaje: &str) -> ErroresDto {
    ErroresDto {
        errores: vec![ErrorCampo {
            campo: Some(campo.to_string()),
            mensaje: mensaje.to_string(),
        }],
    }
}

fn estado_normal(estado: &str) -> String {
    match estado {
        "aceptada" | "aceptado" => "aceptada",
        "rechazada" | "rechazado" => "rechazada",
        "invitado" | "pendiente" => "pendiente",
        otro => otro,
    }
    .to_string()
}

fn publico(usuario: &str, visible: &str) -> UsuarioPublicoDto {
    UsuarioPublicoDto {
        nombre_usuario: usuario.to_string(),
        nombre_visible: visible.to_string(),
    }
}

fn a_invitacion(a: &AmistadServidor) -> InvitacionAmistadDto {
    InvitacionAmistadDto {
        id: a.id.clone(),
        usuario: publico(&a.otro_nombre_usuario, &a.otro_nombre_visible),
        estado: estado_normal(&a.estado),
    }
}

fn respuesta_valida(texto: &str) -> Option<bool> {
    match texto.trim() {
        "aceptar" => Some(true),
        "rechazar" => Some(false),
        _ => None,
    }
}

// ---------- Amistades ----------

pub async fn buscar<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &BuscarUsuarioDto,
) -> Result<UsuarioPublicoDto, ErroresDto> {
    let u = datos.usuario.trim().to_lowercase();
    if u.is_empty() {
        return Err(falla("usuario", "Escribe un nombre de usuario."));
    }
    let ruta = format!("/usuarios?usuario={}", codificar(&u));
    leer(&llamar(c, a, ahora, &Peticion::get(&ruta), &[200]).await?)
}

pub async fn invitar_amigo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &InvitarAmigoDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    let cuerpo = json!({ "usuario": datos.usuario.trim().to_lowercase() });
    let r = llamar(c, a, ahora, &Peticion::post("/amistades", &cuerpo), &[201]).await?;
    let creada: AmistadServidor = leer(&r)?;
    Ok(a_invitacion(&creada))
}

pub async fn listar_amistades<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
) -> Result<ListaAmistadesDto, ErroresDto> {
    let r = llamar(c, a, ahora, &Peticion::get("/amistades"), &[200]).await?;
    let lista: Vec<AmistadServidor> = leer(&r)?;
    let mut fuera = ListaAmistadesDto {
        amigos: Vec::new(),
        enviadas: Vec::new(),
        recibidas: Vec::new(),
    };
    for x in &lista {
        match estado_normal(&x.estado).as_str() {
            "aceptada" => fuera.amigos.push(AmigoDto {
                amistad_id: x.id.clone(),
                usuario: publico(&x.otro_nombre_usuario, &x.otro_nombre_visible),
            }),
            "pendiente" if x.enviada_por_mi => fuera.enviadas.push(a_invitacion(x)),
            "pendiente" => fuera.recibidas.push(a_invitacion(x)),
            _ => {}
        }
    }
    Ok(fuera)
}

pub async fn responder_amistad<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    let Some(aceptar) = respuesta_valida(&datos.respuesta) else {
        return Err(falla("respuesta", "Elige aceptar o rechazar."));
    };
    let ruta = format!("/amistades/{}/respuesta", codificar(&datos.amistad_id));
    let cuerpo = json!({ "aceptar": aceptar });
    llamar(c, a, ahora, &Peticion::post(&ruta, &cuerpo), &[200]).await?;
    // El servidor no devuelve quién es la otra persona: se completa con la lista.
    let usuario = match listar_crudo(c, a, ahora).await {
        Ok(lista) => lista
            .into_iter()
            .find(|x| x.id == datos.amistad_id)
            .map(|x| publico(&x.otro_nombre_usuario, &x.otro_nombre_visible)),
        Err(_) => None,
    };
    Ok(InvitacionAmistadDto {
        id: datos.amistad_id.clone(),
        usuario: usuario.unwrap_or_else(|| publico("", "")),
        estado: if aceptar { "aceptada" } else { "rechazada" }.to_string(),
    })
}

async fn listar_crudo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
) -> Result<Vec<AmistadServidor>, ErroresDto> {
    leer(&llamar(c, a, ahora, &Peticion::get("/amistades"), &[200]).await?)
}

pub async fn eliminar_amistad<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    amistad_id: &str,
) -> Result<(), ErroresDto> {
    let ruta = format!("/amistades/{}", codificar(amistad_id));
    llamar(c, a, ahora, &Peticion::delete(&ruta), &[204]).await?;
    Ok(())
}

// ---------- Grupos ----------

pub async fn crear_grupo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &NuevoGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    let r = llamar(c, a, ahora, &Peticion::post("/grupos", datos), &[201]).await?;
    leer(&r)
}

pub async fn listar_grupos<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
) -> Result<ListaGruposDto, ErroresDto> {
    leer(&llamar(c, a, ahora, &Peticion::get("/grupos"), &[200]).await?)
}

pub async fn invitar_al_grupo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &InvitarAlGrupoDto,
) -> Result<InvitadoDto, ErroresDto> {
    let ruta = format!("/grupos/{}/miembros", codificar(&datos.grupo_id));
    let cuerpo = json!({ "usuario": datos.usuario.trim().to_lowercase() });
    let r = llamar(c, a, ahora, &Peticion::post(&ruta, &cuerpo), &[201]).await?;
    let x: InvitadoServidor = leer(&r)?;
    Ok(InvitadoDto {
        usuario: x.usuario,
        nombre_visible: x.nombre_visible,
        estado: estado_normal(&x.estado),
    })
}

pub async fn responder_invitacion_grupo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    let Some(aceptar) = respuesta_valida(&datos.respuesta) else {
        return Err(falla("respuesta", "Elige aceptar o rechazar."));
    };
    // Los datos del grupo se leen de la invitación: la respuesta del servidor solo trae un mensaje.
    let antes = listar_grupos(c, a, ahora).await?;
    let ruta = format!(
        "/grupos/{}/miembros/yo/respuesta",
        codificar(&datos.grupo_id)
    );
    let cuerpo = json!({ "respuesta": if aceptar { "aceptar" } else { "rechazar" } });
    llamar(c, a, ahora, &Peticion::post(&ruta, &cuerpo), &[200]).await?;
    let invitacion = antes
        .invitaciones
        .into_iter()
        .find(|i| i.grupo_id == datos.grupo_id);
    Ok(GrupoResumenDto {
        id: datos.grupo_id.clone(),
        nombre: invitacion
            .as_ref()
            .map(|i| i.nombre.clone())
            .unwrap_or_default(),
        descripcion: String::new(),
        moneda: invitacion
            .as_ref()
            .map(|i| i.moneda.clone())
            .unwrap_or_default(),
        actividad: invitacion.map(|i| i.actividad).unwrap_or_default(),
        rol: "miembro".to_string(),
        cantidad_miembros: 0,
    })
}

pub async fn ver_grupo<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    grupo_id: &str,
) -> Result<GrupoDetalleDto, ErroresDto> {
    let ruta = format!("/grupos/{}", codificar(grupo_id));
    leer(&llamar(c, a, ahora, &Peticion::get(&ruta), &[200]).await?)
}

/// El servidor recalcula la división y guarda su resultado; responde el gasto con las partes.
pub async fn registrar_gasto<C: ClienteSocial, A: Almacen>(
    c: &C,
    a: &A,
    ahora: &str,
    datos: &NuevoGastoGrupoDto,
) -> Result<GastoGrupoDto, ErroresDto> {
    let ruta = format!("/grupos/{}/gastos", codificar(&datos.grupo_id));
    let cuerpo = json!({
        "concepto": datos.concepto,
        "monto": datos.monto,
        "fecha": datos.fecha,
        "pagador": datos.pagador,
        "regla": datos.regla,
        "participantes": datos.participantes,
    });
    leer(&llamar(c, a, ahora, &Peticion::post(&ruta, &cuerpo), &[201]).await?)
}

// ---------- Comandos (envoltorios con la red real y el almacén del sistema) ----------

fn almacen_de(app: &tauri::AppHandle) -> Result<AlmacenSistema, ErroresDto> {
    let carpeta = app
        .path()
        .app_data_dir()
        .map_err(|_| sin_campo("No se pudo acceder a los datos de la aplicación."))?;
    Ok(AlmacenSistema::nuevo(&carpeta))
}

pub async fn buscar_usuario_real(
    app: &tauri::AppHandle,
    datos: BuscarUsuarioDto,
) -> Result<UsuarioPublicoDto, ErroresDto> {
    buscar(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn invitar_amigo_real(
    app: &tauri::AppHandle,
    datos: InvitarAmigoDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    invitar_amigo(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn responder_amistad_real(
    app: &tauri::AppHandle,
    datos: ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    responder_amistad(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn listar_amistades_real(
    app: &tauri::AppHandle,
) -> Result<ListaAmistadesDto, ErroresDto> {
    listar_amistades(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
    )
    .await
}

pub async fn eliminar_amistad_real(
    app: &tauri::AppHandle,
    amistad_id: String,
) -> Result<(), ErroresDto> {
    eliminar_amistad(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &amistad_id,
    )
    .await
}

pub async fn crear_grupo_real(
    app: &tauri::AppHandle,
    datos: NuevoGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    crear_grupo(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn listar_grupos_real(app: &tauri::AppHandle) -> Result<ListaGruposDto, ErroresDto> {
    listar_grupos(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
    )
    .await
}

pub async fn invitar_al_grupo_real(
    app: &tauri::AppHandle,
    datos: InvitarAlGrupoDto,
) -> Result<InvitadoDto, ErroresDto> {
    invitar_al_grupo(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn responder_invitacion_grupo_real(
    app: &tauri::AppHandle,
    datos: ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    responder_invitacion_grupo(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

pub async fn ver_grupo_real(
    app: &tauri::AppHandle,
    grupo_id: String,
) -> Result<GrupoDetalleDto, ErroresDto> {
    ver_grupo(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &grupo_id,
    )
    .await
}

pub async fn registrar_gasto_grupo_real(
    app: &tauri::AppHandle,
    datos: NuevoGastoGrupoDto,
) -> Result<GastoGrupoDto, ErroresDto> {
    registrar_gasto(
        &ClienteHttp::desde_entorno(),
        &almacen_de(app)?,
        &ahora_iso(),
        &datos,
    )
    .await
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::sesion::{Respuesta, SesionGuardada, SinConexion};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    const AHORA: &str = "2026-11-12T12:00:00Z";

    /// Cliente falso: devuelve las respuestas en el orden en que se piden.
    struct Guion {
        respuestas: Mutex<VecDeque<(u16, String)>>,
        llamadas: Mutex<Vec<String>>,
    }

    fn guion(r: &[(u16, &str)]) -> Guion {
        Guion {
            respuestas: Mutex::new(r.iter().map(|(c, t)| (*c, (*t).to_string())).collect()),
            llamadas: Mutex::new(Vec::new()),
        }
    }

    impl ClienteSocial for Guion {
        async fn enviar_peticion(
            &self,
            p: &Peticion,
            _token: &str,
        ) -> Result<Respuesta, SinConexion> {
            self.llamadas.lock().unwrap().push(format!(
                "{:?} {} {}",
                p.metodo,
                p.ruta,
                p.cuerpo.clone().unwrap_or_default()
            ));
            match self.respuestas.lock().unwrap().pop_front() {
                Some((codigo, cuerpo)) => Ok(Respuesta { codigo, cuerpo }),
                None => Err(SinConexion),
            }
        }
    }

    #[derive(Default)]
    struct Memoria(Mutex<Option<SesionGuardada>>);

    impl Almacen for Memoria {
        fn guardar(&self, s: &SesionGuardada) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(s.clone());
            Ok(())
        }
        fn leer(&self) -> Option<SesionGuardada> {
            self.0.lock().unwrap().clone()
        }
        fn borrar(&self) {
            *self.0.lock().unwrap() = None;
        }
    }

    fn con_sesion() -> Memoria {
        let m = Memoria::default();
        m.guardar(&SesionGuardada {
            token: "t".into(),
            vence_en: "2099-01-01T00:00:00Z".into(),
            nombre_visible: "Ana".into(),
            invalidada: false,
        })
        .unwrap();
        m
    }

    fn exito<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => panic!(
                "no debía fallar: {}",
                serde_json::to_string(&e).unwrap_or_default()
            ),
        }
    }

    const LISTA: &str = r#"[
      {"id":"a1","estado":"aceptada","otro_id":"u1","otro_nombre_usuario":"beto","otro_nombre_visible":"Beto","enviada_por_mi":true,"fecha_invitacion":"x","fecha_respuesta":"y"},
      {"id":"a2","estado":"pendiente","otro_id":"u2","otro_nombre_usuario":"carla","otro_nombre_visible":"Carla","enviada_por_mi":true,"fecha_invitacion":"x","fecha_respuesta":null},
      {"id":"a3","estado":"pendiente","otro_id":"u3","otro_nombre_usuario":"dario","otro_nombre_visible":"Darío","enviada_por_mi":false,"fecha_invitacion":"x","fecha_respuesta":null},
      {"id":"a4","estado":"rechazada","otro_id":"u4","otro_nombre_usuario":"luis","otro_nombre_visible":"Luis","enviada_por_mi":false,"fecha_invitacion":"x","fecha_respuesta":"y"}
    ]"#;

    #[tokio::test]
    async fn buscar_codifica_el_nombre_y_lee_la_respuesta() {
        let c = guion(&[(
            200,
            r#"{"nombre_usuario":"beto","nombre_visible":"Beto G"}"#,
        )]);
        let u = exito(
            buscar(
                &c,
                &con_sesion(),
                AHORA,
                &BuscarUsuarioDto {
                    usuario: " Beto ".into(),
                },
            )
            .await,
        );
        assert_eq!(
            (u.nombre_usuario.as_str(), u.nombre_visible.as_str()),
            ("beto", "Beto G")
        );
        assert_eq!(*c.llamadas.lock().unwrap(), ["Get /usuarios?usuario=beto "]);
    }

    #[tokio::test]
    async fn buscar_sin_texto_no_llama_al_servidor() {
        let c = guion(&[]);
        let r = buscar(
            &c,
            &con_sesion(),
            AHORA,
            &BuscarUsuarioDto {
                usuario: "  ".into(),
            },
        )
        .await;
        assert!(r.is_err());
        assert!(c.llamadas.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn la_lista_plana_se_separa_en_amigos_enviadas_y_recibidas() {
        let c = guion(&[(200, LISTA)]);
        let l = exito(listar_amistades(&c, &con_sesion(), AHORA).await);
        assert_eq!(l.amigos.len(), 1);
        assert_eq!(l.amigos[0].usuario.nombre_usuario, "beto");
        assert_eq!(l.amigos[0].amistad_id, "a1");
        assert_eq!(l.enviadas.len(), 1);
        assert_eq!(l.enviadas[0].usuario.nombre_usuario, "carla");
        assert_eq!(l.recibidas.len(), 1);
        assert_eq!(l.recibidas[0].usuario.nombre_usuario, "dario");
        assert_eq!(l.recibidas[0].estado, "pendiente");
    }

    #[tokio::test]
    async fn invitar_amigo_envia_el_usuario_y_lee_la_amistad_creada() {
        let c = guion(&[(
            201,
            r#"{"id":"a9","estado":"pendiente","otro_id":"u","otro_nombre_usuario":"beto","otro_nombre_visible":"Beto","enviada_por_mi":true}"#,
        )]);
        let r = exito(
            invitar_amigo(
                &c,
                &con_sesion(),
                AHORA,
                &InvitarAmigoDto {
                    usuario: "BETO".into(),
                },
            )
            .await,
        );
        assert_eq!((r.id.as_str(), r.estado.as_str()), ("a9", "pendiente"));
        assert_eq!(r.usuario.nombre_usuario, "beto");
        assert_eq!(
            *c.llamadas.lock().unwrap(),
            [r#"Post /amistades {"usuario":"beto"}"#]
        );
    }

    #[tokio::test]
    async fn responder_amistad_traduce_a_aceptar_y_completa_la_otra_persona() {
        let c = guion(&[
            (
                200,
                r#"{"id":"a3","estado":"aceptada","fecha_respuesta":"y"}"#,
            ),
            (200, LISTA),
        ]);
        let datos = ResponderAmistadDto {
            amistad_id: "a3".into(),
            respuesta: "aceptar".into(),
        };
        let r = exito(responder_amistad(&c, &con_sesion(), AHORA, &datos).await);
        assert_eq!(r.estado, "aceptada");
        assert_eq!(r.usuario.nombre_usuario, "dario");
        assert_eq!(
            c.llamadas.lock().unwrap()[0],
            r#"Post /amistades/a3/respuesta {"aceptar":true}"#
        );
    }

    #[tokio::test]
    async fn responder_con_un_valor_invalido_no_llama_al_servidor() {
        let c = guion(&[]);
        let datos = ResponderAmistadDto {
            amistad_id: "a3".into(),
            respuesta: "quizas".into(),
        };
        assert!(
            responder_amistad(&c, &con_sesion(), AHORA, &datos)
                .await
                .is_err()
        );
        assert!(c.llamadas.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn eliminar_una_amistad_espera_un_204() {
        let c = guion(&[(204, "")]);
        exito(eliminar_amistad(&c, &con_sesion(), AHORA, "a1").await);
        assert_eq!(*c.llamadas.lock().unwrap(), ["Delete /amistades/a1 "]);
    }

    #[tokio::test]
    async fn crear_y_listar_grupos_coinciden_con_los_dto() {
        let grupo = r#"{"id":"g1","nombre":"Viaje","descripcion":"","moneda":"USD","actividad":"marzo","rol":"administrador","cantidad_miembros":1}"#;
        let c = guion(&[(201, grupo)]);
        let nuevo = NuevoGrupoDto {
            nombre: "Viaje".into(),
            descripcion: String::new(),
            moneda: "USD".into(),
            actividad: "marzo".into(),
        };
        let g = exito(crear_grupo(&c, &con_sesion(), AHORA, &nuevo).await);
        assert_eq!(
            (g.id.as_str(), g.rol.as_str(), g.cantidad_miembros),
            ("g1", "administrador", 1)
        );

        let lista = format!(
            r#"{{"grupos":[{grupo}],"invitaciones":[{{"grupo_id":"g2","nombre":"Cena","moneda":"USD","actividad":"viernes","invitado_por":{{"nombre_usuario":"beto","nombre_visible":"Beto"}}}}]}}"#
        );
        let c = guion(&[(200, lista.as_str())]);
        let l = exito(listar_grupos(&c, &con_sesion(), AHORA).await);
        assert_eq!((l.grupos.len(), l.invitaciones.len()), (1, 1));
        assert_eq!(l.invitaciones[0].invitado_por.nombre_usuario, "beto");
    }

    #[tokio::test]
    async fn invitar_al_grupo_normaliza_el_estado_invitado_a_pendiente() {
        let c = guion(&[(
            201,
            r#"{"usuario":"luis","nombre_visible":"Luis","rol":"miembro","estado":"invitado"}"#,
        )]);
        let datos = InvitarAlGrupoDto {
            grupo_id: "g1".into(),
            usuario: "Luis".into(),
        };
        let r = exito(invitar_al_grupo(&c, &con_sesion(), AHORA, &datos).await);
        assert_eq!(
            (r.usuario.as_str(), r.estado.as_str()),
            ("luis", "pendiente")
        );
        assert_eq!(
            *c.llamadas.lock().unwrap(),
            [r#"Post /grupos/g1/miembros {"usuario":"luis"}"#]
        );
    }

    #[tokio::test]
    async fn responder_una_invitacion_de_grupo_arma_el_grupo_con_los_datos_de_la_invitacion() {
        let lista = r#"{"grupos":[],"invitaciones":[{"grupo_id":"g2","nombre":"Cena","moneda":"USD","actividad":"viernes","invitado_por":{"nombre_usuario":"beto","nombre_visible":"Beto"}}]}"#;
        let c = guion(&[(200, lista), (200, r#"{"mensaje":"Invitación aceptada."}"#)]);
        let datos = ResponderInvitacionGrupoDto {
            grupo_id: "g2".into(),
            respuesta: "aceptar".into(),
        };
        let g = exito(responder_invitacion_grupo(&c, &con_sesion(), AHORA, &datos).await);
        assert_eq!(
            (g.nombre.as_str(), g.moneda.as_str(), g.rol.as_str()),
            ("Cena", "USD", "miembro")
        );
        assert_eq!(
            c.llamadas.lock().unwrap()[1],
            r#"Post /grupos/g2/miembros/yo/respuesta {"respuesta":"aceptar"}"#
        );
    }

    const DETALLE: &str = r#"{"grupo":{"id":"g1","nombre":"Viaje","descripcion":"","moneda":"USD","actividad":"marzo","rol":"administrador","cantidad_miembros":2},
      "miembros":[{"usuario":"ana","nombre_visible":"Ana","rol":"administrador"},{"usuario":"beto","nombre_visible":"Beto","rol":"miembro"}],
      "gastos":[{"id":"x1","concepto":"Cena","monto":"100.00","fecha":"2026-11-12","pagador":"ana","regla":"partes_iguales","partes":[{"usuario":"ana","monto":"50.00","incluye_redondeo":false},{"usuario":"beto","monto":"50.00","incluye_redondeo":false}]}],
      "balances":[{"usuario":"ana","pago":"100.00","le_corresponde":"50.00","balance":"50.00"},{"usuario":"beto","pago":"0.00","le_corresponde":"50.00","balance":"-50.00"}]}"#;

    #[tokio::test]
    async fn ver_grupo_lee_el_detalle_con_gastos_y_balances() {
        let c = guion(&[(200, DETALLE)]);
        let d = exito(ver_grupo(&c, &con_sesion(), AHORA, "g1").await);
        assert_eq!(d.grupo.nombre, "Viaje");
        assert_eq!(d.miembros.len(), 2);
        assert_eq!(d.gastos.len(), 1);
        assert_eq!(d.gastos[0].partes.len(), 2);
        assert_eq!(d.balances[1].balance, "-50.00");
        assert_eq!(*c.llamadas.lock().unwrap(), ["Get /grupos/g1 "]);
    }

    #[tokio::test]
    async fn registrar_un_gasto_envia_el_cuerpo_del_servidor_y_lee_el_gasto() {
        let gasto = r#"{"id":"x9","concepto":"Cena","monto":"100.00","fecha":"2026-11-12","pagador":"ana","regla":"partes_iguales","partes":[{"usuario":"ana","monto":"33.34","incluye_redondeo":true},{"usuario":"beto","monto":"33.33","incluye_redondeo":false},{"usuario":"carla","monto":"33.33","incluye_redondeo":false}]}"#;
        let c = guion(&[(201, gasto)]);
        let datos = NuevoGastoGrupoDto {
            grupo_id: "g1".into(),
            concepto: "Cena".into(),
            monto: "100.00".into(),
            fecha: "2026-11-12".into(),
            pagador: "ana".into(),
            regla: "partes_iguales".into(),
            participantes: vec![crate::dto_social::ParticipanteDto {
                usuario: "ana".into(),
                valor: String::new(),
            }],
        };
        let g = exito(registrar_gasto(&c, &con_sesion(), AHORA, &datos).await);
        assert_eq!(g.id, "x9");
        assert!(g.partes[0].incluye_redondeo);
        let enviada = c.llamadas.lock().unwrap()[0].clone();
        assert!(enviada.starts_with("Post /grupos/g1/gastos "));
        assert!(enviada.contains(r#""pagador":"ana""#) && enviada.contains(r#""monto":"100.00""#));
        assert!(
            !enviada.contains("grupo_id"),
            "el grupo va en la ruta, no en el cuerpo"
        );
    }

    #[tokio::test]
    async fn un_gasto_que_no_cuadra_llega_con_la_diferencia_por_campo() {
        let cuerpo = r#"{"errores":[{"campo":"porcentajes","mensaje":"Los porcentajes suman 95; faltan 5."}]}"#;
        let c = guion(&[(400, cuerpo)]);
        let datos = NuevoGastoGrupoDto {
            grupo_id: "g1".into(),
            concepto: "Cena".into(),
            monto: "60.00".into(),
            fecha: "2026-11-12".into(),
            pagador: "beto".into(),
            regla: "porcentajes".into(),
            participantes: Vec::new(),
        };
        let Err(e) = registrar_gasto(&c, &con_sesion(), AHORA, &datos).await else {
            panic!("se esperaba un error");
        };
        assert_eq!(e.errores[0].campo.as_deref(), Some("porcentajes"));
        assert_eq!(e.errores[0].mensaje, "Los porcentajes suman 95; faltan 5.");
    }
}
