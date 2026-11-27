//! Lee `docs/vectores-division.json` y comprueba que la división exacta de la app
//! (`dominio::division::dividir`) da los mismos números que fija el archivo.
//!
//! El servidor lee el mismo archivo con sus propias pruebas: así los dos cálculos
//! quedan fijados con los mismos números (RNF-08, INV-07, ESC-10). Si alguien cambia
//! la regla de reparto del residuo en un lado, esta prueba o la del servidor falla.

use std::path::PathBuf;

use dominio::division::{ParteDividida, Participante, dividir};
use serde_json::Value;

fn cargar() -> Value {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/vectores-division.json");
    let texto = std::fs::read_to_string(&ruta)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", ruta.display()));
    serde_json::from_str(&texto).expect("vectores-division.json no es un JSON válido")
}

fn texto<'a>(valor: &'a Value, campo: &str) -> &'a str {
    valor[campo]
        .as_str()
        .unwrap_or_else(|| panic!("falta el campo de texto `{campo}`"))
}

fn lista<'a>(valor: &'a Value, campo: &str) -> &'a Vec<Value> {
    valor[campo]
        .as_array()
        .unwrap_or_else(|| panic!("falta la lista `{campo}`"))
}

fn participantes(caso: &Value) -> Vec<Participante> {
    lista(caso, "participantes")
        .iter()
        .map(|p| Participante {
            usuario: texto(p, "usuario").to_string(),
            valor: texto(p, "valor").to_string(),
        })
        .collect()
}

fn comparar_partes(id: &str, reales: &[ParteDividida], esperadas: &[Value]) {
    assert_eq!(reales.len(), esperadas.len(), "{id}: cantidad de partes");
    for (real, esperada) in reales.iter().zip(esperadas) {
        assert_eq!(real.usuario, texto(esperada, "usuario"), "{id}: usuario");
        assert_eq!(
            real.monto,
            texto(esperada, "monto"),
            "{id}: monto de {}",
            real.usuario
        );
        let redondeo = esperada["incluye_redondeo"]
            .as_bool()
            .unwrap_or_else(|| panic!("{id}: falta incluye_redondeo"));
        assert_eq!(
            real.incluye_redondeo, redondeo,
            "{id}: marca de redondeo de {}",
            real.usuario
        );
    }
}

#[test]
fn la_division_de_la_app_coincide_con_los_vectores() {
    let documento = cargar();
    let casos = lista(&documento, "divisiones");
    assert!(!casos.is_empty(), "el archivo no trae casos de división");

    for caso in casos {
        let id = texto(caso, "id");
        let resultado = dividir(
            texto(caso, "monto"),
            texto(caso, "regla"),
            &participantes(caso),
        );

        if let Some(esperado) = caso.get("esperado") {
            let division =
                resultado.unwrap_or_else(|e| panic!("{id}: debía dividir y devolvió error: {e:?}"));
            assert_eq!(
                division.residuo,
                texto(esperado, "residuo"),
                "{id}: residuo"
            );
            comparar_partes(id, &division.partes, lista(esperado, "partes"));
        } else {
            let Err(errores) = resultado else {
                panic!("{id}: debía fallar y dividió");
            };
            let error = &caso["error"];
            let esperados = lista(error, "errores");
            let mensaje_exacto = error["mensaje_exacto"]
                .as_bool()
                .unwrap_or_else(|| panic!("{id}: falta mensaje_exacto"));
            let no_cuadra = error["no_cuadra"]
                .as_bool()
                .unwrap_or_else(|| panic!("{id}: falta no_cuadra"));

            assert_eq!(errores.len(), esperados.len(), "{id}: cantidad de errores");
            for (real, esperado) in errores.iter().zip(esperados) {
                assert_eq!(real.campo, texto(esperado, "campo"), "{id}: campo");
                if mensaje_exacto {
                    assert_eq!(real.mensaje, texto(esperado, "mensaje"), "{id}: mensaje");
                }
                assert_eq!(real.no_cuadra, no_cuadra, "{id}: no_cuadra");
            }
        }
    }
}

/// Los vectores de balances traen las partes de cada gasto. Esta prueba comprueba
/// que esas partes son las que calcula la app, para que los balances del archivo
/// partan de los mismos números que ve el usuario en la vista previa.
#[test]
fn las_partes_de_los_gastos_de_los_balances_son_las_de_la_app() {
    let documento = cargar();
    let balances = lista(&documento, "balances");
    assert!(!balances.is_empty(), "el archivo no trae casos de balances");

    for balance in balances {
        let id = texto(balance, "id");
        for gasto in lista(balance, "gastos") {
            let division = dividir(
                texto(gasto, "monto"),
                texto(gasto, "regla"),
                &participantes(gasto),
            )
            .unwrap_or_else(|e| panic!("{id}: un gasto debía dividirse y dio {e:?}"));
            comparar_partes(id, &division.partes, lista(gasto, "partes"));
        }
    }
}
