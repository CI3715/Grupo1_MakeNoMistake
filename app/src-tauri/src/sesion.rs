//! Comandos de registro e inicio de sesión (Entrega 5).
//!
//! De momento solo están las firmas: el contrato que usan la interfaz y las
//! pruebas. La lógica llega en las siguientes tareas.

use crate::dto::{ErroresDto, EstadoSesion, IniciarSesionDto, RegistroDto, SesionDto, UsuarioDto};

/// POST /usuarios. Errores por campo: nombre_usuario, nombre_visible, correo, contrasena.
#[tauri::command]
#[allow(unused_variables)]
pub async fn registrar_usuario(datos: RegistroDto) -> Result<UsuarioDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// POST /sesiones. Con éxito guarda el token en el dispositivo y devuelve el estado.
#[tauri::command]
#[allow(unused_variables)]
pub async fn iniciar_sesion(datos: IniciarSesionDto) -> Result<SesionDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// GET /usuarios/yo con el token guardado.
#[tauri::command]
pub async fn obtener_perfil() -> Result<UsuarioDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Borra el token y los datos de sesión del dispositivo.
#[tauri::command]
pub async fn cerrar_sesion() -> Result<(), ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Al abrir la app: ¿hay sesión vigente, vencida o ninguna? No necesita red.
#[tauri::command]
pub async fn estado_sesion() -> Result<SesionDto, ErroresDto> {
    Ok(SesionDto {
        estado: EstadoSesion::SinSesion,
        nombre_visible: None,
        vence_en: None,
    })
}
