//! Los errores de importar y de sincronizar no alteran el libro (Entrega 8: Katherine -
//! Tarea 3; sección 2.7, RF-INT-02, RF-INT-04, ESC-04).
//!
//! Estas pruebas usan el cifrador real (Argon2id + cifrado autenticado) y la carpeta
//! compartida real. Comprueban, para cada situación de la tabla de la sección 2.7, que:
//!
//! - el error es el que la persona debe ver,
//! - el libro queda exactamente igual (mismos eventos, mismo orden, mismos saldos),
//! - lo pendiente no se pierde: al volver a intentar, todo sale bien.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use aplicacion::sincronizar_libro::{
    ErrorSincronizacion, EstadoSincronizacion, InformeSincronizacion, sincronizar,
};
use aplicacion::transporte::{ErrorTransporte, TransporteSincronizacion, nombre_de_lote};
use aplicacion::{
    CasosDeUso, Cifrador, DispositivoFijo, ErrorRespaldo, GeneradorIdentificadores, NuevaCuenta,
    RelojFijo, RepositorioEnMemoria,
};
use infraestructura::cifrado::{CifradorArgon2, ParametrosArgon2};
use infraestructura::transporte_carpeta::TransporteCarpeta;

const FRASE: &str = "frase-de-paso-de-beto-2026";
const OTRA_FRASE: &str = "otra-frase-de-paso-distinta";
const AHORA: &str = "2026-11-26T12:00:00Z";

// ---------- Apoyo ----------

static CONTADOR: AtomicU32 = AtomicU32::new(0);

/// Carpeta temporal única que se borra al terminar la prueba.
struct CarpetaTemporal {
    base: PathBuf,
}

impl CarpetaTemporal {
    fn nueva() -> CarpetaTemporal {
        let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("cc_errores_{}_{n}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("destino")).unwrap();
        CarpetaTemporal { base }
    }

    fn destino(&self) -> PathBuf {
        self.base.join("destino")
    }
}

impl Drop for CarpetaTemporal {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// Parámetros ligeros de Argon2id: aquí se prueba el comportamiento ante errores, no la
/// resistencia de la derivación.
fn cifrador() -> CifradorArgon2 {
    CifradorArgon2::con_parametros(ParametrosArgon2 {
        memoria_kib: 1024,
        iteraciones: 1,
        paralelismo: 1,
    })
}

/// Identificadores con prefijo propio: dos dispositivos reales nunca repiten ids.
struct Prefijo {
    prefijo: String,
    siguiente: u32,
}

impl GeneradorIdentificadores for Prefijo {
    fn nuevo_id(&mut self) -> String {
        self.siguiente += 1;
        format!("{}-{:04}", self.prefijo, self.siguiente)
    }
}

fn libro(dispositivo: &str, prefijo: &str, cuentas: &[&str]) -> CasosDeUso {
    let mut casos = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo(AHORA)),
        Box::new(Prefijo {
            prefijo: prefijo.to_string(),
            siguiente: 0,
        }),
        Box::new(DispositivoFijo::nuevo(dispositivo)),
    );
    for nombre in cuentas {
        casos
            .crear_cuenta(NuevaCuenta {
                nombre: nombre.to_string(),
                tipo: "efectivo".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "10.00".to_string(),
            })
            .unwrap();
    }
    casos
}

/// Todo lo observable del libro: los eventos (con su orden) y los saldos.
fn instantanea(casos: &CasosDeUso) -> (String, Vec<(String, String)>) {
    let eventos = format!("{:?}", casos.eventos_del_libro().unwrap());
    let mut saldos: Vec<(String, String)> = casos
        .listar_cuentas()
        .unwrap()
        .cuentas
        .iter()
        .map(|c| (c.id.clone(), c.saldo_actual.clone()))
        .collect();
    saldos.sort();
    (eventos, saldos)
}

fn ids(casos: &CasosDeUso) -> BTreeSet<String> {
    casos
        .eventos_del_libro()
        .unwrap()
        .iter()
        .map(|e| e.id.clone())
        .collect()
}

fn cantidad_de_eventos(casos: &CasosDeUso) -> usize {
    casos.eventos_del_libro().unwrap().len()
}

