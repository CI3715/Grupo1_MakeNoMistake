//! Prueba de concepto del almacén de secretos: `guardar`, `leer` o `borrar`.
//! Cada acción es un proceso aparte, para comprobar que el valor persiste.

use keyring::v1::Entry;

fn main() {
    let accion = std::env::args().nth(1).unwrap_or_default();
    let entrada = match Entry::new("cuentas-claras-poc", "sesion") {
        Ok(e) => e,
        Err(e) => {
            println!("no se pudo crear la entrada: {e}");
            std::process::exit(1);
        }
    };
    let resultado = match accion.as_str() {
        "guardar" => entrada
            .set_password("token-de-prueba")
            .map(|()| "guardado".to_string()),
        "leer" => entrada.get_password().map(|v| format!("leído: {v}")),
        "borrar" => entrada.delete_credential().map(|()| "borrado".to_string()),
        _ => {
            println!("uso: almacen_poc guardar | leer | borrar");
            return;
        }
    };
    match resultado {
        Ok(mensaje) => println!("{mensaje}"),
        Err(e) => println!("error: {e}"),
    }
}
