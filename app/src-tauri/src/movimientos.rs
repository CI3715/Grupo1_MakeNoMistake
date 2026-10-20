//! Comandos de categorías, movimientos y detalle de cuenta (Entrega 3).
//!
//! De momento solo fijan el contrato (nombres, datos de entrada y de salida) para
//! que la interfaz y la capa de aplicación puedan programar contra él. Todavía no
//! están conectados a los casos de uso: responden con un error en español.

use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::{
    CategoriaDto, CorreccionDto, DetalleCuentaDto, ErroresDto, MovimientoDto, NuevaCategoriaDto,
    NuevoMovimientoDto, PrevisualizacionDto,
};

fn no_disponible<T>() -> Result<T, ErroresDto> {
    Err(ErroresDto::no_implementado())
}

/// Registra un gasto o un ingreso en una cuenta y devuelve el movimiento guardado.
#[tauri::command]
pub fn registrar_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<MovimientoDto, ErroresDto> {
    let _ = &datos;
    no_disponible()
}

/// Corrige un movimiento registrando un ajuste por la diferencia; devuelve el ajuste.
#[tauri::command]
pub fn corregir_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: CorreccionDto,
) -> Result<MovimientoDto, ErroresDto> {
    let _ = &datos;
    no_disponible()
}

/// Calcula el saldo que quedaría tras un movimiento, sin guardar nada.
#[tauri::command]
pub fn previsualizar_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<PrevisualizacionDto, ErroresDto> {
    let _ = &datos;
    no_disponible()
}

/// Lista las categorías de fábrica y las creadas por la persona.
#[tauri::command]
pub fn listar_categorias(
    _estado: State<'_, EstadoCasosDeUso>,
) -> Result<Vec<CategoriaDto>, ErroresDto> {
    no_disponible()
}

/// Crea una categoría propia.
#[tauri::command]
pub fn crear_categoria(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaCategoriaDto,
) -> Result<CategoriaDto, ErroresDto> {
    let _ = &datos;
    no_disponible()
}

/// Detalle de una cuenta: saldo, composición y movimientos. Se invoca con
/// `invoke('detalle_cuenta', { cuenta_id })`, en snake_case como el resto.
#[tauri::command(rename_all = "snake_case")]
pub fn detalle_cuenta(
    _estado: State<'_, EstadoCasosDeUso>,
    cuenta_id: String,
) -> Result<DetalleCuentaDto, ErroresDto> {
    let _ = &cuenta_id;
    no_disponible()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mientras_no_esten_conectados_responden_con_un_error_claro() {
        let r: Result<Vec<CategoriaDto>, ErroresDto> = no_disponible();
        let e = r.err().expect("debe ser un error");
        assert_eq!(e.errores.len(), 1);
        assert!(e.errores[0].campo.is_none());
    }
}
