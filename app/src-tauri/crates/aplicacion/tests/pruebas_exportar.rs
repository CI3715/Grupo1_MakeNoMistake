//! Pruebas de exportar el libro a un archivo, con y sin cifrar (Entrega 8).

use aplicacion::{
    CasosDeUso, Cifrador, DispositivoFijo, ErrorRespaldo, EventoRegistrado, GeneradorSecuencial,
    NuevaCuenta, RelojFijo, RepositorioEnMemoria, exportar_libro,
};

/// Cifrador de mentira para las pruebas: solo oculta los bytes, no protege nada.
struct CifradorFalso;

impl Cifrador for CifradorFalso {
    fn cifrar(&self, _frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(claro.iter().map(|b| b ^ 0x5A).collect())
    }
    fn descifrar(&self, _frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(archivo.iter().map(|b| b ^ 0x5A).collect())
    }
}

fn evento(id: &str, nombre: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-11-24T12:00:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: format!(r#"{{"nombre":"{nombre}"}}"#),
    }
}

fn dos_eventos() -> Vec<EventoRegistrado> {
    vec![evento("e1", "Efectivo"), evento("e2", "Banco")]
}

const FRASE: &str = "una frase de paso larga";
const FECHA: &str = "2026-11-24T15:00:00Z";

#[test]
fn sin_cifrar_es_texto_legible_con_encabezado() {
    let bytes = exportar_libro(&dos_eventos(), FECHA, None, &CifradorFalso).unwrap();
    let texto = String::from_utf8(bytes).unwrap();
    assert!(texto.contains("version=1"));
    assert!(texto.contains("eventos=2"));
    assert!(texto.contains(&format!("exportado={FECHA}")));
    assert!(texto.contains("Efectivo") && texto.contains("Banco"));
    assert_eq!(texto.lines().count(), 3, "encabezado + un evento por línea");
}

#[test]
fn cifrado_no_deja_datos_legibles() {
    let bytes = exportar_libro(&dos_eventos(), FECHA, Some(FRASE), &CifradorFalso).unwrap();
    let texto = String::from_utf8_lossy(&bytes);
    for pista in ["Efectivo", "Banco", "eventos=", "cuentas-claras"] {
        assert!(!texto.contains(pista), "no debe verse «{pista}»");
    }
}

#[test]
fn frase_de_menos_de_12_caracteres_se_rechaza() {
    let corta = exportar_libro(&dos_eventos(), FECHA, Some("once-chars!"), &CifradorFalso);
    assert_eq!(corta, Err(ErrorRespaldo::FraseCorta));
    let justa = exportar_libro(&dos_eventos(), FECHA, Some("doce-chars!!"), &CifradorFalso);
    assert!(justa.is_ok());
}

#[test]
fn exporta_los_eventos_reales_del_libro() {
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-24T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );
    app.crear_cuenta(NuevaCuenta {
        nombre: "Efectivo".to_string(),
        tipo: "efectivo".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "10.00".to_string(),
    })
    .unwrap();
    let eventos = app.eventos_del_libro().unwrap();
    let bytes = exportar_libro(&eventos, FECHA, None, &CifradorFalso).unwrap();
    let texto = String::from_utf8(bytes).unwrap();
    assert!(texto.contains("eventos=1"));
    assert!(texto.contains(&eventos[0].id));
}
