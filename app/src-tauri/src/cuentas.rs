use std::sync::{Mutex, MutexGuard};

use aplicacion::CasosDeUso;
use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::{CuentaDto, ErroresDto, ListaCuentasDto, MonedaDto, NuevaCuentaDto};

/// Obtiene el acceso exclusivo a los casos de uso. Si un hilo anterior falló
/// con el candado tomado, se recupera el estado: los datos siguen siendo válidos.
fn bloquear(casos: &Mutex<CasosDeUso>) -> MutexGuard<'_, CasosDeUso> {
    casos.lock().unwrap_or_else(|e| e.into_inner())
}

fn monedas(casos: &Mutex<CasosDeUso>) -> Vec<MonedaDto> {
    bloquear(casos)
        .listar_monedas()
        .into_iter()
        .map(MonedaDto::from)
        .collect()
}

fn cuentas(casos: &Mutex<CasosDeUso>) -> Result<ListaCuentasDto, ErroresDto> {
    bloquear(casos)
        .listar_cuentas()
        .map(ListaCuentasDto::from)
        .map_err(ErroresDto::from)
}

fn crear(casos: &Mutex<CasosDeUso>, datos: NuevaCuentaDto) -> Result<CuentaDto, ErroresDto> {
    bloquear(casos)
        .crear_cuenta(datos.into())
        .map(CuentaDto::from)
        .map_err(ErroresDto::from)
}

#[tauri::command]
pub fn listar_monedas(estado: State<'_, EstadoCasosDeUso>) -> Vec<MonedaDto> {
    monedas(&estado)
}

#[tauri::command]
pub fn listar_cuentas(estado: State<'_, EstadoCasosDeUso>) -> Result<ListaCuentasDto, ErroresDto> {
    cuentas(&estado)
}

#[tauri::command]
pub fn crear_cuenta(
    estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaCuentaDto,
) -> Result<CuentaDto, ErroresDto> {
    crear(&estado, datos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::ErrorCampo;

    fn casos() -> Mutex<CasosDeUso> {
        Mutex::new(crate::crear_casos_de_uso())
    }

    fn datos(nombre: &str, moneda: &str, saldo: &str) -> NuevaCuentaDto {
        NuevaCuentaDto {
            nombre: nombre.into(),
            tipo: "efectivo".into(),
            moneda: moneda.into(),
            saldo_inicial: saldo.into(),
        }
    }

    fn errores_de(c: &Mutex<CasosDeUso>, d: NuevaCuentaDto) -> Vec<ErrorCampo> {
        match crear(c, d) {
            Err(e) => e.errores,
            Ok(_) => panic!("debía fallar"),
        }
    }

    #[test]
    fn listar_monedas_devuelve_cuatro() {
        assert_eq!(monedas(&casos()).len(), 4);
    }

    #[test]
    fn sin_cuentas_la_lista_esta_vacia() {
        match cuentas(&casos()) {
            Ok(lista) => assert!(lista.cuentas.is_empty()),
            Err(_) => panic!("listar no debía fallar"),
        }
    }

    #[test]
    fn cuenta_creada_aparece_al_listar() {
        let c = casos();
        match crear(&c, datos("Ahorros", "USD", "10.50")) {
            Ok(cuenta) => {
                assert_eq!(cuenta.nombre, "Ahorros");
                assert_eq!(cuenta.saldo_actual, "10.50");
                assert_eq!(cuenta.id.len(), 36);
            }
            Err(_) => panic!("debía ser válida"),
        }
        match cuentas(&c) {
            Ok(lista) => {
                assert_eq!(lista.cuentas.len(), 1);
                assert_eq!(lista.cuentas[0].nombre, "Ahorros");
                assert_eq!(lista.subtotales[0].moneda, "USD");
                assert_eq!(lista.subtotales[0].total, "10.50");
            }
            Err(_) => panic!("listar no debía fallar"),
        }
    }

    #[test]
    fn nombre_repetido_da_error_de_nombre() {
        let c = casos();
        assert!(crear(&c, datos("Cartera", "VES", "1.00")).is_ok());
        let errores = errores_de(&c, datos("cartera", "VES", "2.00"));
        assert_eq!(errores[0].campo.as_deref(), Some("nombre"));
    }

    #[test]
    fn nombre_vacio_da_error_de_nombre() {
        let errores = errores_de(&casos(), datos("   ", "VES", "10.00"));
        assert_eq!(errores[0].campo.as_deref(), Some("nombre"));
    }

    #[test]
    fn varios_errores_llegan_juntos() {
        let errores = errores_de(&casos(), datos("", "XYZ", "-5"));
        assert_eq!(errores.len(), 3);
    }
}