fn archivos_de(dir: &Path) -> Vec<PathBuf> {
    let mut encontrados = Vec::new();
    for entrada in fs::read_dir(dir).unwrap() {
        let ruta = entrada.unwrap().path();
        if ruta.is_dir() {
            encontrados.extend(archivos_de(&ruta));
        } else {
            encontrados.push(ruta);
        }
    }
    encontrados
}

fn con_byte_alterado(bytes: &[u8], desde_el_final: usize) -> Vec<u8> {
    let mut copia = bytes.to_vec();
    let posicion = copia.len() - desde_el_final;
    copia[posicion] ^= 0xFF;
    copia
}

fn con_version_99(texto_claro: &[u8]) -> Vec<u8> {
    let texto = String::from_utf8(texto_claro.to_vec()).unwrap();
    assert!(texto.contains("version=1"), "el encabezado cambió de forma");
    texto.replacen("version=1", "version=99", 1).into_bytes()
}

// ---------- Importar ----------

/// Intenta importar `bytes` en el libro del teléfono, que ya tiene datos propios.
/// Devuelve el error y comprueba que el libro quedó idéntico.
fn importar_y_comprobar_que_no_cambia(bytes: &[u8], frase: Option<&str>) -> ErrorRespaldo {
    let mut telefono = libro("telefono-beto", "tel", &["Ahorros"]);
    let antes = instantanea(&telefono);

    let error = match telefono.importar_copia(bytes, frase, &cifrador()) {
        Ok(_) => panic!("se esperaba un error al importar"),
        Err(e) => e,
    };

    assert_eq!(
        instantanea(&telefono),
        antes,
        "el libro cambió tras el error: {error}"
    );
    error
}

fn copia_cifrada_de_la_laptop() -> Vec<u8> {
    let laptop = libro("laptop-beto", "lap", &["Efectivo", "Bóveda"]);
    laptop.exportar_copia(Some(FRASE), &cifrador()).unwrap()
}

fn copia_legible_de_la_laptop() -> Vec<u8> {
    let laptop = libro("laptop-beto", "lap", &["Efectivo", "Bóveda"]);
    laptop.exportar_copia(None, &cifrador()).unwrap()
}

#[test]
fn importar_con_frase_incorrecta_no_cambia_el_libro() {
    let error = importar_y_comprobar_que_no_cambia(&copia_cifrada_de_la_laptop(), Some(OTRA_FRASE));
    assert_eq!(error, ErrorRespaldo::FraseIncorrectaOArchivoDanado);
}

