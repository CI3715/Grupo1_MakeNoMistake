use crate::dto::{
    CuentaDto, ErrorCampo, ErroresDto, ListaCuentasDto, MonedaDto, NuevaCuentaDto, SubtotalDto,
};

fn moneda(codigo: &str, nombre: &str, simbolo: &str) -> MonedaDto {
    MonedaDto {
        codigo: codigo.into(),
        nombre: nombre.into(),
        simbolo: simbolo.into(),
        decimales: 2,
    }
}

#[tauri::command]
pub fn listar_monedas() -> Vec<MonedaDto> {
    vec![
        moneda("VES", "Bolívar", "Bs."),
        moneda("USD", "Dólar estadounidense", "$"),
        moneda("EUR", "Euro", "€"),
        moneda("USDT", "Tether (dólar digital)", "USDT"),
    ]
}

fn cuenta(id: &str, nombre: &str, tipo: &str, moneda: &str, saldo: &str) -> CuentaDto {
    CuentaDto {
        id: id.into(),
        nombre: nombre.into(),
        tipo: tipo.into(),
        moneda: moneda.into(),
        saldo_actual: saldo.into(),
    }
}

#[tauri::command]
pub fn listar_cuentas() -> Result<ListaCuentasDto, ErroresDto> {
    let cuentas = vec![
        cuenta(
            "a1f0c9d2-0b6e-4c55-9a4e-3d1f7c2b8e01",
            "Cartera",
            "efectivo",
            "VES",
            "1250.50",
        ),
        // TODO: agrega aquí las otras dos cuentas
    ];
    let subtotales = vec![
        SubtotalDto {
            moneda: "VES".into(),
            total: "1250.50".into(),
        },
        // TODO: agrega aquí los otros dos subtotales
    ];
    Ok(ListaCuentasDto {
        cuentas,
        subtotales,
    })
}

const NOMBRES_FIJOS: [&str; 3] = ["cartera", "cuenta nómina", "billetera usdt"];
const MONEDAS: [&str; 4] = ["VES", "USD", "EUR", "USDT"];

fn error(campo: &str, mensaje: &str) -> ErrorCampo {
    ErrorCampo {
        campo: Some(campo.into()),
        mensaje: mensaje.into(),
    }
}

// Un saldo válido es un número con a lo sumo un punto decimal (y quizá un signo "-").
fn es_numero(texto: &str) -> bool {
    let sin_signo = texto.strip_prefix('-').unwrap_or(texto);
    let mut puntos = 0;
    !sin_signo.is_empty()
        && sin_signo.chars().all(|c| {
            if c == '.' {
                puntos += 1;
                puntos == 1
            } else {
                c.is_ascii_digit()
            }
        })
}

#[tauri::command]
pub fn crear_cuenta(datos: NuevaCuentaDto) -> Result<CuentaDto, ErroresDto> {
    let mut errores: Vec<ErrorCampo> = Vec::new();
    let nombre = datos.nombre.trim();
    let saldo = datos.saldo_inicial.trim();
    let decimales = saldo.split_once('.').map_or(0, |(_, d)| d.len());

    if nombre.is_empty() {
        errores.push(error(
            "nombre",
            "El nombre no puede estar vacío. Escribe un nombre para la cuenta.",
        ));
    } else if NOMBRES_FIJOS.contains(&nombre.to_lowercase().as_str()) {
        errores.push(error(
            "nombre",
            "Ya existe una cuenta con ese nombre. Elige otro.",
        ));
    }

    if !MONEDAS.contains(&datos.moneda.as_str()) {
        errores.push(error(
            "moneda",
            "La moneda elegida no está en el catálogo. Elige una de la lista.",
        ));
    }

    if !es_numero(saldo) {
        errores.push(error(
            "saldo_inicial",
            "El saldo inicial debe ser un número, por ejemplo 1250.50.",
        ));
    } else if saldo.starts_with('-') {
        errores.push(error(
            "saldo_inicial",
            "El saldo inicial no puede ser negativo. Escribe 0 o un monto mayor.",
        ));
    } else if decimales > 2 {
        errores.push(error(
            "saldo_inicial",
            &format!(
                "El saldo inicial en {} admite como máximo 2 decimales.",
                datos.moneda
            ),
        ));
    }

    if !errores.is_empty() {
        return Err(ErroresDto { errores });
    }

    Ok(CuentaDto {
        id: uuid::Uuid::new_v4().to_string(),
        nombre: nombre.to_string(),
        tipo: datos.tipo,
        moneda: datos.moneda,
        saldo_actual: saldo.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn datos(nombre: &str, moneda: &str, saldo: &str) -> NuevaCuentaDto {
        NuevaCuentaDto {
            nombre: nombre.into(),
            tipo: "efectivo".into(),
            moneda: moneda.into(),
            saldo_inicial: saldo.into(),
        }
    }

    fn errores_de(d: NuevaCuentaDto) -> Vec<ErrorCampo> {
        match crear_cuenta(d) {
            Err(e) => e.errores,
            Ok(_) => panic!("debía fallar"),
        }
    }

    #[test]
    fn listar_monedas_devuelve_cuatro() {
        assert_eq!(listar_monedas().len(), 4);
    }

    #[test]
    fn nombre_vacio_da_error_de_nombre() {
        let errores = errores_de(datos("   ", "VES", "10.00"));
        assert_eq!(errores[0].campo.as_deref(), Some("nombre"));
    }

    #[test]
    fn varios_errores_llegan_juntos() {
        let errores = errores_de(datos("", "XYZ", "-5"));
        assert_eq!(errores.len(), 3);
    }

    #[test]
    fn cuenta_valida_se_devuelve_con_id_nuevo() {
        match crear_cuenta(datos("Ahorros", "USD", "10.50")) {
            Ok(cuenta) => {
                assert_eq!(cuenta.nombre, "Ahorros");
                assert_eq!(cuenta.saldo_actual, "10.50");
                assert_eq!(cuenta.id.len(), 36);
            }
            Err(_) => panic!("debía ser válida"),
        }
    }
}
