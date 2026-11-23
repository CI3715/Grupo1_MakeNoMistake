//! Comandos de sincronización (Entrega 7).
//!
//! VERSIÓN CON DATOS FIJOS para construir la interfaz: responden siempre lo mismo.
//! Los comandos reales los reemplazan (Cristina - Tareas 9 y 10).
//!
//!   sincronizar                pide los cambios, en bucle      (real, después)
//!   estado_sincronizacion      lee el estado local             (real, después)
//!   cuentas_para_grupo         cuentas activas en la moneda del grupo
//!   elegir_cuenta_grupo        guarda la elección solo en el dispositivo
//!   listar_partes_pendientes   partes sin cuenta elegida
//!   reiniciar_cursor           pone el cursor en 0 (para la demostración)

use crate::dto::ErroresDto;
use crate::dto_sincronizacion::{
    ConsultaCuentasGrupoDto, CuentaParaGrupoDto, CuentasParaGrupoDto, ElegirCuentaGrupoDto,
    EstadoSincronizacionDto, GastoEnColaDto, ParteGrupoPendienteDto,
};

fn partes_fijas() -> Vec<ParteGrupoPendienteDto> {
    vec![ParteGrupoPendienteDto {
        evento_id: "e-100".to_string(),
        grupo_id: "g1".to_string(),
        grupo_nombre: "Viaje a Mérida".to_string(),
        concepto: "Hotel".to_string(),
        monto: "33.33".to_string(),
        moneda: "USD".to_string(),
        fecha: "2026-11-20".to_string(),
    }]
}

fn estado_fijo() -> EstadoSincronizacionDto {
    EstadoSincronizacionDto {
        ultima_sincronizacion: Some("2026-11-20T10:30:00".to_string()),
        sincronizando: false,
        partes_pendientes: partes_fijas(),
        cola: vec![
            GastoEnColaDto {
                clave: "k-1".to_string(),
                grupo_id: "g1".to_string(),
                grupo_nombre: "Viaje a Mérida".to_string(),
                concepto: "Cena".to_string(),
                monto: "60.00".to_string(),
                moneda: "USD".to_string(),
                fecha: "2026-11-20".to_string(),
                estado: "pendiente".to_string(),
                motivo: None,
            },
            GastoEnColaDto {
                clave: "k-2".to_string(),
                grupo_id: "g3".to_string(),
                grupo_nombre: "Cena del viernes".to_string(),
                concepto: "Postre".to_string(),
                monto: "12.00".to_string(),
                moneda: "USD".to_string(),
                fecha: "2026-11-19".to_string(),
                estado: "rechazado".to_string(),
                motivo: Some("Ya no eres miembro del grupo.".to_string()),
            },
        ],
    }
}

/// Sincroniza con el servidor y devuelve cómo quedó el estado.
#[tauri::command]
pub fn sincronizar() -> Result<EstadoSincronizacionDto, ErroresDto> {
    Ok(estado_fijo())
}

/// Estado actual: última sincronización, partes pendientes y gastos en cola.
#[tauri::command]
pub fn estado_sincronizacion() -> Result<EstadoSincronizacionDto, ErroresDto> {
    Ok(estado_fijo())
}

/// Cuentas personales que se pueden elegir para el grupo.
#[tauri::command]
pub fn cuentas_para_grupo(
    datos: ConsultaCuentasGrupoDto,
) -> Result<CuentasParaGrupoDto, ErroresDto> {
    Ok(CuentasParaGrupoDto {
        grupo_id: datos.grupo_id,
        cuenta_elegida: None,
        cuentas: vec![
            CuentaParaGrupoDto {
                id: "c1".to_string(),
                nombre: "Efectivo USD".to_string(),
                moneda: "USD".to_string(),
            },
            CuentaParaGrupoDto {
                id: "c2".to_string(),
                nombre: "Banco USD".to_string(),
                moneda: "USD".to_string(),
            },
        ],
    })
}

/// Guarda la cuenta elegida para el grupo; devuelve las cuentas con la elección hecha.
#[tauri::command]
pub fn elegir_cuenta_grupo(datos: ElegirCuentaGrupoDto) -> Result<CuentasParaGrupoDto, ErroresDto> {
    let mut resultado = cuentas_para_grupo(ConsultaCuentasGrupoDto {
        grupo_id: datos.grupo_id,
    })?;
    resultado.cuenta_elegida = Some(datos.cuenta_id);
    Ok(resultado)
}

/// Partes de grupo que esperan que se elija una cuenta.
#[tauri::command]
pub fn listar_partes_pendientes() -> Result<Vec<ParteGrupoPendienteDto>, ErroresDto> {
    Ok(partes_fijas())
}

/// Pone el cursor en cero (para la demostración): la siguiente sincronización baja todo de nuevo.
#[tauri::command]
pub fn reiniciar_cursor() -> Result<EstadoSincronizacionDto, ErroresDto> {
    Ok(estado_fijo())
}
