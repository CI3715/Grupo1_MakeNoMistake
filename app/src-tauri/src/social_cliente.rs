//! Base de los comandos reales de amistades, grupos y gastos (Entrega 6).
//!
//! Cada comando real arma una `Peticion` y la manda con `llamar`, que se ocupa de lo común:
//!
//! - Sin sesión: no se llama al servidor y se pide iniciar sesión.
//! - Sesión vencida por reloj: tampoco se llama.
//! - El token guardado va en el encabezado `Authorization: Bearer`.
//! - Un 401 marca la sesión como vencida (igual que `obtener_perfil`).
//! - Servidor apagado o sin red: mensaje claro, sin tocar la sesión.
//! - 400, 403, 404 y 409 se traducen a `{campo, mensaje}` en español.
//!
//! Este archivo no usa la red de forma directa: el adaptador HTTP real es
//! `ClienteHttp::enviar_con_token`, en `sesion.rs`, el único lugar permitido
//! junto con `ping.rs` (INV-16, RNF-01). Los cuerpos que se envían son solo
//! datos de amistades, grupos y gastos compartidos.

use std::future::Future;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::dto::{ErroresDto, EstadoSesion};
use crate::sesion::{
    Almacen, ClienteHttp, Respuesta, SinConexion, errores_del_servidor, estado_de, inesperado,
    sesion_vencida, sin_campo, sin_conexion,
};

// ---------- La petición ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metodo {
    Get,
    Post,
    Delete,
}

impl Metodo {
    fn texto(self) -> &'static str {
        match self {
            Metodo::Get => "GET",
            Metodo::Post => "POST",
            Metodo::Delete => "DELETE",
        }
    }
}

/// Una petición al servidor. `ruta` empieza con «/» y ya viene codificada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peticion {
    pub metodo: Metodo,
    pub ruta: String,
    pub cuerpo: Option<String>,
}

impl Peticion {
    pub fn get(ruta: &str) -> Self {
        Peticion {
            metodo: Metodo::Get,
            ruta: ruta.to_string(),
            cuerpo: None,
        }
    }

    pub fn delete(ruta: &str) -> Self {
        Peticion {
            metodo: Metodo::Delete,
            ruta: ruta.to_string(),
            cuerpo: None,
        }
    }

    pub fn post<T: Serialize>(ruta: &str, cuerpo: &T) -> Self {
        Peticion {
            metodo: Metodo::Post,
            ruta: ruta.to_string(),
            cuerpo: serde_json::to_string(cuerpo).ok(),
        }
    }
}

/// Codifica un texto para ponerlo en una dirección: `a b&ñ` -> `a%20b%26%C3%B1`.
pub fn codificar(texto: &str) -> String {
    let mut fuera = String::new();
    for b in texto.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            fuera.push(char::from(b));
        } else {
            fuera.push_str(&format!("%{b:02X}"));
        }
    }
    fuera
}

// ---------- Puerto ----------

/// Lo que debe saber hacer cualquier cliente del servidor para estas rutas.
pub trait ClienteSocial {
    fn enviar_peticion(
        &self,
        peticion: &Peticion,
        token: &str,
    ) -> impl Future<Output = Result<Respuesta, SinConexion>> + Send;
}

/// ADAPTADOR REAL: delega en `ClienteHttp` (sesion.rs).
impl ClienteSocial for ClienteHttp {
    async fn enviar_peticion(
        &self,
        peticion: &Peticion,
        token: &str,
    ) -> Result<Respuesta, SinConexion> {
        self.enviar_con_token(
            peticion.metodo.texto(),
            &peticion.ruta,
            token,
            peticion.cuerpo.as_deref(),
        )
        .await
    }
}

// ---------- Casos de uso comunes ----------

/// Token de la sesión vigente, o el error que corresponde.
pub fn token_vigente<A: Almacen>(almacen: &A, ahora: &str) -> Result<String, ErroresDto> {
    let Some(sesion) = almacen.leer() else {
        return Err(sin_campo("No has iniciado sesión."));
    };
    if estado_de(&sesion, ahora) == EstadoSesion::Vencida {
        return Err(sesion_vencida());
    }
    Ok(sesion.token)
}