#[test]
fn importar_un_archivo_alterado_no_cambia_el_libro() {
    let copia = copia_cifrada_de_la_laptop();
    // Un byte de la etiqueta de autenticación y otro dentro del contenido cifrado.
    for desde_el_final in [1, 20] {
        let alterada = con_byte_alterado(&copia, desde_el_final);
        let error = importar_y_comprobar_que_no_cambia(&alterada, Some(FRASE));
        assert_eq!(error, ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }
}

#[test]
fn importar_un_archivo_truncado_no_cambia_el_libro() {
    let copia = copia_cifrada_de_la_laptop();
    let truncada = &copia[..copia.len() / 2];
    let error = importar_y_comprobar_que_no_cambia(truncada, Some(FRASE));
    assert!(
        matches!(
            error,
            ErrorRespaldo::FraseIncorrectaOArchivoDanado | ErrorRespaldo::ArchivoInvalido
        ),
        "error inesperado: {error}"
    );
}

#[test]
fn importar_un_archivo_cifrado_sin_frase_no_cambia_el_libro() {
    let error = importar_y_comprobar_que_no_cambia(&copia_cifrada_de_la_laptop(), None);
    assert_eq!(error, ErrorRespaldo::FraseRequerida);
}

#[test]
fn importar_con_una_frase_corta_no_cambia_el_libro() {
    let error = importar_y_comprobar_que_no_cambia(&copia_cifrada_de_la_laptop(), Some("corta"));
    assert_eq!(error, ErrorRespaldo::FraseCorta);
}

#[test]
fn importar_una_version_no_soportada_no_cambia_el_libro() {
    let claro = con_version_99(&copia_legible_de_la_laptop());

    // En texto legible.
    let error = importar_y_comprobar_que_no_cambia(&claro, None);
    assert!(
        matches!(error, ErrorRespaldo::VersionNoSoportada { archivo: 99, .. }),
        "error inesperado: {error}"
    );

    // Cifrada con la frase correcta: la frase no basta si la versión no se entiende.
    let cifrada = cifrador().cifrar(FRASE, &claro).unwrap();
    let error = importar_y_comprobar_que_no_cambia(&cifrada, Some(FRASE));
    assert!(
        matches!(error, ErrorRespaldo::VersionNoSoportada { archivo: 99, .. }),
        "error inesperado: {error}"
    );
}

#[test]
fn importar_un_contenedor_de_version_futura_no_cambia_el_libro() {
    // El quinto byte del contenedor cifrado (`CCBK`) es la versión del contenedor.
    let mut copia = copia_cifrada_de_la_laptop();
    copia[4] = 2;
    let error = importar_y_comprobar_que_no_cambia(&copia, Some(FRASE));
    assert_eq!(
        error,
        ErrorRespaldo::VersionNoSoportada {
            archivo: 2,
            soportada: 1
        }
    );
}

#[test]
fn importar_un_archivo_que_no_es_una_copia_no_cambia_el_libro() {
    let error =
        importar_y_comprobar_que_no_cambia(b"esto no es una copia de seguridad", Some(FRASE));
    assert!(
        matches!(
            error,
            ErrorRespaldo::FraseIncorrectaOArchivoDanado | ErrorRespaldo::ArchivoInvalido
        ),
        "error inesperado: {error}"
    );
}

#[test]
fn importar_una_copia_legible_con_una_linea_danada_no_cambia_el_libro() {
    let mut claro = copia_legible_de_la_laptop();
    claro.extend_from_slice(b"{esto no es un evento\n");
    let error = importar_y_comprobar_que_no_cambia(&claro, None);
    assert_eq!(error, ErrorRespaldo::ArchivoInvalido);
}

// ---------- Sincronizar ----------

/// Envuelve un transporte real y le inyecta un fallo en `traer` o en `publicar`, para
/// reproducir sin red los errores del destino.
struct ConFallo<T: TransporteSincronizacion> {
    interno: T,
    fallo_al_traer: Option<ErrorTransporte>,
    fallo_al_publicar: Option<ErrorTransporte>,
}

impl<T: TransporteSincronizacion> ConFallo<T> {
    fn nuevo(interno: T) -> ConFallo<T> {
        ConFallo {
            interno,
            fallo_al_traer: None,
            fallo_al_publicar: None,
        }
    }
}

impl<T: TransporteSincronizacion> TransporteSincronizacion for ConFallo<T> {
    fn dispositivo_local(&self) -> &str {
        self.interno.dispositivo_local()
    }

    fn traer(&mut self) -> Result<(), ErrorTransporte> {
        match &self.fallo_al_traer {
            Some(error) => Err(error.clone()),
            None => self.interno.traer(),
        }
    }

    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte> {
        self.interno.listar_dispositivos()
    }

    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        self.interno.listar_lotes(dispositivo)
    }

    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte> {
        self.interno.leer_lote(dispositivo, lote)
    }

    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte> {
        self.interno.escribir_lote(lote, contenido)
    }

    fn publicar(&mut self) -> Result<(), ErrorTransporte> {
        match &self.fallo_al_publicar {
            Some(error) => Err(error.clone()),
            None => self.interno.publicar(),
        }
    }
}

type Resultado = Result<(InformeSincronizacion, EstadoSincronizacion), ErrorSincronizacion>;

fn sincronizar_con(
    casos: &mut CasosDeUso,
    transporte: &mut dyn TransporteSincronizacion,
    frase: &str,
    estado: &EstadoSincronizacion,
) -> Resultado {
    sincronizar(casos, transporte, &cifrador(), frase, AHORA, estado)
}

fn salio_bien(resultado: Resultado) -> (InformeSincronizacion, EstadoSincronizacion) {
    match resultado {
        Ok(salida) => salida,
        Err(e) => panic!("se esperaba que sincronizara bien y falló: {e}"),
    }
}

