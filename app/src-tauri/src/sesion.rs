//! Comandos de registro e inicio de sesión (Entrega 5).
//!
//! VERSIÓN CON DATOS FIJOS: no hablan con el servidor. Sirven para construir y
//! probar la interfaz. Los correos de prueba disparan cada caso:
//!   sin-conexion@prueba.com      servidor apagado (registro e inicio de sesión)
//!   usuario-repetido@prueba.com  nombre de usuario ya registrado (registro)
//!   correo-repetido@prueba.com   correo ya registrado (registro)
//!   incorrecto@prueba.com        credenciales incorrectas (inicio de sesión)
//!   vencido@prueba.com           inicia sesión, pero luego la sesión figura vencida
//! Además se rechazan por campo: nombre visible vacío, correo sin «@» y contraseña
//! de menos de 10 caracteres. Cualquier otro dato funciona.

use std::sync::{Mutex, MutexGuard};

use crate::dto::{
    ErrorCampo, ErroresDto, EstadoSesion, IniciarSesionDto, RegistroDto, SesionDto, UsuarioDto,
};

const NOMBRE_VISIBLE: &str = "Ana Pérez";
const ID_FIJO: &str = "3f2b8c1e-6a4d-4b7e-9c10-2d5e7a9b0c11";
const FECHA_ALTA: &str = "2026-11-06T12:00:00Z";
const VENCE_EN: &str = "2099-01-01T00:00:00Z";
const VENCIDO_EN: &str = "2020-01-01T00:00:00Z";
const SIN_CONEXION: &str = "sin-conexion@prueba.com";

#[derive(Debug, Clone)]
struct SesionFija {
    correo: String,
    nombre_visible: String,
    vencida: bool,
}

static SESION_FIJA: Mutex<Option<SesionFija>> = Mutex::new(None);

fn sesion_fija() -> MutexGuard<'static, Option<SesionFija>> {
    SESION_FIJA.lock().unwrap_or_else(|e| e.into_inner())
}

fn falla(campo: &str, mensaje: &str) -> ErrorCampo {
    ErrorCampo {
        campo: Some(campo.to_string()),
        mensaje: mensaje.to_string(),
    }
}

fn sin_campo(mensaje: &str) -> ErroresDto {
    ErroresDto {
        errores: vec![ErrorCampo {
            campo: None,
            mensaje: mensaje.to_string(),
        }],
    }
}

fn sin_conexion() -> ErroresDto {
    sin_campo(
        "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión.",
    )
}

fn registrar(datos: &RegistroDto) -> Result<UsuarioDto, ErroresDto> {
    let correo = datos.correo.trim().to_lowercase();
    if correo == SIN_CONEXION {
        return Err(sin_conexion());
    }
    let mut errores = Vec::new();
    if datos.nombre_visible.trim().is_empty() {
        errores.push(falla("nombre_visible", "El nombre visible es obligatorio."));
    }
    if !correo.contains('@') {
        errores.push(falla("correo", "El correo no tiene un formato válido."));
    }
    if datos.contrasena.chars().count() < 10 {
        errores.push(falla(
            "contrasena",
            "La contraseña debe tener al menos 10 caracteres.",
        ));
    }
    if correo == "usuario-repetido@prueba.com" {
        errores.push(falla(
            "nombre_usuario",
            "Ese nombre de usuario ya está registrado.",
        ));
    }
    if correo == "correo-repetido@prueba.com" {
        errores.push(falla("correo", "Ese correo ya está registrado."));
    }
    if !errores.is_empty() {
        return Err(ErroresDto { errores });
    }
    Ok(UsuarioDto {
        id: ID_FIJO.to_string(),
        nombre_usuario: datos.nombre_usuario.trim().to_string(),
        nombre_visible: datos.nombre_visible.trim().to_string(),
        correo,
        fecha_alta: FECHA_ALTA.to_string(),
    })
}

fn iniciar(
    datos: &IniciarSesionDto,
    sesion: &mut Option<SesionFija>,
) -> Result<SesionDto, ErroresDto> {
    let correo = datos.correo.trim().to_lowercase();
    if correo == SIN_CONEXION {
        return Err(sin_conexion());
    }
    let mut errores = Vec::new();
    if correo.is_empty() {
        errores.push(falla("correo", "El correo es obligatorio."));
    }
    if datos.contrasena.is_empty() {
        errores.push(falla("contrasena", "La contraseña es obligatoria."));
    }
    if !errores.is_empty() {
        return Err(ErroresDto { errores });
    }
    if correo == "incorrecto@prueba.com" {
        return Err(sin_campo("Correo o contraseña incorrectos."));
    }
    let vencida = correo == "vencido@prueba.com";
    *sesion = Some(SesionFija {
        correo,
        nombre_visible: NOMBRE_VISIBLE.to_string(),
        vencida,
    });
    Ok(SesionDto {
        estado: EstadoSesion::Vigente,
        nombre_visible: Some(NOMBRE_VISIBLE.to_string()),
        vence_en: Some(VENCE_EN.to_string()),
    })
}

fn estado(sesion: Option<&SesionFija>) -> SesionDto {
    match sesion {
        None => SesionDto {
            estado: EstadoSesion::SinSesion,
            nombre_visible: None,
            vence_en: None,
        },
        Some(s) if s.vencida => SesionDto {
            estado: EstadoSesion::Vencida,
            nombre_visible: Some(s.nombre_visible.clone()),
            vence_en: Some(VENCIDO_EN.to_string()),
        },
        Some(s) => SesionDto {
            estado: EstadoSesion::Vigente,
            nombre_visible: Some(s.nombre_visible.clone()),
            vence_en: Some(VENCE_EN.to_string()),
        },
    }
}

