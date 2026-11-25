//! Pruebas automatizadas de derivación de clave con Argon2id y cifrado autenticado
//! AES-256-GCM (Entrega 8: Angel - Tarea 4; RF-INT-05, RF-INT-06, ADR-017).
//!
//! Verifica:
//! 1. Ida y vuelta (round-trip) con `cifrar`/`descifrar` y con `exportar_libro`/`leer_libro`.
//! 2. Rechazo de frase de menos de 12 caracteres al cifrar.
//! 3. Frase de paso incorrecta al descifrar.
//! 4. Archivo alterado: un byte cambiado en la cabecera, en el cuerpo y en la etiqueta GCM.
//! 5. Archivo truncado en distintos puntos (vacío, cabecera incompleta, sin etiqueta, cuerpo cortado).
//! 6. No determinismo: dos cifrados del mismo contenido con la misma frase producen archivos,
//!    sales y nonces distintos.
//! 7. Privacidad del contenedor: ni la cabecera en claro ni el archivo completo filtran datos
//!    del libro ni la frase de paso.
//! 8. Versión de contenedor no soportada.

use aplicacion::{Cifrador, ErrorRespaldo, EventoRegistrado, exportar_libro, leer_libro};
use infraestructura::cifrado::{
    CifradorArgon2, LARGO_CABECERA, LARGO_ETIQUETA, LARGO_MINIMO_ARCHIVO, ParametrosArgon2,
    VERSION_CONTENEDOR, cifrar, cifrar_con_parametros, descifrar,
};

const FRASE_VALIDA: &str = "mi-frase-secreta-de-respaldo-2026";
const OTRA_FRASE: &str = "otra-frase-completamente-distinta";
const MENSAJE_ERROR_ESPERADO: &str = "Frase de paso incorrecta o archivo dañado.";

/// Parámetros ligeros de Argon2id para pruebas que verifican múltiples mutaciones de bytes
/// sin sacrificar la cobertura del algoritmo real Argon2id + AES-256-GCM.
fn params_rapidos() -> ParametrosArgon2 {
    ParametrosArgon2 {
        memoria_kib: 1024,
        iteraciones: 1,
        paralelismo: 1,
    }
}

fn eventos_de_ejemplo() -> Vec<EventoRegistrado> {
    vec![
        EventoRegistrado {
            id: "evt-secreto-001".to_string(),
            tipo: "CuentaCreada".to_string(),
            momento_registro: "2026-11-25T10:00:00Z".to_string(),
            dispositivo_origen: "laptop-personal-beto".to_string(),
            datos: r#"{"nombre":"Bóveda Principal USD","tipo":"banco","moneda":"USD","saldo_inicial":"15432.89"}"#.to_string(),
        },
        EventoRegistrado {
            id: "evt-secreto-002".to_string(),
            tipo: "MovimientoRegistrado".to_string(),
            momento_registro: "2026-11-25T11:30:00Z".to_string(),
            dispositivo_origen: "laptop-personal-beto".to_string(),
            datos: r#"{"cuenta_id":"evt-secreto-001","tipo":"egreso","monto":"250.00","fecha":"2026-11-25","categoria_id":"gasto:alimentacion","nota":"Compra confidencial supermercado"}"#.to_string(),
        },
    ]
}