fn fallo_de(resultado: Resultado) -> ErrorSincronizacion {
    match resultado {
        Ok(_) => panic!("se esperaba un error al sincronizar"),
        Err(e) => e,
    }
}

fn transporte_de(destino: &Path, dispositivo: &str) -> TransporteCarpeta {
    TransporteCarpeta::nuevo(destino, dispositivo).unwrap()
}

/// La laptop publica su libro en la carpeta compartida con la frase correcta.
fn laptop_publica(destino: &Path) -> BTreeSet<String> {
    let mut laptop = libro("laptop-beto", "lap", &["Efectivo", "Bóveda"]);
    let mut transporte = transporte_de(destino, "laptop-beto");
    let estado = EstadoSincronizacion::default();
    salio_bien(sincronizar_con(
        &mut laptop,
        &mut transporte,
        FRASE,
        &estado,
    ));
    ids(&laptop)
}

#[test]
fn un_destino_que_falla_no_cambia_el_libro_y_lo_pendiente_sale_despues() {
    let fallos = [
        ErrorTransporte::SinConexion,
        ErrorTransporte::DestinoInaccesible,
        ErrorTransporte::CredencialesRechazadas,
    ];
    for fallo in fallos {
        let carpeta = CarpetaTemporal::nueva();
        let mut laptop = libro("laptop-beto", "lap", &["Efectivo", "Ahorros"]);
        let antes = instantanea(&laptop);
        let total = cantidad_de_eventos(&laptop);

        let mut transporte = ConFallo::nuevo(transporte_de(&carpeta.destino(), "laptop-beto"));
        transporte.fallo_al_traer = Some(fallo.clone());
        let estado = EstadoSincronizacion::default();

        let error = fallo_de(sincronizar_con(
            &mut laptop,
            &mut transporte,
            FRASE,
            &estado,
        ));
        assert_eq!(error, ErrorSincronizacion::Transporte(fallo.clone()));
        assert_eq!(instantanea(&laptop), antes, "el libro cambió con {fallo}");
        assert!(
            archivos_de(&carpeta.destino()).is_empty(),
            "no debía escribirse nada con {fallo}"
        );

        // Se arregla el problema: lo pendiente sale completo y una sola vez.
        transporte.fallo_al_traer = None;
        let (informe, estado) = salio_bien(sincronizar_con(
            &mut laptop,
            &mut transporte,
            FRASE,
            &estado,
        ));
        assert_eq!(informe.enviados, total);
        assert_eq!(archivos_de(&carpeta.destino()).len(), 1);
        assert_eq!(instantanea(&laptop), antes);

        let (informe, _) = salio_bien(sincronizar_con(
            &mut laptop,
            &mut transporte,
            FRASE,
            &estado,
        ));
        assert_eq!((informe.enviados, informe.recibidos), (0, 0));
    }
}

#[test]
fn un_fallo_al_publicar_no_pierde_nada_y_el_reintento_converge() {
    let carpeta = CarpetaTemporal::nueva();
    let mut laptop = libro("laptop-beto", "lap", &["Efectivo", "Ahorros"]);
    let antes = instantanea(&laptop);
    let total = cantidad_de_eventos(&laptop);

    let mut transporte = ConFallo::nuevo(transporte_de(&carpeta.destino(), "laptop-beto"));
    transporte.fallo_al_publicar = Some(ErrorTransporte::DestinoInaccesible);
    let estado = EstadoSincronizacion::default();

    let error = fallo_de(sincronizar_con(
        &mut laptop,
        &mut transporte,
        FRASE,
        &estado,
    ));
    assert_eq!(
        error,
        ErrorSincronizacion::Transporte(ErrorTransporte::DestinoInaccesible)
    );
    assert_eq!(instantanea(&laptop), antes);

    // El reintento envía todo otra vez: el estado no avanzó con el fallo.
    transporte.fallo_al_publicar = None;
    let (informe, _) = salio_bien(sincronizar_con(
        &mut laptop,
        &mut transporte,
        FRASE,
        &estado,
    ));
    assert_eq!(informe.enviados, total);

    // El teléfono recibe todo, una sola vez, aunque el destino tenga el lote repetido.
    let mut telefono = libro("telefono-beto", "tel", &[]);
    let mut transporte_telefono = transporte_de(&carpeta.destino(), "telefono-beto");
    let estado_telefono = EstadoSincronizacion::default();
    let (informe, _) = salio_bien(sincronizar_con(
        &mut telefono,
        &mut transporte_telefono,
        FRASE,
        &estado_telefono,
    ));
    assert_eq!(informe.recibidos, total);
    assert_eq!(ids(&telefono), ids(&laptop));
    assert_eq!(cantidad_de_eventos(&telefono), total);
}