fn perfil(sesion: Option<&SesionFija>) -> Result<UsuarioDto, ErroresDto> {
    match sesion {
        None => Err(sin_campo("No has iniciado sesión.")),
        Some(s) if s.vencida => Err(sin_campo("Tu sesión venció, inicia sesión de nuevo.")),
        Some(s) => Ok(UsuarioDto {
            id: ID_FIJO.to_string(),
            nombre_usuario: "ana.perez".to_string(),
            nombre_visible: s.nombre_visible.clone(),
            correo: s.correo.clone(),
            fecha_alta: FECHA_ALTA.to_string(),
        }),
    }
}

/// POST /usuarios. Errores por campo: nombre_usuario, nombre_visible, correo, contrasena.
#[tauri::command]
pub async fn registrar_usuario(datos: RegistroDto) -> Result<UsuarioDto, ErroresDto> {
    registrar(&datos)
}

/// POST /sesiones. Con éxito guarda el token en el dispositivo y devuelve el estado.
#[tauri::command]
pub async fn iniciar_sesion(datos: IniciarSesionDto) -> Result<SesionDto, ErroresDto> {
    iniciar(&datos, &mut sesion_fija())
}

/// GET /usuarios/yo con el token guardado.
#[tauri::command]
pub async fn obtener_perfil() -> Result<UsuarioDto, ErroresDto> {
    perfil(sesion_fija().as_ref())
}

/// Borra el token y los datos de sesión del dispositivo.
#[tauri::command]
pub async fn cerrar_sesion() -> Result<(), ErroresDto> {
    *sesion_fija() = None;
    Ok(())
}

/// Al abrir la app: ¿hay sesión vigente, vencida o ninguna? No necesita red.
#[tauri::command]
pub async fn estado_sesion() -> Result<SesionDto, ErroresDto> {
    Ok(estado(sesion_fija().as_ref()))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn registro(correo: &str, contrasena: &str, visible: &str) -> RegistroDto {
        RegistroDto {
            nombre_usuario: "ana.perez".to_string(),
            nombre_visible: visible.to_string(),
            correo: correo.to_string(),
            contrasena: contrasena.to_string(),
        }
    }

    fn login(correo: &str, contrasena: &str) -> IniciarSesionDto {
        IniciarSesionDto {
            correo: correo.to_string(),
            contrasena: contrasena.to_string(),
        }
    }

    /// Pares (campo, mensaje) de un resultado que debe ser un error.
    fn errores_de<T>(r: Result<T, ErroresDto>) -> Vec<(Option<String>, String)> {
        match r {
            Ok(_) => panic!("se esperaban errores"),
            Err(e) => e
                .errores
                .into_iter()
                .map(|c| (c.campo, c.mensaje))
                .collect(),
        }
    }

    #[test]
    fn un_registro_valido_devuelve_el_usuario() {
        let Ok(u) = registrar(&registro("Ana@Prueba.com", "una-clave-larga", " Ana ")) else {
            panic!("el registro debía funcionar");
        };
        assert_eq!(u.correo, "ana@prueba.com");
        assert_eq!(u.nombre_visible, "Ana");
    }

    #[test]
    fn los_errores_de_registro_llegan_juntos_y_por_campo() {
        let e = errores_de(registrar(&registro("sin-arroba", "corta", "  ")));
        let campos: Vec<_> = e.iter().filter_map(|(c, _)| c.as_deref()).collect();
        assert_eq!(campos, ["nombre_visible", "correo", "contrasena"]);
    }

    #[test]
    fn los_repetidos_nombran_su_campo() {
        let e = errores_de(registrar(&registro(
            "usuario-repetido@prueba.com",
            "una-clave-larga",
            "Ana",
        )));
        assert_eq!(e[0].0.as_deref(), Some("nombre_usuario"));
        let e = errores_de(registrar(&registro(
            "correo-repetido@prueba.com",
            "una-clave-larga",
            "Ana",
        )));
        assert_eq!(e[0].0.as_deref(), Some("correo"));
    }

    #[test]
    fn sin_conexion_y_credenciales_incorrectas_no_llevan_campo() {
        let e = errores_de(registrar(&registro(SIN_CONEXION, "una-clave-larga", "Ana")));
        assert_eq!(e[0].0, None);
        let e = errores_de(iniciar(&login(SIN_CONEXION, "x"), &mut None));
        assert_eq!(e[0].0, None);
        let e = errores_de(iniciar(&login("incorrecto@prueba.com", "x"), &mut None));
        assert_eq!(e, [(None, "Correo o contraseña incorrectos.".to_string())]);
    }

    #[test]
    fn iniciar_sesion_deja_la_sesion_vigente_y_se_puede_pedir_el_perfil() {
        let mut sesion = None;
        assert_eq!(estado(sesion.as_ref()).estado, EstadoSesion::SinSesion);
        assert!(iniciar(&login("ana@prueba.com", "x"), &mut sesion).is_ok());
        assert_eq!(estado(sesion.as_ref()).estado, EstadoSesion::Vigente);
        assert!(perfil(sesion.as_ref()).is_ok());
    }

    #[test]
    fn la_sesion_vencida_se_ve_vencida_y_el_perfil_responde_error() {
        let mut sesion = None;
        assert!(iniciar(&login("vencido@prueba.com", "x"), &mut sesion).is_ok());
        assert_eq!(estado(sesion.as_ref()).estado, EstadoSesion::Vencida);
        let e = errores_de(perfil(sesion.as_ref()));
        assert_eq!(e[0].1, "Tu sesión venció, inicia sesión de nuevo.");
    }
}
