//! Pruebas de arquitectura: verifican el diseño, no el comportamiento.
//!
//! Fallan si se rompe la regla de dependencia (RNF-13) o si aparece coma
//! flotante en el dominio (INV-03). Corren con `cargo test --workspace`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Dependencias externas que `dominio` y `aplicacion` no pueden usar.
const EXTERNAS: [&str; 3] = ["rusqlite", "tauri", "reqwest"];

// ---------- Utilidades ----------

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dir_crate(nombre: &str) -> PathBuf {
    raiz().join("crates").join(nombre)
}

fn leer(ruta: &Path) -> String {
    fs::read_to_string(ruta).unwrap_or_else(|e| panic!("No se pudo leer {}: {e}", ruta.display()))
}

/// Un nombre es prohibido si coincide con una prohibida o es de su familia
fn es_prohibida(nombre: &str, prohibidas: &[&str]) -> bool {
    prohibidas
        .iter()
        .any(|p| nombre == *p || nombre.starts_with(&format!("{p}-")))
}

/// Devuelve las dependencias prohibidas que declara un Cargo.toml.
/// Solo mira las secciones de dependencias, no el resto del archivo.
fn dependencias_prohibidas(cargo_toml: &str, prohibidas: &[&str]) -> Vec<String> {
    let mut encontradas = Vec::new();
    let mut en_dependencias = false;

    for linea in cargo_toml.lines() {
        let linea = linea.trim();
        if linea.starts_with('[') {
            let cabecera = linea.trim_matches(|c| c == '[' || c == ']');
            en_dependencias = cabecera.ends_with("dependencies");
            // Forma `[dependencies.reqwest]`
            if let Some((_, nombre)) = cabecera.rsplit_once("dependencies.")
                && es_prohibida(nombre, prohibidas)
            {
                encontradas.push(nombre.to_string());
            }
            continue;
        }
        if !en_dependencias || linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let clave = linea.split('=').next().unwrap_or("").trim();
        let nombre = clave.split('.').next().unwrap_or("").trim();
        if es_prohibida(nombre, prohibidas) {
            encontradas.push(nombre.to_string());
        }
    }
    encontradas
}

/// Busca `palabra` como identificador completo (no dentro de otro nombre).
fn contiene_palabra(texto: &str, palabra: &str) -> bool {
    texto.match_indices(palabra).any(|(i, _)| {
        let antes = texto[..i].chars().next_back();
        let despues = texto[i + palabra.len()..].chars().next();
        let antes_ok = !antes.is_some_and(|c| c.is_alphabetic() || c == '_');
        let despues_ok = !despues.is_some_and(|c| c.is_alphanumeric() || c == '_');
        antes_ok && despues_ok
    })
}

fn archivos_rs(dir: &Path, acumulado: &mut Vec<PathBuf>) {
    let entradas =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("No se pudo leer {}: {e}", dir.display()));
    for entrada in entradas {
        let ruta = entrada.expect("entrada de directorio inválida").path();
        if ruta.is_dir() {
            archivos_rs(&ruta, acumulado);
        } else if ruta.extension().is_some_and(|e| e == "rs") {
            acumulado.push(ruta);
        }
    }
}

/// Dependencias DIRECTAS: lee el Cargo.toml del crate.
fn comprobar_dependencias(nombre: &str, prohibidas: &[&str]) {
    let toml = leer(&dir_crate(nombre).join("Cargo.toml"));
    let halladas = dependencias_prohibidas(&toml, prohibidas);
    assert!(
        halladas.is_empty(),
        "El crate `{nombre}` no puede depender de: {}. Revisa su Cargo.toml (RNF-13).",
        halladas.join(", ")
    );
}