#[test]
fn sincronizar_con_otra_frase_no_importa_nada_ni_cambia_el_libro() {
    let carpeta = CarpetaTemporal::nueva();
    laptop_publica(&carpeta.destino());
    let lotes_antes = archivos_de(&carpeta.destino()).len();

    let mut telefono = libro("telefono-beto", "tel", &["Ahorros"]);
    let antes = instantanea(&telefono);
    let mut transporte = transporte_de(&carpeta.destino(), "telefono-beto");

    let error = fallo_de(sincronizar_con(
        &mut telefono,
        &mut transporte,
        OTRA_FRASE,
        &EstadoSincronizacion::default(),
    ));
    assert_eq!(
        error,
        ErrorSincronizacion::Respaldo(ErrorRespaldo::FraseIncorrectaOArchivoDanado)
    );
    assert_eq!(instantanea(&telefono), antes);
    assert_eq!(
        archivos_de(&carpeta.destino()).len(),
        lotes_antes,
        "el teléfono no debe escribir nada si no pudo leer el destino"
    );
}

#[test]
fn sincronizar_con_un_lote_alterado_no_importa_nada_ni_cambia_el_libro() {
    let carpeta = CarpetaTemporal::nueva();
    laptop_publica(&carpeta.destino());

    // Alguien altera el lote en el destino (no pasa por la aplicación).
    let archivos = archivos_de(&carpeta.destino());
    assert_eq!(archivos.len(), 1);
    let original = fs::read(&archivos[0]).unwrap();
    fs::write(&archivos[0], con_byte_alterado(&original, 1)).unwrap();

    let mut telefono = libro("telefono-beto", "tel", &["Ahorros"]);
    let antes = instantanea(&telefono);
    let mut transporte = transporte_de(&carpeta.destino(), "telefono-beto");

    let error = fallo_de(sincronizar_con(
        &mut telefono,
        &mut transporte,
        FRASE,
        &EstadoSincronizacion::default(),
    ));
    assert_eq!(
        error,
        ErrorSincronizacion::Respaldo(ErrorRespaldo::FraseIncorrectaOArchivoDanado)
    );
    assert_eq!(instantanea(&telefono), antes);
}

#[test]
fn sincronizar_con_un_lote_de_version_no_soportada_no_cambia_el_libro() {
    let carpeta = CarpetaTemporal::nueva();

    // Un lote de una versión futura del formato, cifrado con la frase correcta.
    let laptop = libro("laptop-beto", "lap", &["Efectivo"]);
    let claro = con_version_99(&laptop.exportar_copia(None, &cifrador()).unwrap());
    let lote = cifrador().cifrar(FRASE, &claro).unwrap();
    let mut transporte_laptop = transporte_de(&carpeta.destino(), "laptop-beto");
    transporte_laptop
        .escribir_lote(&nombre_de_lote(AHORA, 1), &lote)
        .unwrap();
    transporte_laptop.publicar().unwrap();

    let mut telefono = libro("telefono-beto", "tel", &["Ahorros"]);
    let antes = instantanea(&telefono);
    let mut transporte = transporte_de(&carpeta.destino(), "telefono-beto");

    let error = fallo_de(sincronizar_con(
        &mut telefono,
        &mut transporte,
        FRASE,
        &EstadoSincronizacion::default(),
    ));
    assert!(
        matches!(
            error,
            ErrorSincronizacion::Respaldo(ErrorRespaldo::VersionNoSoportada { archivo: 99, .. })
        ),
        "error inesperado: {error}"
    );
    assert_eq!(instantanea(&telefono), antes);
}
