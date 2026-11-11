//! Comandos de amistades, grupos y gastos compartidos (Entrega 6).
//!
//! SOLO FIRMAS: todavía no hablan con el servidor ni devuelven datos fijos; cada
//! comando responde «Esta función todavía no está disponible.». Existen para que
//! la interfaz y los tipos de TypeScript puedan apoyarse en una forma acordada.
//! Los comandos con datos fijos y los reales reemplazan estos cuerpos.
//!
//! Rutas del servidor que usará cada comando:
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

// Los cuerpos todavía no leen sus argumentos.
#![allow(unused_variables)]

use crate::dto::ErroresDto;
use crate::dto_social::{
    BuscarUsuarioDto, GastoGrupoDto, GrupoDetalleDto, GrupoResumenDto, InvitacionAmistadDto,
    InvitadoDto, InvitarAlGrupoDto, InvitarAmigoDto, ListaAmistadesDto, ListaGruposDto,
    NuevoGastoGrupoDto, NuevoGrupoDto, PrevisualizacionDivisionDto, PrevisualizarDivisionDto,
    ResponderAmistadDto, ResponderInvitacionGrupoDto, UsuarioPublicoDto,
};

/// Busca por nombre de usuario exacto. Nunca devuelve el correo.
#[tauri::command]
pub async fn buscar_usuario(datos: BuscarUsuarioDto) -> Result<UsuarioPublicoDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Envía una invitación de amistad por nombre de usuario.
#[tauri::command]
pub async fn invitar_amigo(datos: InvitarAmigoDto) -> Result<InvitacionAmistadDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Acepta o rechaza una invitación de amistad recibida.
#[tauri::command]
pub async fn responder_amistad(
    datos: ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Amigos e invitaciones pendientes, enviadas y recibidas.
#[tauri::command]
pub async fn listar_amistades() -> Result<ListaAmistadesDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Elimina una amistad. No borra los grupos ni los gastos que ya comparten.
#[tauri::command]
pub async fn eliminar_amistad(amistad_id: String) -> Result<(), ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Crea un grupo; quien lo crea queda como miembro y administrador.
#[tauri::command]
pub async fn crear_grupo(datos: NuevoGrupoDto) -> Result<GrupoResumenDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Mis grupos y mis invitaciones pendientes.
#[tauri::command]
pub async fn listar_grupos() -> Result<ListaGruposDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// El administrador invita a un amigo al grupo.
#[tauri::command]
pub async fn invitar_al_grupo(datos: InvitarAlGrupoDto) -> Result<InvitadoDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// El invitado acepta o rechaza la invitación al grupo.
#[tauri::command]
pub async fn responder_invitacion_grupo(
    datos: ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Registra un gasto del grupo; el servidor calcula y guarda la división.
#[tauri::command]
pub async fn registrar_gasto_grupo(datos: NuevoGastoGrupoDto) -> Result<GastoGrupoDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Miembros, gastos y balances de un grupo (solo para miembros).
#[tauri::command]
pub async fn ver_grupo(grupo_id: String) -> Result<GrupoDetalleDto, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Vista previa de la división con decimales exactos. No usa la red.
#[tauri::command]
pub async fn previsualizar_division(
    datos: PrevisualizarDivisionDto,
) -> Result<PrevisualizacionDivisionDto, ErroresDto> {
    crate::previsualizacion::previsualizar(&datos)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn mensaje<T>(r: Result<T, ErroresDto>) -> String {
        match r {
            Ok(_) => panic!("se esperaba un error"),
            Err(e) => e.errores[0].mensaje.clone(),
        }
    }

    #[tokio::test]
    async fn los_comandos_sin_conectar_responden_en_espanol_y_sin_campo() {
        let esperado = "Esta función todavía no está disponible.";
        assert_eq!(mensaje(listar_amistades().await), esperado);
        assert_eq!(mensaje(listar_grupos().await), esperado);
        assert_eq!(mensaje(ver_grupo("g1".into()).await), esperado);
        assert_eq!(mensaje(eliminar_amistad("a1".into()).await), esperado);
    }
}
