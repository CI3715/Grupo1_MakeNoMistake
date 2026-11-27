//! Verificación de privacidad del destino y de la exportación (Entrega 8: Katherine -
//! Tarea 4; RNF-01, RF-INT-05, RF-INT-06, INV-16).
//!
//! Comprueba con archivos reales que:
//!
//! 1. La carpeta compartida contiene solo archivos cifrados: ningún dato legible del libro
//!    (cuentas, montos, tipos de evento), ni la frase de paso, ni credenciales de Git.
//! 2. La exportación, cifrada o no, nunca incluye la frase de paso ni credenciales.
//! 3. El escáner no es de adorno: detecta un archivo en claro que alguien deje en el destino.
//!
//! El contenido de un lote es, por especificación (sección 2.5), el mismo formato cifrado
//! de la exportación; por eso aquí se usa `exportar_copia` con frase para fabricarlo.
//!
//! Pendiente de conectar: cuando existan el caso de uso de sincronizar (Cristina - Tarea 5)
//! y el adaptador Git (John - Tarea 5), esta misma comprobación debe correr sobre lo que
//! ellos escriben de verdad, sobre la copia de trabajo del repositorio y sobre el archivo
//! SQLite del libro (para confirmar que allí no quedan la frase ni el token).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use aplicacion::{
    CasosDeUso, DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria,
    TransporteSincronizacion, nombre_de_lote,
};
use infraestructura::cifrado::{CifradorArgon2, MAGIA_CONTENEDOR, ParametrosArgon2};
use infraestructura::transporte_carpeta::TransporteCarpeta;

const FRASE: &str = "frase-de-paso-de-beto-2026";
const TOKEN_GIT: &str = "ghp_TokenSecretoDeGitDeBeto0123456789";
const USUARIO_GIT: &str = "beto-usuario-git-secreto";

/// Textos del libro que nunca deben verse en el destino ni en un archivo cifrado.
const DATOS_DEL_LIBRO: [&str; 6] = [
    "Bóveda Secreta Beto",
    "Ahorros Ocultos Beto",
    "15432.89",
    "CuentaCreada",
    "saldo_inicial",
    "cuentas-claras-respaldo",
];

/// Secretos de acceso que nunca deben salir del almacén del sistema operativo.
const SECRETOS_DE_ACCESO: [&str; 3] = [FRASE, TOKEN_GIT, USUARIO_GIT];

static CONTADOR: AtomicU32 = AtomicU32::new(0);

/// Carpeta temporal única que se borra al terminar la prueba.
struct CarpetaTemporal {
    base: PathBuf,
}

impl CarpetaTemporal {
    fn nueva() -> CarpetaTemporal {
        let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("cc_privacidad_{}_{n}", std::process::id()));
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

/// Parámetros ligeros de Argon2id: la prueba es de privacidad, no de resistencia de la
/// derivación (esa la cubren las pruebas de cifrado de Angel).
fn cifrador() -> CifradorArgon2 {
    CifradorArgon2::con_parametros(ParametrosArgon2 {
        memoria_kib: 1024,
        iteraciones: 1,
        paralelismo: 1,
    })
}

fn casos_con(nombre: &str, dispositivo: &str) -> CasosDeUso {
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-25T10:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo(dispositivo)),
    );
    app.crear_cuenta(NuevaCuenta {
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "15432.89".to_string(),
    })
    .unwrap();
    app
}

/// Todos los archivos bajo `dir`, en cualquier profundidad.
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

fn contiene(bytes: &[u8], aguja: &str) -> bool {
    let aguja = aguja.as_bytes();
    !aguja.is_empty() && bytes.windows(aguja.len()).any(|ventana| ventana == aguja)
}

/// Qué textos prohibidos aparecen en el contenido o en el nombre de algún archivo bajo `dir`.
/// Cada hallazgo dice en qué archivo y cuál texto.
fn filtraciones_en(dir: &Path, prohibidos: &[&str]) -> Vec<String> {
    let mut halladas = Vec::new();
    for ruta in archivos_de(dir) {
        let contenido = fs::read(&ruta).unwrap();
        let nombre = ruta.to_string_lossy().to_string();
        for texto in prohibidos {
            if contiene(&contenido, texto) {
                halladas.push(format!("{} (contenido) muestra «{texto}»", ruta.display()));
            }
            if nombre.contains(texto) {
                halladas.push(format!("{} (nombre) muestra «{texto}»", ruta.display()));
            }
        }
    }
    halladas
}

fn todo_lo_prohibido() -> Vec<&'static str> {
    let mut todo: Vec<&'static str> = Vec::new();
    todo.extend(DATOS_DEL_LIBRO);
    todo.extend(SECRETOS_DE_ACCESO);
    todo
}