/// Marca la sesión guardada como vencida y la conserva (así se recuerda el nombre).
fn marcar_vencida<A: Almacen>(almacen: &A) {
    if let Some(mut sesion) = almacen.leer() {
        sesion.invalidada = true;
        let _ = almacen.guardar(&sesion);
    }
}

fn traducir<A: Almacen>(almacen: &A, respuesta: &Respuesta) -> ErroresDto {
    match respuesta.codigo {
        401 => {
            marcar_vencida(almacen);
            sesion_vencida()
        }
        400 | 403 | 404 | 409 => errores_del_servidor(respuesta),
        _ => inesperado(),
    }
}

/// Hace la petición con el token guardado. Devuelve la respuesta si su código está
/// en `exitos` (por ejemplo `[200]` o `[201]`); si no, el error ya traducido.
pub async fn llamar<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    peticion: &Peticion,
    exitos: &[u16],
) -> Result<Respuesta, ErroresDto> {
    let token = token_vigente(almacen, ahora)?;
    let respuesta = cliente
        .enviar_peticion(peticion, &token)
        .await
        .map_err(|_| sin_conexion())?;
    if exitos.contains(&respuesta.codigo) {
        return Ok(respuesta);
    }
    Err(traducir(almacen, &respuesta))
}

/// Lee el cuerpo de una respuesta exitosa.
pub fn leer<T: DeserializeOwned>(respuesta: &Respuesta) -> Result<T, ErroresDto> {
    serde_json::from_str(&respuesta.cuerpo).map_err(|_| inesperado())
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dto_social::InvitarAmigoDto;
    use crate::sesion::SesionGuardada;
    use std::sync::Mutex;

    const AHORA: &str = "2026-11-10T12:00:00Z";
    const LEJOS: &str = "2099-01-01T00:00:00Z";
    const TOKEN: &str = "token.de.prueba";

    #[derive(Default)]
    struct Falso {
        respuesta: Option<(u16, String)>,
        llamadas: Mutex<Vec<String>>,
    }

    impl ClienteSocial for Falso {
        async fn enviar_peticion(
            &self,
            peticion: &Peticion,
            token: &str,
        ) -> Result<Respuesta, SinConexion> {
            self.llamadas.lock().unwrap().push(format!(
                "{} {} con {token}",
                peticion.metodo.texto(),
                peticion.ruta
            ));
            match &self.respuesta {
                Some((codigo, cuerpo)) => Ok(Respuesta {
                    codigo: *codigo,
                    cuerpo: cuerpo.clone(),
                }),
                None => Err(SinConexion),
            }
        }
    }

    #[derive(Default)]
    struct Memoria {
        dentro: Mutex<Option<SesionGuardada>>,
    }

    impl Almacen for Memoria {
        fn guardar(&self, sesion: &SesionGuardada) -> Result<(), String> {
            *self.dentro.lock().unwrap() = Some(sesion.clone());
            Ok(())
        }
        fn leer(&self) -> Option<SesionGuardada> {
            self.dentro.lock().unwrap().clone()
        }
        fn borrar(&self) {
            *self.dentro.lock().unwrap() = None;
        }
    }

    fn con_sesion(vence_en: &str) -> Memoria {
        let m = Memoria::default();
        m.guardar(&SesionGuardada {
            token: TOKEN.to_string(),
            vence_en: vence_en.to_string(),
            nombre_visible: "Ana".to_string(),
            invalidada: false,
        })
        .unwrap();
        m
    }

    fn falso(codigo: u16, cuerpo: &str) -> Falso {
        Falso {
            respuesta: Some((codigo, cuerpo.to_string())),
            ..Default::default()
        }
    }

    type Fallos = Vec<(Option<String>, String)>;

    fn errores<T>(r: Result<T, ErroresDto>) -> Fallos {
        match r {
            Ok(_) => panic!("se esperaban errores"),
            Err(e) => e
                .errores
                .into_iter()
                .map(|c| (c.campo, c.mensaje))
                .collect(),
        }
    }

    #[tokio::test]
    async fn sin_sesion_no_se_llama_al_servidor() {
        let c = falso(200, "{}");
        let e = errores(
            llamar(
                &c,
                &Memoria::default(),
                AHORA,
                &Peticion::get("/grupos"),
                &[200],
            )
            .await,
        );
        assert_eq!(e, [(None, "No has iniciado sesión.".to_string())]);
        assert!(c.llamadas.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn con_la_sesion_vencida_por_reloj_no_se_llama_al_servidor() {
        let c = falso(200, "{}");
        let a = con_sesion("2026-11-10T11:00:00Z");
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/grupos"), &[200]).await);
        assert_eq!(e[0].1, "Tu sesión venció, inicia sesión de nuevo.");
        assert!(c.llamadas.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn una_respuesta_exitosa_viaja_con_el_token_guardado() {
        let c = falso(200, r#"{"grupos":[]}"#);
        let a = con_sesion(LEJOS);
        let Ok(r) = llamar(&c, &a, AHORA, &Peticion::get("/grupos"), &[200]).await else {
            panic!("debía funcionar");
        };
        assert_eq!(r.cuerpo, r#"{"grupos":[]}"#);
        assert_eq!(
            *c.llamadas.lock().unwrap(),
            [format!("GET /grupos con {TOKEN}")]
        );
    }

    #[tokio::test]
    async fn un_401_marca_la_sesion_como_vencida_y_la_conserva() {
        let c = falso(401, r#"{"mensaje":"Sesión no válida o vencida."}"#);
        let a = con_sesion(LEJOS);
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/amistades"), &[200]).await);
        assert_eq!(
            e,
            [(
                None,
                "Tu sesión venció, inicia sesión de nuevo.".to_string()
            )]
        );
        let guardada = a.leer().expect("se conserva");
        assert!(guardada.invalidada);
        assert_eq!(guardada.nombre_visible, "Ana");
    }

    #[tokio::test]
    async fn los_errores_del_servidor_llegan_por_campo_o_generales() {
        let a = con_sesion(LEJOS);
        let c = falso(
            409,
            r#"{"errores":[{"campo":"usuario","mensaje":"Ya son amigos."}]}"#,
        );
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/x"), &[201]).await);
        assert_eq!(
            e,
            [(Some("usuario".to_string()), "Ya son amigos.".to_string())]
        );

        let c = falso(
            403,
            r#"{"mensaje":"Solo el administrador puede invitar al grupo."}"#,
        );
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/x"), &[201]).await);
        assert_eq!(
            e,
            [(
                None,
                "Solo el administrador puede invitar al grupo.".to_string()
            )]
        );

        let c = falso(404, r#"{"mensaje":"Grupo no encontrado."}"#);
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/x"), &[200]).await);
        assert_eq!(e[0].1, "Grupo no encontrado.");
        assert!(
            !a.leer().unwrap().invalidada,
            "un 404 no invalida la sesión"
        );
    }

    #[tokio::test]
    async fn servidor_apagado_da_un_mensaje_claro_y_no_toca_la_sesion() {
        let c = Falso::default();
        let a = con_sesion(LEJOS);
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/grupos"), &[200]).await);
        assert_eq!(
            e,
            [(None, "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión.".to_string())]
        );
        assert!(!a.leer().unwrap().invalidada);
    }

    #[tokio::test]
    async fn una_respuesta_inesperada_no_muestra_trazas() {
        let c = falso(500, "Traceback (most recent call last)");
        let a = con_sesion(LEJOS);
        let e = errores(llamar(&c, &a, AHORA, &Peticion::get("/grupos"), &[200]).await);
        assert_eq!(e.len(), 1);
        assert!(!e[0].1.contains("Traceback"));
        assert!(!a.leer().unwrap().invalidada);
    }

    #[test]
    fn las_direcciones_se_codifican_y_el_cuerpo_se_serializa() {
        assert_eq!(codificar("ana"), "ana");
        assert_eq!(codificar("a b&ñ"), "a%20b%26%C3%B1");
        let p = Peticion::post(
            "/amistades",
            &InvitarAmigoDto {
                usuario: "beto".into(),
            },
        );
        assert_eq!(p.cuerpo.as_deref(), Some(r#"{"usuario":"beto"}"#));
        assert_eq!(p.metodo, Metodo::Post);
    }
}
