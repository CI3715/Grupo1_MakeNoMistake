//! Pruebas de los comandos de sesión con un cliente HTTP falso y un almacén en
//! memoria (mismo patrón de puerto y adaptador que `ping.rs`). Sin red ni
//! almacén de secretos del sistema.

use super::*;
use std::sync::Mutex;

const CLAVE: &str = "ClaveMuyPrivada123";
const TOKEN: &str = "eyJ.token.secreto";
const AHORA: &str = "2026-11-10T12:00:00Z";
const LEJOS: &str = "2099-01-01T00:00:00Z";
const USUARIO: &str = r#"{"id":"522b4761-a477-4fcf-91bc-4a6cf430262a","nombre_usuario":"cris","nombre_visible":"Cristina","correo":"cris@prueba.com","fecha_alta":"2026-11-10T04:32:52Z"}"#;
const SESION_NO_VALIDA: &str = r#"{"mensaje":"Sesión no válida o vencida."}"#;

// ---------- Dobles de prueba ----------

fn responde(codigo: u16, cuerpo: &str) -> Option<(u16, String)> {
    Some((codigo, cuerpo.to_string()))
}

fn token_json(vence: &str) -> String {
    format!(r#"{{"token":"{TOKEN}","tipo":"Bearer","vence_en":"{vence}"}}"#)
}

/// Cada campo es la respuesta de una ruta; `None` simula el servidor apagado.
#[derive(Default)]
struct ClienteFalso {
    registro: Option<(u16, String)>,
    inicio: Option<(u16, String)>,
    perfil: Option<(u16, String)>,
    llamadas: Mutex<Vec<String>>,
}

impl ClienteFalso {
    fn contestar(
        &self,
        nombre: &str,
        guion: &Option<(u16, String)>,
    ) -> Result<Respuesta, SinConexion> {
        self.llamadas.lock().unwrap().push(nombre.to_string());
        match guion {
            Some((codigo, cuerpo)) => Ok(Respuesta {
                codigo: *codigo,
                cuerpo: cuerpo.clone(),
            }),
            None => Err(SinConexion),
        }
    }

    fn llamadas(&self) -> Vec<String> {
        self.llamadas.lock().unwrap().clone()
    }
}

impl ClienteSesiones for ClienteFalso {
    async fn registrar(&self, _datos: &RegistroDto) -> Result<Respuesta, SinConexion> {
        self.contestar("registrar", &self.registro)
    }
    async fn iniciar(&self, _datos: &IniciarSesionDto) -> Result<Respuesta, SinConexion> {
        self.contestar("iniciar", &self.inicio)
    }
    async fn perfil(&self, token: &str) -> Result<Respuesta, SinConexion> {
        self.contestar(&format!("perfil:{token}"), &self.perfil)
    }
}

#[derive(Default)]
struct AlmacenMemoria {
    dentro: Mutex<Option<SesionGuardada>>,
    no_guarda: bool,
}

impl Almacen for AlmacenMemoria {
    fn guardar(&self, sesion: &SesionGuardada) -> Result<(), String> {
        if self.no_guarda {
            return Err("falla".to_string());
        }
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

// ---------- Ayudas ----------

fn registro() -> RegistroDto {
    RegistroDto {
        nombre_usuario: "cris".to_string(),
        nombre_visible: "Cristina".to_string(),
        correo: "cris@prueba.com".to_string(),
        contrasena: CLAVE.to_string(),
    }
}

fn login() -> IniciarSesionDto {
    IniciarSesionDto {
        correo: "cris@prueba.com".to_string(),
        contrasena: CLAVE.to_string(),
    }
}

fn sesion(vence_en: &str, invalidada: bool) -> SesionGuardada {
    SesionGuardada {
        token: TOKEN.to_string(),
        vence_en: vence_en.to_string(),
        nombre_visible: "Cristina".to_string(),
        invalidada,
    }
}

fn almacen_con(s: SesionGuardada) -> AlmacenMemoria {
    let a = AlmacenMemoria::default();
    a.guardar(&s).unwrap();
    a
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

fn exito<T>(r: Result<T, ErroresDto>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!(
            "no debía fallar: {}",
            serde_json::to_string(&e).unwrap_or_default()
        ),
    }
}

fn sin_conexion_msg() -> String {
    "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión."
        .to_string()
}

// ---------- Registro: 201, 400, 409 y servidor apagado ----------

#[tokio::test]
async fn registro_201_devuelve_el_usuario() {
    let c = ClienteFalso {
        registro: responde(201, USUARIO),
        ..Default::default()
    };
    let u = exito(registrar(&c, &registro()).await);
    assert_eq!(u.correo, "cris@prueba.com");
    assert_eq!(u.nombre_visible, "Cristina");
}

#[tokio::test]
async fn registro_400_llega_por_campo() {
    let cuerpo = r#"{"errores":[{"campo":"contrasena","mensaje":"La contraseña debe tener al menos 10 caracteres."},{"campo":"correo","mensaje":"El correo no tiene un formato válido."}]}"#;
    let c = ClienteFalso {
        registro: responde(400, cuerpo),
        ..Default::default()
    };
    let e = errores(registrar(&c, &registro()).await);
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].0.as_deref(), Some("contrasena"));
    assert_eq!(e[0].1, "La contraseña debe tener al menos 10 caracteres.");
    assert_eq!(e[1].0.as_deref(), Some("correo"));
}

#[tokio::test]
async fn registro_409_nombra_el_campo_repetido() {
    let cuerpo = r#"{"errores":[{"campo":"nombre_usuario","mensaje":"Ese nombre de usuario ya está registrado."},{"campo":"correo","mensaje":"Ese correo ya está registrado."}]}"#;
    let c = ClienteFalso {
        registro: responde(409, cuerpo),
        ..Default::default()
    };
    let e = errores(registrar(&c, &registro()).await);
    let campos: Vec<_> = e.iter().filter_map(|(c, _)| c.as_deref()).collect();
    assert_eq!(campos, ["nombre_usuario", "correo"]);
}

#[tokio::test]
async fn registro_con_servidor_apagado_da_un_mensaje_claro_sin_campo() {
    let c = ClienteFalso::default(); // registro: None
    let e = errores(registrar(&c, &registro()).await);
    assert_eq!(e, [(None, sin_conexion_msg())]);
}

#[tokio::test]
async fn registro_con_respuesta_inesperada_no_muestra_trazas() {
    for (codigo, cuerpo) in [(500, "Traceback (most recent call last)"), (400, "<html>")] {
        let c = ClienteFalso {
            registro: responde(codigo, cuerpo),
            ..Default::default()
        };
        let e = errores(registrar(&c, &registro()).await);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].0, None);
        assert!(!e[0].1.contains("Traceback") && !e[0].1.contains("<html>"));
    }
}