/// Escribe un lote cifrado de `casos` en la carpeta compartida, como lo hace un dispositivo.
fn publicar_lote(destino: &Path, dispositivo: &str, casos: &CasosDeUso, secuencia: u32) {
    let cifrado = casos.exportar_copia(Some(FRASE), &cifrador()).unwrap();
    let mut transporte = TransporteCarpeta::nuevo(destino, dispositivo).unwrap();
    let lote = nombre_de_lote("2026-11-25T10:00:00Z", secuencia);
    transporte.escribir_lote(&lote, &cifrado).unwrap();
    transporte.publicar().unwrap();
}

#[test]
fn la_carpeta_compartida_solo_contiene_archivos_cifrados() {
    let carpeta = CarpetaTemporal::nueva();
    publicar_lote(
        &carpeta.destino(),
        "laptop-beto",
        &casos_con("Bóveda Secreta Beto", "laptop-beto"),
        1,
    );
    publicar_lote(
        &carpeta.destino(),
        "telefono-beto",
        &casos_con("Ahorros Ocultos Beto", "telefono-beto"),
        1,
    );

    let archivos = archivos_de(&carpeta.destino());
    assert_eq!(archivos.len(), 2, "se esperaban dos lotes: {archivos:?}");
    for ruta in &archivos {
        let contenido = fs::read(ruta).unwrap();
        assert!(
            contenido.starts_with(MAGIA_CONTENEDOR),
            "{} no es un contenedor cifrado",
            ruta.display()
        );
        let nombre = ruta.file_name().unwrap().to_string_lossy().to_string();
        assert!(!nombre.ends_with(".tmp"), "quedó un temporal: {nombre}");
    }

    let halladas = filtraciones_en(&carpeta.destino(), &todo_lo_prohibido());
    assert!(halladas.is_empty(), "el destino filtra datos: {halladas:?}");
}

#[test]
fn el_destino_no_guarda_nada_mas_que_lotes_de_cada_dispositivo() {
    let carpeta = CarpetaTemporal::nueva();
    publicar_lote(
        &carpeta.destino(),
        "laptop-beto",
        &casos_con("Bóveda Secreta Beto", "laptop-beto"),
        1,
    );

    // Ni archivos de configuración ni de credenciales junto a los lotes.
    for ruta in archivos_de(&carpeta.destino()) {
        let relativa = ruta.strip_prefix(carpeta.destino()).unwrap();
        let partes = relativa.components().count();
        assert_eq!(
            partes,
            2,
            "se esperaba <dispositivo>/<lote>, pero hay {}",
            relativa.display()
        );
    }
}

#[test]
fn la_exportacion_cifrada_no_muestra_el_libro_ni_la_frase() {
    let casos = casos_con("Bóveda Secreta Beto", "laptop-beto");
    let bytes = casos.exportar_copia(Some(FRASE), &cifrador()).unwrap();

    assert!(bytes.starts_with(MAGIA_CONTENEDOR));
    for texto in todo_lo_prohibido() {
        assert!(
            !contiene(&bytes, texto),
            "la exportación cifrada muestra «{texto}»"
        );
    }
}

#[test]
fn la_exportacion_sin_cifrar_no_incluye_frase_ni_credenciales() {
    // RF-INT-05: la copia legible sirve para revisar el libro, pero jamás lleva secretos.
    let casos = casos_con("Bóveda Secreta Beto", "laptop-beto");
    let bytes = casos.exportar_copia(None, &cifrador()).unwrap();

    assert!(contiene(&bytes, "Bóveda Secreta Beto"), "debe ser legible");
    for secreto in SECRETOS_DE_ACCESO {
        assert!(
            !contiene(&bytes, secreto),
            "la exportación sin cifrar incluye «{secreto}»"
        );
    }
}

#[test]
fn el_escaner_detecta_un_archivo_en_claro_en_el_destino() {
    let carpeta = CarpetaTemporal::nueva();
    publicar_lote(
        &carpeta.destino(),
        "laptop-beto",
        &casos_con("Bóveda Secreta Beto", "laptop-beto"),
        1,
    );
    assert!(filtraciones_en(&carpeta.destino(), &todo_lo_prohibido()).is_empty());

    // Alguien deja por error una copia legible y un archivo con el token.
    let casos = casos_con("Bóveda Secreta Beto", "laptop-beto");
    let claro = casos.exportar_copia(None, &cifrador()).unwrap();
    fs::write(carpeta.destino().join("copia-olvidada.txt"), claro).unwrap();
    fs::write(carpeta.destino().join("config.txt"), TOKEN_GIT).unwrap();

    let halladas = filtraciones_en(&carpeta.destino(), &todo_lo_prohibido());
    assert!(
        halladas.iter().any(|h| h.contains("Bóveda Secreta Beto")),
        "no detectó el libro en claro: {halladas:?}"
    );
    assert!(
        halladas.iter().any(|h| h.contains(TOKEN_GIT)),
        "no detectó el token: {halladas:?}"
    );
}
