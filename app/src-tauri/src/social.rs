//! Comandos de amistades, grupos y gastos compartidos (Entrega 6).
//!
//! Hablan con el servidor con el token de la sesión (ver `social_real.rs` y
//! `social_cliente.rs`). Sin sesión no llaman al servidor y piden iniciar sesión; con
//! el servidor apagado dan un mensaje claro y el resto de la aplicación sigue igual.
//! `previsualizar_division` es local: calcula con la división exacta del dominio.
//! Los datos fijos de `social_fijo.rs` quedan como respaldo, sin uso.
//!
//!   buscar_usuario                GET    /usuarios?usuario=<exacto>
//!   invitar_amigo                 POST   /amistades
//!   responder_amistad             POST   /amistades/{id}/respuesta
//!   listar_amistades              GET    /amistades
//!   eliminar_amistad              DELETE /amistades/{id}
//!   crear_grupo                   POST   /grupos
//!   listar_grupos                 GET    /grupos
//!   invitar_al_grupo              POST   /grupos/{id}/miembros
//!   responder_invitacion_grupo    POST   /grupos/{id}/miembros/yo/respuesta
//!   registrar_gasto_grupo         POST   /grupos/{id}/gastos
//!   ver_grupo                     GET    /grupos/{id}
//!   previsualizar_division        (local, sin red)

use crate::dto::ErroresDto;
use crate::dto_social::{
    BuscarUsuarioDto, GastoGrupoDto, GrupoDetalleDto, GrupoResumenDto, InvitacionAmistadDto,
    InvitadoDto, InvitarAlGrupoDto, InvitarAmigoDto, ListaAmistadesDto, ListaGruposDto,
    NuevoGastoGrupoDto, NuevoGrupoDto, PrevisualizacionDivisionDto, PrevisualizarDivisionDto,
    ResponderAmistadDto, ResponderInvitacionGrupoDto, UsuarioPublicoDto,
};
use crate::social_real as real;

/// Busca por nombre de usuario exacto. Nunca devuelve el correo.
#[tauri::command]
pub async fn buscar_usuario(
    app: tauri::AppHandle,
    datos: BuscarUsuarioDto,
) -> Result<UsuarioPublicoDto, ErroresDto> {
    real::buscar_usuario_real(&app, datos).await
}

/// Envía una invitación de amistad por nombre de usuario.
#[tauri::command]
pub async fn invitar_amigo(
    app: tauri::AppHandle,
    datos: InvitarAmigoDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    real::invitar_amigo_real(&app, datos).await
}

/// Acepta o rechaza una invitación de amistad recibida.
#[tauri::command]
pub async fn responder_amistad(
    app: tauri::AppHandle,
    datos: ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    real::responder_amistad_real(&app, datos).await
}

/// Amigos e invitaciones pendientes, enviadas y recibidas.
#[tauri::command]
pub async fn listar_amistades(app: tauri::AppHandle) -> Result<ListaAmistadesDto, ErroresDto> {
    real::listar_amistades_real(&app).await
}

/// Elimina una amistad. No borra los grupos ni los gastos que ya comparten.
#[tauri::command]
pub async fn eliminar_amistad(app: tauri::AppHandle, amistad_id: String) -> Result<(), ErroresDto> {
    real::eliminar_amistad_real(&app, amistad_id).await
}

/// Crea un grupo; quien lo crea queda como miembro y administrador.
#[tauri::command]
pub async fn crear_grupo(
    app: tauri::AppHandle,
    datos: NuevoGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    real::crear_grupo_real(&app, datos).await
}

/// Mis grupos y mis invitaciones pendientes.
#[tauri::command]
pub async fn listar_grupos(app: tauri::AppHandle) -> Result<ListaGruposDto, ErroresDto> {
    real::listar_grupos_real(&app).await
}

/// El administrador invita a un amigo al grupo.
#[tauri::command]
pub async fn invitar_al_grupo(
    app: tauri::AppHandle,
    datos: InvitarAlGrupoDto,
) -> Result<InvitadoDto, ErroresDto> {
    real::invitar_al_grupo_real(&app, datos).await
}

/// El invitado acepta o rechaza la invitación al grupo.
#[tauri::command]
pub async fn responder_invitacion_grupo(
    app: tauri::AppHandle,
    datos: ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    real::responder_invitacion_grupo_real(&app, datos).await
}

/// Registra un gasto del grupo; el servidor calcula y guarda la división.
#[tauri::command]
pub async fn registrar_gasto_grupo(
    app: tauri::AppHandle,
    datos: NuevoGastoGrupoDto,
) -> Result<GastoGrupoDto, ErroresDto> {
    real::registrar_gasto_grupo_real(&app, datos).await
}

/// Miembros, gastos y balances de un grupo (solo para miembros).
#[tauri::command]
pub async fn ver_grupo(
    app: tauri::AppHandle,
    grupo_id: String,
) -> Result<GrupoDetalleDto, ErroresDto> {
    real::ver_grupo_real(&app, grupo_id).await
}

/// Vista previa de la división con decimales exactos. No usa la red.
#[tauri::command]
pub async fn previsualizar_division(
    datos: PrevisualizarDivisionDto,
) -> Result<PrevisualizacionDivisionDto, ErroresDto> {
    crate::previsualizacion::previsualizar(&datos)
}