// ---------- Inicio de sesión: 200, 400, 401 y servidor apagado ----------

#[tokio::test]
async fn inicio_200_guarda_token_vencimiento_y_nombre_visible() {
    let c = ClienteFalso {
        inicio: responde(200, &token_json(LEJOS)),
        perfil: responde(200, USUARIO),
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let vista = exito(iniciar(&c, &a, &login()).await);
    assert_eq!(vista.estado, EstadoSesion::Vigente);
    assert_eq!(vista.nombre_visible.as_deref(), Some("Cristina"));
    assert_eq!(vista.vence_en.as_deref(), Some(LEJOS));
    let guardada = a.leer().expect("la sesión debía guardarse");
    assert_eq!(guardada.token, TOKEN);
    assert_eq!(guardada.vence_en, LEJOS);
    assert_eq!(guardada.nombre_visible, "Cristina");
    assert!(!guardada.invalidada);
    // El perfil se pide con el token recién emitido.
    assert_eq!(
        c.llamadas(),
        ["iniciar".to_string(), format!("perfil:{TOKEN}")]
    );
}

#[tokio::test]
async fn inicio_401_credenciales_incorrectas_sin_campo_y_sin_guardar() {
    let c = ClienteFalso {
        inicio: responde(401, r#"{"mensaje":"Correo o contraseña incorrectos."}"#),
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(e, [(None, "Correo o contraseña incorrectos.".to_string())]);
    assert!(a.leer().is_none());
    assert_eq!(c.llamadas(), ["iniciar"], "no se pide el perfil sin token");
}

#[tokio::test]
async fn inicio_400_llega_por_campo() {
    let cuerpo = r#"{"errores":[{"campo":"correo","mensaje":"Este campo es obligatorio."}]}"#;
    let c = ClienteFalso {
        inicio: responde(400, cuerpo),
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(e[0].0.as_deref(), Some("correo"));
    assert!(a.leer().is_none());
}

#[tokio::test]
async fn inicio_con_servidor_apagado_no_guarda_nada() {
    let c = ClienteFalso::default();
    let a = AlmacenMemoria::default();
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(e, [(None, sin_conexion_msg())]);
    assert!(a.leer().is_none());
}

#[tokio::test]
async fn si_el_servidor_cae_al_pedir_el_perfil_no_se_guarda_sesion() {
    let c = ClienteFalso {
        inicio: responde(200, &token_json(LEJOS)),
        perfil: None,
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(e, [(None, sin_conexion_msg())]);
    assert!(a.leer().is_none());
}

#[tokio::test]
async fn si_el_perfil_responde_401_justo_despues_de_iniciar_es_inesperado() {
    let c = ClienteFalso {
        inicio: responde(200, &token_json(LEJOS)),
        perfil: responde(401, SESION_NO_VALIDA),
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(e.len(), 1);
    assert!(a.leer().is_none());
}

#[tokio::test]
async fn si_no_se_puede_guardar_la_sesion_se_avisa() {
    let c = ClienteFalso {
        inicio: responde(200, &token_json(LEJOS)),
        perfil: responde(200, USUARIO),
        ..Default::default()
    };
    let a = AlmacenMemoria {
        no_guarda: true,
        ..Default::default()
    };
    let e = errores(iniciar(&c, &a, &login()).await);
    assert_eq!(
        e,
        [(
            None,
            "No se pudo guardar la sesión en este dispositivo.".to_string()
        )]
    );
}

// ---------- Perfil: 200, 401 (token vencido) y sin red ----------

#[tokio::test]
async fn perfil_200_con_el_token_guardado() {
    let c = ClienteFalso {
        perfil: responde(200, USUARIO),
        ..Default::default()
    };
    let a = almacen_con(sesion(LEJOS, false));
    let u = exito(perfil(&c, &a, AHORA).await);
    assert_eq!(u.nombre_usuario, "cris");
    assert_eq!(c.llamadas(), [format!("perfil:{TOKEN}")]);
}

#[tokio::test]
async fn perfil_401_marca_la_sesion_como_vencida_y_la_conserva() {
    let c = ClienteFalso {
        perfil: responde(401, SESION_NO_VALIDA),
        ..Default::default()
    };
    let a = almacen_con(sesion(LEJOS, false));
    let e = errores(perfil(&c, &a, AHORA).await);
    assert_eq!(
        e,
        [(
            None,
            "Tu sesión venció, inicia sesión de nuevo.".to_string()
        )]
    );
    assert!(a.leer().expect("se conserva").invalidada);
    let vista = estado(&a, AHORA);
    assert_eq!(vista.estado, EstadoSesion::Vencida);
    assert_eq!(vista.nombre_visible.as_deref(), Some("Cristina"));
}

#[tokio::test]
async fn perfil_con_token_vencido_por_reloj_no_llama_al_servidor() {
    let c = ClienteFalso {
        perfil: responde(200, USUARIO),
        ..Default::default()
    };
    let a = almacen_con(sesion("2026-11-10T11:00:00Z", false));
    let e = errores(perfil(&c, &a, AHORA).await);
    assert_eq!(e[0].1, "Tu sesión venció, inicia sesión de nuevo.");
    assert!(c.llamadas().is_empty());
}

#[tokio::test]
async fn perfil_sin_sesion_pide_iniciar_sesion() {
    let c = ClienteFalso::default();
    let e = errores(perfil(&c, &AlmacenMemoria::default(), AHORA).await);
    assert_eq!(e, [(None, "No has iniciado sesión.".to_string())]);
    assert!(c.llamadas().is_empty());
}

#[tokio::test]
async fn perfil_con_servidor_apagado_conserva_la_sesion() {
    let c = ClienteFalso::default();
    let a = almacen_con(sesion(LEJOS, false));
    let e = errores(perfil(&c, &a, AHORA).await);
    assert_eq!(e, [(None, sin_conexion_msg())]);
    assert!(!a.leer().expect("sigue guardada").invalidada);
}

#[tokio::test]
async fn perfil_con_respuesta_inesperada_es_un_mensaje_general() {
    let c = ClienteFalso {
        perfil: responde(500, "boom"),
        ..Default::default()
    };
    let a = almacen_con(sesion(LEJOS, false));
    let e = errores(perfil(&c, &a, AHORA).await);
    assert_eq!(e.len(), 1);
    assert!(
        !a.leer().unwrap().invalidada,
        "un 500 no invalida la sesión"
    );
}

// ---------- Estado de la sesión (sin red) y cerrar sesión ----------

#[test]
fn el_estado_se_decide_sin_red() {
    let vacio = AlmacenMemoria::default();
    assert_eq!(estado(&vacio, AHORA).estado, EstadoSesion::SinSesion);
    assert!(estado(&vacio, AHORA).nombre_visible.is_none());

    let vigente = almacen_con(sesion(LEJOS, false));
    let v = estado(&vigente, AHORA);
    assert_eq!(v.estado, EstadoSesion::Vigente);
    assert_eq!(v.nombre_visible.as_deref(), Some("Cristina"));

    let vencida = almacen_con(sesion("2026-11-10T11:59:59Z", false));
    assert_eq!(estado(&vencida, AHORA).estado, EstadoSesion::Vencida);

    // Justo en el instante de vencimiento ya no vale.
    let limite = almacen_con(sesion(AHORA, false));
    assert_eq!(estado(&limite, AHORA).estado, EstadoSesion::Vencida);

    let invalidada = almacen_con(sesion(LEJOS, true));
    assert_eq!(estado(&invalidada, AHORA).estado, EstadoSesion::Vencida);
}

#[test]
fn cerrar_sesion_borra_todo() {
    let a = almacen_con(sesion(LEJOS, false));
    a.borrar();
    assert!(a.leer().is_none());
    assert_eq!(estado(&a, AHORA).estado, EstadoSesion::SinSesion);
}

#[test]
fn la_hora_actual_tiene_el_formato_del_servidor() {
    let h = ahora_iso();
    let b = h.as_bytes();
    assert_eq!(h.len(), 20, "AAAA-MM-DDThh:mm:ssZ: {h}");
    assert!(b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':');
    assert!(h.ends_with('Z'));
    assert!(
        h.as_str() >= "2026-11-10T00:00:00Z",
        "el reloj no puede ir atrás: {h}"
    );
    assert_eq!(estado_de(&sesion(LEJOS, false), &h), EstadoSesion::Vigente);
}

// ---------- Contraseña y token: ni en Debug, ni en errores, ni en logs ----------

#[tokio::test]
async fn ni_la_contrasena_ni_el_token_salen_en_los_resultados() {
    let c = ClienteFalso {
        inicio: responde(200, &token_json(LEJOS)),
        perfil: responde(200, USUARIO),
        ..Default::default()
    };
    let a = AlmacenMemoria::default();
    let vista = exito(iniciar(&c, &a, &login()).await);
    let usuario = exito(perfil(&c, &a, AHORA).await);
    let mostrado = format!("{vista:?} {usuario:?}");
    let enviado = serde_json::to_string(&vista).unwrap();
    for texto in [mostrado, enviado] {
        assert!(!texto.contains(CLAVE), "la contraseña no debe verse");
        assert!(
            !texto.contains(TOKEN),
            "el token no debe llegar a la interfaz"
        );
    }
}

#[tokio::test]
async fn si_el_servidor_repite_el_valor_enviado_no_llega_a_la_interfaz() {
    let cuerpo = format!(
        r#"{{"errores":[{{"campo":"contrasena","mensaje":"La contraseña debe tener al menos 10 caracteres.","valor":"{CLAVE}"}}]}}"#
    );
    let c = ClienteFalso {
        registro: responde(400, &cuerpo),
        inicio: responde(400, &cuerpo),
        ..Default::default()
    };
    let Err(e1) = registrar(&c, &registro()).await else {
        panic!("se esperaba un error");
    };
    let Err(e2) = iniciar(&c, &AlmacenMemoria::default(), &login()).await else {
        panic!("se esperaba un error");
    };
    for e in [e1, e2] {
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains(CLAVE));
    }
}

/// Líneas `#[derive(...)]` de la estructura declarada con `declaracion`.
fn derives_de(fuente: &str, declaracion: &str) -> String {
    let lineas: Vec<&str> = fuente.lines().collect();
    let i = lineas
        .iter()
        .position(|l| l.trim_start().starts_with(declaracion))
        .unwrap_or_else(|| panic!("no se encontró `{declaracion}`"));
    lineas[..i]
        .iter()
        .rev()
        .take_while(|l| {
            let t = l.trim_start();
            t.starts_with("#[") || t.starts_with("///")
        })
        .filter(|l| l.trim_start().starts_with("#[derive"))
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn las_estructuras_con_secretos_no_derivan_debug() {
    let dto = include_str!("../dto.rs");
    let sesion_rs = include_str!("../sesion.rs");
    for (fuente, decl) in [
        (dto, "pub struct RegistroDto {"),
        (dto, "pub struct IniciarSesionDto {"),
        (sesion_rs, "pub struct SesionGuardada {"),
    ] {
        let derives = derives_de(fuente, decl);
        assert!(!derives.is_empty(), "{decl} debía tener derive");
        assert!(
            !derives.contains("Debug"),
            "{decl} no puede derivar Debug: {derives}"
        );
    }
}

#[test]
fn ningun_log_incluye_contrasena_ni_token_ni_cuerpos() {
    let fuente = include_str!("../sesion.rs");
    for linea in fuente.lines().filter(|l| l.contains("log::")) {
        for prohibido in ["contrasena", "token", "datos", "cuerpo", "{"] {
            assert!(
                !linea.contains(prohibido),
                "el log `{}` no puede incluir `{prohibido}`",
                linea.trim()
            );
        }
    }
}

// ---------- Respaldo en archivo ----------

#[test]
fn el_respaldo_en_archivo_se_escribe_con_permisos_privados() {
    let carpeta = std::env::temp_dir().join(format!("cuentas-claras-{}", uuid::Uuid::new_v4()));
    let ruta = carpeta.join("sesion.json");
    escribir_privado(&ruta, r#"{"a":1}"#).expect("debía escribirse");
    assert_eq!(std::fs::read_to_string(&ruta).unwrap(), r#"{"a":1}"#);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let modo = std::fs::metadata(&ruta).unwrap().permissions().mode() & 0o777;
        assert_eq!(modo, 0o600);
    }
    let _ = std::fs::remove_dir_all(&carpeta);
}

#[test]
fn una_sesion_guardada_sin_el_campo_invalidada_se_lee_como_valida() {
    let json = format!(r#"{{"token":"{TOKEN}","vence_en":"{LEJOS}","nombre_visible":"Cristina"}}"#);
    let s: SesionGuardada = serde_json::from_str(&json).unwrap();
    assert!(!s.invalidada);
}
