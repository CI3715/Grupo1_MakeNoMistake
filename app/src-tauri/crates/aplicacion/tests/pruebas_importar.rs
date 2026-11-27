//! Pruebas de leer y validar una copia de seguridad (Entrega 8, RF-INT-02).

use aplicacion::{Cifrador, ErrorRespaldo, EventoRegistrado, exportar_libro, leer_libro};

struct CifradorFalso;

impl Cifrador for CifradorFalso {
    fn cifrar(&self, _frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(claro.iter().map(|b| b ^ 0x5A).collect())
    }
    fn descifrar(&self, _frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(archivo.iter().map(|b| b ^ 0x5A).collect())
    }
}

const FRASE: &str = "una frase de paso larga";
const FECHA: &str = "2026-11-24T15:00:00Z";

fn evento(id: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-11-24T12:00:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: datos.to_string(),
    }
}

fn eventos() -> Vec<EventoRegistrado> {
    vec![
        evento("e1", r#"{"nombre":"Efectivo"}"#),
        evento("e2", r#"{"nota":"línea 1\nlínea 2 con \"comillas\""}"#),
        evento("e3", "texto con salto\nreal de línea"),
    ]
}

fn exportado_en_texto() -> String {
    let bytes = exportar_libro(&eventos(), FECHA, None, &CifradorFalso).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn ida_y_vuelta_sin_cifrar_conserva_los_eventos() {
    let bytes = exportado_en_texto().into_bytes();
    assert_eq!(leer_libro(&bytes, None, &CifradorFalso), Ok(eventos()));
}

#[test]
fn ida_y_vuelta_cifrada_conserva_los_eventos() {
    let bytes = exportar_libro(&eventos(), FECHA, Some(FRASE), &CifradorFalso).unwrap();
    assert_eq!(
        leer_libro(&bytes, Some(FRASE), &CifradorFalso),
        Ok(eventos())
    );
}

#[test]
fn archivo_cifrado_sin_frase_la_pide() {
    let bytes = exportar_libro(&eventos(), FECHA, Some(FRASE), &CifradorFalso).unwrap();
    assert_eq!(
        leer_libro(&bytes, None, &CifradorFalso),
        Err(ErrorRespaldo::FraseRequerida)
    );
}

#[test]
fn version_no_soportada_dice_cual_tiene_y_cual_soporta() {
    let texto = exportado_en_texto().replace("version=1", "version=99");
    assert_eq!(
        leer_libro(texto.as_bytes(), None, &CifradorFalso),
        Err(ErrorRespaldo::VersionNoSoportada {
            archivo: 99,
            soportada: 1
        })
    );
}

#[test]
fn cantidad_declarada_distinta_rechaza_todo_el_archivo() {
    let texto = exportado_en_texto().replace("eventos=3", "eventos=4");
    assert_eq!(
        leer_libro(texto.as_bytes(), None, &CifradorFalso),
        Err(ErrorRespaldo::ArchivoInvalido)
    );
}

#[test]
fn una_linea_mal_formada_rechaza_todo_el_archivo() {
    let texto = exportado_en_texto().replace("\"tipo\"", "\"tipx\"");
    assert_eq!(
        leer_libro(texto.as_bytes(), None, &CifradorFalso),
        Err(ErrorRespaldo::ArchivoInvalido)
    );
}