/// Nombres de todas las dependencias (directas y transitivas) de un crate,
/// obtenidas con `cargo tree`.
fn dependencias_transitivas(crate_nombre: &str) -> BTreeSet<String> {
    let salida = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            crate_nombre,
            "-e",
            "normal,build",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .current_dir(raiz())
        .output()
        .expect("No se pudo ejecutar cargo tree");
    assert!(
        salida.status.success(),
        "cargo tree falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    String::from_utf8_lossy(&salida.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Dependencias TRANSITIVAS: recorre todo el árbol del crate.
fn comprobar_transitivas(nombre: &str, prohibidas: &[&str]) {
    let halladas: Vec<String> = dependencias_transitivas(nombre)
        .into_iter()
        .filter(|d| es_prohibida(d, prohibidas))
        .collect();
    assert!(
        halladas.is_empty(),
        "El crate `{nombre}` arrastra dependencias prohibidas: {}. Revisa con `cargo tree -p {nombre}` (RNF-13).",
        halladas.join(", ")
    );
}

// ---------- Regla de dependencia: declaradas en el Cargo.toml ----------

#[test]
fn dominio_no_depende_de_infraestructura_externa() {
    let mut prohibidas = EXTERNAS.to_vec();
    prohibidas.extend(["aplicacion", "infraestructura"]);
    comprobar_dependencias("dominio", &prohibidas);
}

#[test]
fn aplicacion_no_depende_de_infraestructura_externa() {
    let mut prohibidas = EXTERNAS.to_vec();
    prohibidas.push("infraestructura");
    comprobar_dependencias("aplicacion", &prohibidas);
}

#[test]
fn infraestructura_no_depende_de_reqwest() {
    comprobar_dependencias("infraestructura", &["reqwest"]);
}

// ---------- Regla de dependencia: todo el árbol (transitivas) ----------

#[test]
fn dominio_no_arrastra_dependencias_externas() {
    let mut prohibidas = EXTERNAS.to_vec();
    prohibidas.extend(["aplicacion", "infraestructura"]);
    comprobar_transitivas("dominio", &prohibidas);
}

#[test]
fn aplicacion_no_arrastra_dependencias_externas() {
    let mut prohibidas = EXTERNAS.to_vec();
    prohibidas.push("infraestructura");
    comprobar_transitivas("aplicacion", &prohibidas);
}

#[test]
fn infraestructura_no_arrastra_reqwest() {
    comprobar_transitivas("infraestructura", &["reqwest"]);
}

// ---------- INV-03: sin coma flotante en el dominio ----------

#[test]
fn dominio_no_usa_coma_flotante() {
    let mut archivos = Vec::new();
    archivos_rs(&dir_crate("dominio").join("src"), &mut archivos);

    let mut infracciones = Vec::new();
    for ruta in archivos {
        for (n, linea) in leer(&ruta).lines().enumerate() {
            let codigo = linea.split("//").next().unwrap_or("");
            for tipo in ["f32", "f64"] {
                if contiene_palabra(codigo, tipo) {
                    infracciones.push(format!("{}:{}: usa {tipo}", ruta.display(), n + 1));
                }
            }
        }
    }
    assert!(
        infracciones.is_empty(),
        "El dominio no puede usar f32 ni f64 (INV-03):\n{}",
        infracciones.join("\n")
    );
}

// ---------- Pruebas de las propias pruebas ----------

#[test]
fn detecta_dependencias_prohibidas() {
    let toml = "[package]\nname = \"dominio\"\n\n[dependencies]\nrust_decimal = \"1\"\nreqwest = { version = \"0.12\" }\n\n[dev-dependencies]\ntauri-plugin-log = \"2\"\n";
    assert_eq!(
        dependencias_prohibidas(toml, &EXTERNAS),
        vec!["reqwest", "tauri-plugin-log"]
    );
}

#[test]
fn ignora_texto_fuera_de_las_dependencias() {
    let toml = "[package]\nname = \"dominio\"\ndescription = \"sin tauri ni reqwest\"\n\n[dependencies]\nrust_decimal = \"1\"\n";
    assert!(dependencias_prohibidas(toml, &EXTERNAS).is_empty());
}

#[test]
fn detecta_flotantes_como_palabra_completa() {
    assert!(contiene_palabra("let x: f64 = 1.0;", "f64"));
    assert!(contiene_palabra("let x = 1.0f64;", "f64"));
    assert!(contiene_palabra("let x: f32 = 0.0;", "f32"));
    assert!(!contiene_palabra("let inf64 = 1;", "f64"));
    assert!(!contiene_palabra("let f64_total = 1;", "f64"));
}
