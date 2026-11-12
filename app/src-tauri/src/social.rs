//! Comandos de amistades, grupos y gastos compartidos (Entrega 6).
//!
//! VERSIÓN CON DATOS FIJOS: no hablan con el servidor; usan el simulador en memoria
//! de `social_fijo.rs` (ver ese archivo para los datos de ejemplo y los nombres de
//! usuario de prueba que disparan cada caso). `previsualizar_division` ya es el
//! real: calcula en local con la división exacta del dominio.
//!
//! Rutas del servidor que usará cada comando cuando se conecten:
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
use crate::social_fijo::{self as fijo, con_estado};

/// Busca por nombre de usuario exacto. Nunca devuelve el correo.
#[tauri::command]
pub async fn buscar_usuario(datos: BuscarUsuarioDto) -> Result<UsuarioPublicoDto, ErroresDto> {
    fijo::buscar(&datos)
}

/// Envía una invitación de amistad por nombre de usuario.
#[tauri::command]
pub async fn invitar_amigo(datos: InvitarAmigoDto) -> Result<InvitacionAmistadDto, ErroresDto> {
    con_estado(|e| fijo::invitar_amigo(e, &datos))
}

/// Acepta o rechaza una invitación de amistad recibida.
#[tauri::command]
pub async fn responder_amistad(
    datos: ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    con_estado(|e| fijo::responder_amistad(e, &datos))
}

/// Amigos e invitaciones pendientes, enviadas y recibidas.
#[tauri::command]
pub async fn listar_amistades() -> Result<ListaAmistadesDto, ErroresDto> {
    Ok(con_estado(|e| fijo::listar_amistades(e)))
}

/// Elimina una amistad. No borra los grupos ni los gastos que ya comparten.
#[tauri::command(rename_all = "snake_case")]
pub async fn eliminar_amistad(amistad_id: String) -> Result<(), ErroresDto> {
    con_estado(|e| fijo::eliminar_amistad(e, &amistad_id))
}

/// Crea un grupo; quien lo crea queda como miembro y administrador.
#[tauri::command]
pub async fn crear_grupo(datos: NuevoGrupoDto) -> Result<GrupoResumenDto, ErroresDto> {
    con_estado(|e| fijo::crear_grupo(e, &datos))
}

/// Mis grupos y mis invitaciones pendientes.
#[tauri::command]
pub async fn listar_grupos() -> Result<ListaGruposDto, ErroresDto> {
    Ok(con_estado(|e| fijo::listar_grupos(e)))
}

/// El administrador invita a un amigo al grupo.
#[tauri::command]
pub async fn invitar_al_grupo(datos: InvitarAlGrupoDto) -> Result<InvitadoDto, ErroresDto> {
    con_estado(|e| fijo::invitar_al_grupo(e, &datos))
}

/// El invitado acepta o rechaza la invitación al grupo.
#[tauri::command]
pub async fn responder_invitacion_grupo(
    datos: ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    con_estado(|e| fijo::responder_invitacion_grupo(e, &datos))
}

/// Registra un gasto del grupo; el servidor calcula y guarda la división.
#[tauri::command]
pub async fn registrar_gasto_grupo(datos: NuevoGastoGrupoDto) -> Result<GastoGrupoDto, ErroresDto> {
    con_estado(|e| fijo::registrar_gasto(e, &datos))
}

/// Miembros, gastos y balances de un grupo (solo para miembros).
#[tauri::command(rename_all = "snake_case")]
pub async fn ver_grupo(grupo_id: String) -> Result<GrupoDetalleDto, ErroresDto> {
    con_estado(|e| fijo::ver_grupo(e, &grupo_id))
}

/// Vista previa de la división con decimales exactos. No usa la red.
#[tauri::command]
pub async fn previsualizar_division(
    datos: PrevisualizarDivisionDto,
) -> Result<PrevisualizacionDivisionDto, ErroresDto> {
    crate::previsualizacion::previsualizar(&datos)
}