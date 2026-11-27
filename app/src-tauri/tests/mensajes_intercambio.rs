//! Los mensajes de error de la copia de seguridad y de la sincronización (Entrega 8,
//! sección 2.7, RNF-18).
//!
//! Hoy el núcleo manda el error como texto y la interfaz decide cuál es leyendo ese texto
//! (`app/utils/erroresCopia.ts`, función `tipoPorMensaje`). Estas pruebas atan los dos
//! extremos para que nadie rompa la pantalla de errores cambiando una frase en Rust:
//!
//! 1. Cada mensaje del núcleo es reconocido con el tipo que la interfaz espera.
//! 2. Las palabras con las que la interfaz reconoce cada tipo siguen escritas en el `.ts`.
//! 3. Ningún mensaje lleva trazas técnicas (rutas, `os error`, nombres de tipos de Rust).

use std::fs;
use std::path::PathBuf;

use aplicacion::{ErrorRespaldo, ErrorSecretos, ErrorTransporte};

const FRASE_INCORRECTA: &str = "Frase de paso incorrecta o archivo dañado.";

/// Réplica de `tipoPorMensaje` de `erroresCopia.ts`, en el mismo orden. Si cambia una,
/// hay que cambiar la otra (la prueba 2 lo vigila).
fn tipo_por_mensaje(mensaje: &str) -> &'static str {
    let texto = mensaje.to_lowercase();
    if texto.contains("frase de paso incorrecta") {
        return "frase_incorrecta";
    }
    if texto.contains("está cifrado") {
        return "frase_requerida";
    }
    if texto.contains("al menos 12 caracteres") {
        return "frase_corta";
    }
    if texto.contains("versión") && texto.contains("soporta") {
        return "version_no_soportada";
    }
    if texto.contains("no tiene el formato") {
        return "archivo_invalido";
    }
    if texto.contains("credenciales") && texto.contains("rechazad") {
        return "credenciales_rechazadas";
    }
    if texto.contains("sin conexión") {
        return "sin_conexion";
    }
    if texto.contains("no se pudo llegar") {
        return "destino_inaccesible";
    }
    "otro"
}

/// Palabras que `erroresCopia.ts` debe seguir usando para reconocer cada tipo.
const PALABRAS_EN_LA_INTERFAZ: [&str; 10] = [
    "frase de paso incorrecta",
    "está cifrado",
    "al menos 12 caracteres",
    "versión",
    "soporta",
    "no tiene el formato",
    "credenciales",
    "rechazad",
    "sin conexión",
    "no se pudo llegar",
];

/// Fuerza un error de compilación si alguien agrega una variante: así se acuerda de
/// decidir qué ve la persona y de añadirla a las pruebas.
fn _variantes_respaldo_cubiertas(e: &ErrorRespaldo) {
    match e {
        ErrorRespaldo::FraseCorta
        | ErrorRespaldo::FraseIncorrectaOArchivoDanado
        | ErrorRespaldo::LibroNoLegible
        | ErrorRespaldo::ArchivoInvalido
        | ErrorRespaldo::FraseRequerida
        | ErrorRespaldo::VersionNoSoportada { .. }
        | ErrorRespaldo::CifradoNoDisponible => {}
    }
}

fn _variantes_transporte_cubiertas(e: &ErrorTransporte) {
    match e {
        ErrorTransporte::SinConexion
        | ErrorTransporte::DestinoInaccesible
        | ErrorTransporte::CredencialesRechazadas
        | ErrorTransporte::DestinoCambio
        | ErrorTransporte::LoteYaExiste { .. }
        | ErrorTransporte::LoteNoExiste { .. }
        | ErrorTransporte::NombreInvalido(_)
        | ErrorTransporte::Interno(_) => {}
    }
}

fn mensajes_de_respaldo() -> Vec<(ErrorRespaldo, &'static str)> {
    vec![
        (ErrorRespaldo::FraseCorta, "frase_corta"),
        (
            ErrorRespaldo::FraseIncorrectaOArchivoDanado,
            "frase_incorrecta",
        ),
        (ErrorRespaldo::ArchivoInvalido, "archivo_invalido"),
        (ErrorRespaldo::FraseRequerida, "frase_requerida"),
        (
            ErrorRespaldo::VersionNoSoportada {
                archivo: 99,
                soportada: 1,
            },
            "version_no_soportada",
        ),
        // Sin tipo propio en la interfaz: se muestra su texto tal cual.
        (ErrorRespaldo::LibroNoLegible, "otro"),
        (ErrorRespaldo::CifradoNoDisponible, "otro"),
    ]
}