fn verificar_fallo_autenticacion(resultado: Result<Vec<u8>, ErrorRespaldo>) {
    let err = resultado.expect_err("el descifrado debía fallar");
    assert_eq!(err, ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    assert_eq!(err.to_string(), MENSAJE_ERROR_ESPERADO);
}

#[test]
fn cifrado_ida_y_vuelta_con_parametros_de_produccion() {
    let original = b"contenido estructurado del libro personal con acentos: \xc3\xa1\xc3\xa9\xc3\xad\xc3\xb3\xc3\xba";
    let archivo = cifrar(FRASE_VALIDA, original).expect("debe cifrar con frase válida");
    let recuperado = descifrar(FRASE_VALIDA, &archivo).expect("debe descifrar con la misma frase");
    assert_eq!(recuperado, original);

    let adaptador = CifradorArgon2::con_parametros(params_rapidos());
    let cifrado_puerto = Cifrador::cifrar(&adaptador, FRASE_VALIDA, original).unwrap();
    let descifrado_puerto = Cifrador::descifrar(&adaptador, FRASE_VALIDA, &cifrado_puerto).unwrap();
    assert_eq!(descifrado_puerto, original);
}

#[test]
fn cifrado_ida_y_vuelta_con_exportar_y_leer_libro() {
    let cifrador = CifradorArgon2::con_parametros(params_rapidos());
    let eventos = eventos_de_ejemplo();
    let archivo = exportar_libro(&eventos, "2026-11-25T12:00:00Z", Some(FRASE_VALIDA), &cifrador)
        .expect("exportar con cifrador real debe funcionar");

    let leidos = leer_libro(&archivo, Some(FRASE_VALIDA), &cifrador)
        .expect("leer libro cifrado con frase correcta debe funcionar");
    assert_eq!(leidos, eventos);
}

#[test]
fn cifrado_acepta_carga_vacia_y_la_recupera() {
    let archivo =
        cifrar_con_parametros(FRASE_VALIDA, b"", params_rapidos()).expect("debe cifrar vacío");
    assert_eq!(archivo.len(), LARGO_MINIMO_ARCHIVO);
    let recuperado = descifrar(FRASE_VALIDA, &archivo).expect("debe descifrar vacío");
    assert!(recuperado.is_empty());
}

#[test]
fn cifrado_rechaza_frase_de_menos_de_12_caracteres() {
    let corta = cifrar_con_parametros("once-letras", b"datos", params_rapidos());
    assert_eq!(corta, Err(ErrorRespaldo::FraseCorta));

    let exacta = cifrar_con_parametros("doce-letras!", b"datos", params_rapidos());
    assert!(exacta.is_ok());
}

#[test]
fn cifrado_con_frase_incorrecta_falla_con_mensaje_claro() {
    let original = b"evento importante del libro personal";
    let archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    verificar_fallo_autenticacion(descifrar(OTRA_FRASE, &archivo));
    verificar_fallo_autenticacion(descifrar("corta", &archivo));
    verificar_fallo_autenticacion(descifrar("", &archivo));
}

#[test]
fn cifrado_archivo_alterado_en_la_cabecera_se_rechaza() {
    let original = b"datos protegidos por AAD en la cabecera";
    let archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    // Probamos alterar un byte en cada sección de la cabecera (excepto el byte 4 de versión,
    // que tiene su propia prueba cuando cambia a una versión superior):
    // - byte 0: magia ("CCBK")
    // - byte 5: identificador de algoritmo
    // - byte 6: costo de memoria Argon2id
    // - byte 10: iteraciones Argon2id
    // - byte 14: paralelismo Argon2id
    // - byte 20: sal aleatoria
    // - byte 38: nonce aleatorio
    for posicion in [0, 2, 5, 6, 10, 14, 18, 25, 34, 45] {
        assert!(posicion < LARGO_CABECERA);
        let mut alterado = archivo.clone();
        alterado[posicion] ^= 0x01;
        verificar_fallo_autenticacion(descifrar(FRASE_VALIDA, &alterado));
    }

    // Si el byte de versión (byte 4) se corrompe a 0, también es archivo dañado.
    let mut version_cero = archivo;
    version_cero[4] = 0;
    verificar_fallo_autenticacion(descifrar(FRASE_VALIDA, &version_cero));
}

#[test]
fn cifrado_archivo_alterado_en_el_cuerpo_se_rechaza() {
    let original = b"linea 1 del libro\nlinea 2 del libro\nlinea 3 del libro";
    let archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    let inicio_cuerpo = LARGO_CABECERA;
    let fin_cuerpo = archivo.len() - LARGO_ETIQUETA;
    let mitad_cuerpo = inicio_cuerpo + (fin_cuerpo - inicio_cuerpo) / 2;

    for posicion in [inicio_cuerpo, mitad_cuerpo, fin_cuerpo - 1] {
        let mut alterado = archivo.clone();
        alterado[posicion] ^= 0x80;
        verificar_fallo_autenticacion(descifrar(FRASE_VALIDA, &alterado));
    }
}

#[test]
fn cifrado_archivo_alterado_en_la_etiqueta_se_rechaza() {
    let original = b"datos autenticados con etiqueta GCM de 128 bits";
    let archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    let inicio_etiqueta = archivo.len() - LARGO_ETIQUETA;
    let mitad_etiqueta = inicio_etiqueta + LARGO_ETIQUETA / 2;
    let fin_etiqueta = archivo.len() - 1;

    for posicion in [inicio_etiqueta, mitad_etiqueta, fin_etiqueta] {
        let mut alterado = archivo.clone();
        alterado[posicion] ^= 0x40;
        verificar_fallo_autenticacion(descifrar(FRASE_VALIDA, &alterado));
    }
}

#[test]
fn cifrado_archivo_truncado_se_rechaza() {
    let original = b"contenido completo que no puede aplicarse si se corta el archivo";
    let archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    for corte in [
        0,
        3,
        LARGO_CABECERA - 1,
        LARGO_CABECERA,
        LARGO_MINIMO_ARCHIVO - 1,
        LARGO_CABECERA + 5,
        archivo.len() - 1,
    ] {
        let truncado = &archivo[..corte];
        verificar_fallo_autenticacion(descifrar(FRASE_VALIDA, truncado));
    }
}

#[test]
fn dos_cifrados_del_mismo_contenido_producen_archivos_distintos() {
    let original = b"mismo contenido para comprobar sal y nonce aleatorios";
    let primero = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();
    let segundo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();

    assert_ne!(primero, segundo, "dos cifrados nunca deben ser idénticos");
    // Sal (18..34) y nonce (34..46) deben cambiar en cada cifrado.
    assert_ne!(&primero[18..34], &segundo[18..34], "la sal debe ser nueva");
    assert_ne!(
        &primero[34..46],
        &segundo[34..46],
        "el nonce debe ser nuevo"
    );
    // El cuerpo cifrado también debe diferir.
    assert_ne!(&primero[LARGO_CABECERA..], &segundo[LARGO_CABECERA..]);
}

#[test]
fn la_parte_en_claro_y_el_archivo_no_contienen_texto_del_libro_ni_la_frase() {
    let cifrador = CifradorArgon2::con_parametros(params_rapidos());
    let eventos = eventos_de_ejemplo();
    let archivo =
        exportar_libro(&eventos, "2026-11-25T12:00:00Z", Some(FRASE_VALIDA), &cifrador).unwrap();

    let cabecera_texto = String::from_utf8_lossy(&archivo[..LARGO_CABECERA]);
    let archivo_texto = String::from_utf8_lossy(&archivo);

    for secreto in [
        FRASE_VALIDA,
        "Bóveda Principal USD",
        "15432.89",
        "250.00",
        "Compra confidencial supermercado",
        "gasto:alimentacion",
        "evt-secreto-001",
        "evt-secreto-002",
        "laptop-personal-beto",
        "CuentaCreada",
        "MovimientoRegistrado",
        "cuentas-claras-respaldo",
    ] {
        assert!(
            !cabecera_texto.contains(secreto),
            "la cabecera en claro no puede filtrar «{secreto}»"
        );
        assert!(
            !archivo_texto.contains(secreto),
            "el archivo cifrado no puede contener «{secreto}» en claro"
        );
    }
}

#[test]
fn version_no_soportada_del_contenedor_devuelve_error_tipado() {
    let original = b"datos de prueba";
    let mut archivo = cifrar_con_parametros(FRASE_VALIDA, original, params_rapidos()).unwrap();
    archivo[4] = 99;

    let err = descifrar(FRASE_VALIDA, &archivo).expect_err("versión 99 no debe aceptarse");
    assert_eq!(
        err,
        ErrorRespaldo::VersionNoSoportada {
            archivo: 99,
            soportada: u32::from(VERSION_CONTENEDOR),
        }
    );
}