fn mensajes_de_transporte() -> Vec<(ErrorTransporte, &'static str)> {
    vec![
        (ErrorTransporte::SinConexion, "sin_conexion"),
        (ErrorTransporte::DestinoInaccesible, "destino_inaccesible"),
        (
            ErrorTransporte::CredencialesRechazadas,
            "credenciales_rechazadas",
        ),
        (ErrorTransporte::DestinoCambio, "otro"),
        (
            ErrorTransporte::LoteYaExiste {
                dispositivo: "laptop-beto".to_string(),
                lote: "20261124T120000Z-0001".to_string(),
            },
            "otro",
        ),
        (
            ErrorTransporte::LoteNoExiste {
                dispositivo: "laptop-beto".to_string(),
                lote: "20261124T120000Z-0001".to_string(),
            },
            "otro",
        ),
        (ErrorTransporte::NombreInvalido("a/b".to_string()), "otro"),
        // El detalle técnico va en el campo, pero no debe salir en el mensaje.
        (
            ErrorTransporte::Interno("os error 13 en /home/beto/.git/config".to_string()),
            "otro",
        ),
    ]
}

fn todos_los_mensajes() -> Vec<String> {
    let mut mensajes: Vec<String> = Vec::new();
    for (error, _) in mensajes_de_respaldo() {
        mensajes.push(error.to_string());
    }
    for (error, _) in mensajes_de_transporte() {
        mensajes.push(error.to_string());
    }
    let secretos = vec![
        ErrorSecretos::FraseCorta,
        ErrorSecretos::CredencialesIncompletas,
        ErrorSecretos::AlmacenNoDisponible("keyring: org.freedesktop.Secret".to_string()),
    ];
    for error in secretos {
        mensajes.push(error.to_string());
    }
    mensajes
}

#[test]
fn cada_error_del_nucleo_se_reconoce_con_el_tipo_que_espera_la_interfaz() {
    for (error, esperado) in mensajes_de_respaldo() {
        assert_eq!(
            tipo_por_mensaje(&error.to_string()),
            esperado,
            "mensaje: {error}"
        );
    }
    for (error, esperado) in mensajes_de_transporte() {
        assert_eq!(
            tipo_por_mensaje(&error.to_string()),
            esperado,
            "mensaje: {error}"
        );
    }
}

#[test]
fn la_frase_incorrecta_dice_exactamente_lo_que_pide_la_especificacion() {
    assert_eq!(
        ErrorRespaldo::FraseIncorrectaOArchivoDanado.to_string(),
        FRASE_INCORRECTA
    );
}

#[test]
fn version_no_soportada_dice_cual_tiene_el_archivo_y_cual_soporta_la_app() {
    let texto = ErrorRespaldo::VersionNoSoportada {
        archivo: 99,
        soportada: 1,
    }
    .to_string();
    assert!(
        texto.contains("99"),
        "falta la versión del archivo: {texto}"
    );
    assert!(
        texto.contains(" 1."),
        "falta la versión que soporta la aplicación: {texto}"
    );
}

#[test]
fn los_errores_reintentables_son_solo_los_que_basta_con_volver_a_intentar() {
    // La interfaz muestra «Reintentar» en sin conexión y destino inaccesible, y
    // «Actualizar credenciales» en credenciales rechazadas, nunca los dos juntos.
    assert!(ErrorTransporte::SinConexion.es_reintentable());
    assert!(ErrorTransporte::DestinoInaccesible.es_reintentable());
    assert!(!ErrorTransporte::CredencialesRechazadas.es_reintentable());
}

#[test]
fn ningun_mensaje_lleva_trazas_tecnicas() {
    const TRAZAS: [&str; 14] = [
        "::",
        "panicked",
        "unwrap",
        "backtrace",
        "os error",
        "errno",
        "/home",
        "\\users",
        "src/",
        ".rs",
        "keyring",
        "org.freedesktop",
        "{",
        "}",
    ];
    for mensaje in todos_los_mensajes() {
        let minusculas = mensaje.to_lowercase();
        for traza in TRAZAS {
            assert!(
                !minusculas.contains(traza),
                "el mensaje «{mensaje}» contiene «{traza}» (RNF-18)"
            );
        }
    }
}

#[test]
fn la_interfaz_sigue_reconociendo_los_errores_por_las_mismas_palabras() {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../app/utils/erroresCopia.ts");
    let fuente =
        fs::read_to_string(&ruta).expect("No se encontró erroresCopia.ts: actualiza esta prueba");
    for palabra in PALABRAS_EN_LA_INTERFAZ {
        assert!(
            fuente.contains(&format!("'{palabra}'")),
            "erroresCopia.ts ya no busca «{palabra}»: o se cambió allí, o hay que cambiar \
             esta prueba y los mensajes de Rust a la vez"
        );
    }
}
