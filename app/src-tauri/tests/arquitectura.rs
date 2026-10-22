//! Pruebas de arquitectura: verifican el diseño, no el comportamiento.
//!
//! Fallan si se rompe la regla de dependencia (RNF-13), si aparece coma
//! flotante (tipos, conversiones o literales como `1.5`) en el dominio o en la
//! aplicación (INV-03), si los comandos de
//! cuentas y movimientos tocan la red (INV-16, RNF-01) o si el código ejecuta
//! UPDATE o DELETE sobre la tabla `eventos` (INV-04).
//! Corren con `cargo test --workspace`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Dependencias externas que `dominio` y `aplicacion` no pueden usar.
const EXTERNAS: [&str; 3] = ["rusqlite", "tauri", "reqwest"];

/// Métodos que convierten desde o hacia coma flotante sin nombrar `f32`/`f64`
/// como palabra suelta (por ejemplo `Decimal::from_f64`).
const METODOS_FLOTANTES: [&str; 6] = [
    "from_f64",
    "from_f32",
    "to_f64",
    "to_f32",
    "from_f64_retain",
    "from_f32_retain",
];

/// Nombres que delatan acceso a la red (en minúsculas). Solo `ping.rs` puede usarlos.
const RED_PROHIBIDA: [&str; 9] = [
    "reqwest",
    "hyper",
    "ureq",
    "http://",
    "https://",
    "std::net",
    "tokio::net",
    "tcpstream",
    "udpsocket",
];

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

/// Todos los `.rs` bajo `dir`, ordenados para que los mensajes sean estables.
fn lista_rs(dir: &Path) -> Vec<PathBuf> {
    let mut archivos = Vec::new();
    archivos_rs(dir, &mut archivos);
    archivos.sort();
    archivos
}

// ---------- Análisis del código fuente (comentarios y textos) ----------

/// Indica si la posición `i` empieza una palabra nueva (no está dentro de un identificador).
fn es_inicio_de_palabra(c: &[char], i: usize) -> bool {
    i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_')
}

/// Destino del código limpio. Mientras `omitiendo` es verdadero solo deja pasar
/// los saltos de línea, para que los números de línea no cambien.
struct Salida {
    texto: String,
    omitiendo: bool,
}

impl Salida {
    fn caracter(&mut self, c: char) {
        if !self.omitiendo || c == '\n' {
            self.texto.push(c);
        }
    }

    /// Copia un texto entre comillas: completo, o vacío si `quitar` es verdadero.
    fn trozo(&mut self, trozo: &[char], quitar: bool) {
        if self.omitiendo || quitar {
            if quitar && !self.omitiendo {
                self.texto.push_str("\"\"");
            }
            self.texto.extend(trozo.iter().filter(|c| **c == '\n'));
        } else {
            self.texto.extend(trozo.iter());
        }
    }
}

/// Quita del código Rust los comentarios de línea, los de bloque (también
/// anidados) y, si `quitar_textos` es verdadero, el contenido de las cadenas
/// (normales y crudas) y de los caracteres. Si `sin_pruebas` es verdadero, quita
/// además cada elemento marcado con `#[cfg(test)]` (el módulo de pruebas completo,
/// esté donde esté en el archivo) y deja intacto el resto. Conserva los saltos de línea.
fn limpiar_codigo(fuente: &str, quitar_textos: bool, sin_pruebas: bool) -> String {
    let c: Vec<char> = fuente.chars().collect();
    let marca: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut salida = Salida {
        texto: String::with_capacity(fuente.len()),
        omitiendo: false,
    };
    let mut i = 0;
    // Llaves abiertas en el código (las de comentarios y textos no cuentan).
    let mut profundidad: i32 = 0;
    // Elemento de pruebas que se está omitiendo: (profundidad al empezar, ya abrió su llave).
    let mut omision: Option<(i32, bool)> = None;

    while i < c.len() {
        let actual = c[i];
        let siguiente = c.get(i + 1).copied();

        // Comentario de línea.
        if actual == '/' && siguiente == Some('/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Comentario de bloque, con anidamiento.
        if actual == '/' && siguiente == Some('*') {
            let mut anidados = 1;
            i += 2;
            while i < c.len() && anidados > 0 {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                    anidados += 1;
                    i += 2;
                } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    anidados -= 1;
                    i += 2;
                } else {
                    if c[i] == '\n' {
                        salida.caracter('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // Cadena cruda: r"..." o r#"..."#.
        if actual == 'r' && es_inicio_de_palabra(&c, i) {
            let mut j = i + 1;
            let mut almohadillas = 0;
            while c.get(j) == Some(&'#') {
                almohadillas += 1;
                j += 1;
            }
            if c.get(j) == Some(&'"') {
                let mut fin = c.len();
                let mut k = j + 1;
                while k < c.len() {
                    if c[k] == '"' && (0..almohadillas).all(|h| c.get(k + 1 + h) == Some(&'#')) {
                        fin = k + 1 + almohadillas;
                        break;
                    }
                    k += 1;
                }
                salida.trozo(&c[i..fin], quitar_textos);
                i = fin;
                continue;
            }
        }

        // Cadena normal, con escapes.
        if actual == '"' {
            let mut j = i + 1;
            while j < c.len() && c[j] != '"' {
                if c[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            let fin = (j + 1).min(c.len());
            salida.trozo(&c[i..fin], quitar_textos);
            i = fin;
            continue;
        }

        // Carácter ('x', '\n', '\'', '"') o parámetro de tiempo de vida ('a).
        if actual == '\'' {
            if siguiente == Some('\\') {
                let mut j = i + 3;
                while j < c.len() && c[j] != '\'' {
                    j += 1;
                }
                let fin = (j + 1).min(c.len());
                salida.trozo(&c[i..fin], quitar_textos);
                i = fin;
                continue;
            }
            if c.get(i + 2) == Some(&'\'') {
                salida.trozo(&c[i..i + 3], quitar_textos);
                i += 3;
                continue;
            }
        }

        // Inicio de un elemento de pruebas: se omite hasta que cierre su llave
        // (o hasta el `;` si no tiene cuerpo, como `#[cfg(test)] use ...;`).
        if sin_pruebas && omision.is_none() && c[i..].starts_with(&marca) {
            omision = Some((profundidad, false));
            salida.omitiendo = true;
            i += marca.len();
            continue;
        }

        if actual == '{' {
            profundidad += 1;
            if let Some((base, false)) = omision
                && profundidad == base + 1
            {
                omision = Some((base, true));
            }
        } else if actual == '}' {
            profundidad -= 1;
            if let Some((base, true)) = omision
                && profundidad == base
            {
                // Cierre del elemento de pruebas: tampoco se copia.
                omision = None;
                salida.omitiendo = false;
                i += 1;
                continue;
            }
        } else if actual == ';'
            && let Some((base, false)) = omision
            && profundidad == base
        {
            omision = None;
            salida.omitiendo = false;
            i += 1;
            continue;
        }

        salida.caracter(actual);
        i += 1;
    }
    salida.texto
}

/// Literales de coma flotante (`1.5`, `2e10`, `1_000.25`) con su número de línea.
/// Los que van dentro de `dec!(...)` no cuentan: son `Decimal` exactos, no `f64`.
/// Hay que pasarle código ya limpio (sin comentarios ni textos).
fn literales_flotantes(codigo: &str) -> Vec<(usize, String)> {
    let c: Vec<char> = codigo.chars().collect();
    let mut halladas = Vec::new();
    let mut linea = 1;
    let mut dentro_de_dec: Option<i32> = None; // paréntesis abiertos dentro de dec!(
    let mut i = 0;

    while i < c.len() {
        if c[i] == '\n' {
            linea += 1;
        }

        if dentro_de_dec.is_none()
            && c[i..].starts_with(&['d', 'e', 'c', '!', '('])
            && es_inicio_de_palabra(&c, i)
        {
            dentro_de_dec = Some(1);
            i += 5;
            continue;
        }
        if let Some(abiertos) = dentro_de_dec {
            match c[i] {
                '(' => dentro_de_dec = Some(abiertos + 1),
                ')' => {
                    dentro_de_dec = if abiertos == 1 {
                        None
                    } else {
                        Some(abiertos - 1)
                    }
                }
                _ => {}
            }
            i += 1;
            continue;
        }

        // Un número empieza donde no continúa un nombre ni un campo (`a.0.1`).
        let empieza_numero = c[i].is_ascii_digit()
            && (i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_' || c[i - 1] == '.'));
        if empieza_numero {
            let mut j = i;
            while j < c.len() && (c[j].is_ascii_digit() || c[j] == '_') {
                j += 1;
            }
            let con_punto =
                c.get(j) == Some(&'.') && c.get(j + 1).is_some_and(char::is_ascii_digit);
            let con_exponente = matches!(c.get(j), Some('e' | 'E'))
                && (c.get(j + 1).is_some_and(char::is_ascii_digit)
                    || (matches!(c.get(j + 1), Some('+' | '-'))
                        && c.get(j + 2).is_some_and(char::is_ascii_digit)));
            if con_punto || con_exponente {
                let mut fin = j + 1;
                while fin < c.len() && (c[fin].is_ascii_digit() || c[fin] == '_') {
                    fin += 1;
                }
                halladas.push((linea, c[i..fin].iter().collect()));
                i = fin;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    halladas
}

/// Si la línea usa coma flotante, devuelve qué la delata.
fn flotante_en(codigo: &str) -> Option<String> {
    for tipo in ["f32", "f64"] {
        if contiene_palabra(codigo, tipo) {
            return Some(tipo.to_string());
        }
    }
    METODOS_FLOTANTES
        .iter()
        .find(|m| codigo.contains(**m))
        .map(|m| (*m).to_string())
}

/// Nombres de acceso a la red que aparecen en `codigo` (ya sin comentarios).
fn red_en(codigo: &str) -> Vec<&'static str> {
    let minusculas = codigo.to_lowercase();
    RED_PROHIBIDA
        .iter()
        .copied()
        .filter(|nombre| minusculas.contains(nombre))
        .collect()
}

/// Sentencias SQL que modifican o destruyen la tabla `eventos`: UPDATE, DELETE,
/// REPLACE, DROP, ALTER o TRUNCATE sobre ella. Los triggers que las bloquean
/// (`BEFORE UPDATE ON eventos`) no cuentan.
fn sentencias_prohibidas_sobre_eventos(codigo: &str) -> Vec<String> {
    let normalizado = codigo
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase();
    let mut halladas = Vec::new();

    for palabra in ["UPDATE", "DELETE", "REPLACE", "DROP", "ALTER", "TRUNCATE"] {
        for (i, _) in normalizado.match_indices(palabra) {
            let antes = normalizado[..i].chars().next_back();
            let despues = normalizado[i + palabra.len()..].chars().next();
            let pegada_a_otro_nombre = antes.is_some_and(|c| c.is_alphanumeric() || c == '_')
                || despues.is_some_and(|c| c.is_alphanumeric() || c == '_');
            if pegada_a_otro_nombre {
                continue;
            }

            // Ventana: lo que sigue a la palabra hasta el fin de la sentencia.
            let mut ventana = &normalizado[i + palabra.len()..];
            for corte in [";", " SET ", " WHERE ", " BEGIN ", " VALUES ", " SELECT "] {
                if let Some(p) = ventana.find(corte) {
                    ventana = &ventana[..p];
                }
            }
            let limite = ventana
                .char_indices()
                .nth(80)
                .map_or(ventana.len(), |(p, _)| p);
            let ventana = &ventana[..limite];

            // `BEFORE UPDATE ON eventos`: es el trigger que protege la tabla.
            if ventana.trim_start().starts_with("ON ") {
                continue;
            }
            let nombra_eventos = ventana
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|token| token == "EVENTOS");
            if nombra_eventos {
                halladas.push(format!("{palabra}{}", ventana.trim_end()));
            }
        }
    }
    halladas
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

// ---------- INV-03: sin coma flotante en dominio ni en aplicación (D-05) ----------

/// Falla si algún archivo de `src` del crate usa coma flotante: tipos `f32`/`f64`,
/// conversiones y literales como `1.5`. Ignora comentarios y textos, permite
/// `dec!(...)` y conserva los números de línea.
fn comprobar_sin_flotantes(nombre: &str) {
    let mut infracciones = Vec::new();
    for ruta in lista_rs(&dir_crate(nombre).join("src")) {
        let limpio = limpiar_codigo(&leer(&ruta), true, false);
        for (n, linea) in limpio.lines().enumerate() {
            if let Some(causa) = flotante_en(linea) {
                infracciones.push(format!("{}:{}: usa {causa}", ruta.display(), n + 1));
            }
        }
        for (linea, literal) in literales_flotantes(&limpio) {
            infracciones.push(format!(
                "{}:{linea}: literal decimal `{literal}` (usa `dec!` o Decimal::new)",
                ruta.display()
            ));
        }
    }
    assert!(
        infracciones.is_empty(),
        "El crate `{nombre}` no puede usar coma flotante (INV-03):\n{}",
        infracciones.join("\n")
    );
}

#[test]
fn dominio_no_usa_coma_flotante() {
    comprobar_sin_flotantes("dominio");
}

#[test]
fn aplicacion_no_usa_coma_flotante() {
    comprobar_sin_flotantes("aplicacion");
}

// ---------- INV-16 / RNF-01: sin red en cuentas, movimientos y DTO (D-06) ----------

/// Solo el comando del ping puede hablar con el servidor.
const ARCHIVO_CON_RED: &str = "ping.rs";

#[test]
fn comandos_y_dto_no_usan_la_red() {
    let src = raiz().join("src");
    assert!(
        src.join(ARCHIVO_CON_RED).exists(),
        "{ARCHIVO_CON_RED} ya no existe: actualiza esta prueba"
    );

    let archivos: Vec<PathBuf> = lista_rs(&src)
        .into_iter()
        .filter(|r| r.file_name().is_some_and(|n| n != ARCHIVO_CON_RED))
        .collect();

    // Que la prueba no pase en vacío si alguien renombra los archivos.
    for esperado in ["cuentas.rs", "dto.rs"] {
        assert!(
            archivos.iter().any(|r| r.ends_with(esperado)),
            "No se encontró {esperado} en src/: actualiza esta prueba"
        );
    }

    let mut infracciones = Vec::new();
    for ruta in archivos {
        // Los textos se conservan: una URL entre comillas también es red.
        let limpio = limpiar_codigo(&leer(&ruta), false, false);
        for nombre in red_en(&limpio) {
            infracciones.push(format!("{}: menciona `{nombre}`", ruta.display()));
        }
    }
    assert!(
        infracciones.is_empty(),
        "Solo {ARCHIVO_CON_RED} puede usar la red (INV-16, RNF-01):\n{}",
        infracciones.join("\n")
    );
}

// ---------- INV-04: sin UPDATE ni DELETE sobre la tabla eventos (M-04) ----------

#[test]
fn el_codigo_no_modifica_ni_borra_eventos() {
    let carpetas = [
        dir_crate("dominio").join("src"),
        dir_crate("aplicacion").join("src"),
        dir_crate("infraestructura").join("src"),
        raiz().join("src"),
    ];

    let mut infracciones = Vec::new();
    let mut revisados = 0;
    for carpeta in carpetas {
        for ruta in lista_rs(&carpeta) {
            // Los textos se conservan (el SQL vive dentro de cadenas) y se omiten
            // los bloques `#[cfg(test)]`, que sí pueden probar los triggers.
            let limpio = limpiar_codigo(&leer(&ruta), false, true);
            revisados += 1;
            for sentencia in sentencias_prohibidas_sobre_eventos(&limpio) {
                infracciones.push(format!("{}: `{sentencia}`", ruta.display()));
            }
        }
    }
    assert!(revisados > 0, "No se revisó ningún archivo");
    assert!(
        infracciones.is_empty(),
        "El libro de eventos solo admite inserciones (INV-04):\n{}",
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

#[test]
fn detecta_conversiones_desde_y_hacia_flotantes() {
    assert!(flotante_en("let d = Decimal::from_f64(x);").is_some());
    assert!(flotante_en("let v = d.to_f64();").is_some());
    assert!(flotante_en("let d = Decimal::from_f32_retain(x);").is_some());
    assert!(flotante_en("let d = Decimal::new(1050, 2);").is_none());
}

#[test]
fn el_limpiador_quita_comentarios_de_linea_y_de_bloque() {
    let codigo = "let a = 1; // f64\n/* f32 /* anidado f64 */ f32 */ let b = 2;";
    let limpio = limpiar_codigo(codigo, true, false);
    assert!(flotante_en(&limpio).is_none(), "quedó: {limpio}");
    assert!(limpio.contains("let a = 1;"));
    assert!(limpio.contains("let b = 2;"));
}

#[test]
fn el_limpiador_quita_o_conserva_los_textos_segun_se_pida() {
    let codigo = "let m = \"usa f64 aqui\";";
    assert!(flotante_en(&limpiar_codigo(codigo, true, false)).is_none());
    assert!(flotante_en(&limpiar_codigo(codigo, false, false)).is_some());
}

#[test]
fn el_limpiador_entiende_las_cadenas_crudas() {
    let codigo = "let j = r#\"{\"x\":\"f64\"}\"#; let y = 1;";
    let limpio = limpiar_codigo(codigo, true, false);
    assert!(flotante_en(&limpio).is_none(), "quedó: {limpio}");
    assert!(limpio.contains("let y = 1;"));
}

#[test]
fn el_limpiador_no_confunde_caracteres_ni_tiempos_de_vida() {
    // Una comilla dentro de un carácter no abre una cadena.
    let con_comilla = "let c = '\"'; let x: f64 = 1.0;";
    assert!(flotante_en(&limpiar_codigo(con_comilla, true, false)).is_some());
    // Un tiempo de vida no abre un carácter.
    let con_vida = "fn f<'a>(x: &'a str) -> f64 { 0.0 }";
    assert!(flotante_en(&limpiar_codigo(con_vida, true, false)).is_some());
    // Un escapado dentro de una cadena no la cierra antes de tiempo.
    let con_escape = "let t = \"dijo \\\"f64\\\"\"; let u = 1;";
    let limpio = limpiar_codigo(con_escape, true, false);
    assert!(flotante_en(&limpio).is_none(), "quedó: {limpio}");
    assert!(limpio.contains("let u = 1;"));
}

#[test]
fn el_limpiador_conserva_los_numeros_de_linea() {
    let codigo = "a\n/* x\ny */\nb\n\"p\nq\"\nc";
    let limpio = limpiar_codigo(codigo, true, false);
    assert_eq!(limpio.lines().count(), codigo.lines().count());
}

#[test]
fn detecta_nombres_de_red() {
    assert_eq!(red_en("use reqwest::Client;"), vec!["reqwest"]);
    assert!(!red_en("let url = \"http://localhost:8000\";").is_empty());
    assert!(!red_en("let s = std::net::TcpStream::connect(a);").is_empty());
    assert!(red_en("let n = Cuenta::crear(id, nombre);").is_empty());
}

#[test]
fn detecta_update_y_delete_sobre_eventos() {
    for sql in [
        "conn.execute(\"UPDATE eventos SET tipo = 'x'\", [])",
        "conn.execute(\"delete from eventos where id = 1\", [])",
        "conn.execute(\"DELETE FROM main.eventos\", [])",
        "conn.execute(\"INSERT OR REPLACE INTO eventos (id) VALUES (1)\", [])",
        "conn.execute(\"DROP TABLE IF EXISTS eventos;\", [])",
        "conn.execute(\n    \"UPDATE\n       eventos\n     SET tipo = 'x'\",\n    [])",
    ] {
        assert!(
            !sentencias_prohibidas_sobre_eventos(sql).is_empty(),
            "debía detectarse: {sql}"
        );
    }
}

#[test]
fn permite_los_triggers_y_las_proyecciones() {
    for sql in [
        "CREATE TRIGGER IF NOT EXISTS eventos_no_update BEFORE UPDATE ON eventos BEGIN SELECT RAISE(ABORT, 'eventos es inmutable'); END;",
        "CREATE TRIGGER IF NOT EXISTS eventos_no_delete BEFORE DELETE ON eventos BEGIN SELECT RAISE(ABORT, 'eventos es inmutable'); END;",
        "conn.execute(\"DELETE FROM cuentas\", [])",
        "conn.execute(\"UPDATE cuentas SET saldo_actual = ?1 WHERE id = ?2\", [])",
        "conn.execute(\"INSERT INTO eventos (id, tipo) VALUES (?1, ?2)\", [])",
        "conn.query_row(\"SELECT COUNT(*) FROM eventos WHERE id = ?1\", [])",
        "let actualizado_en = evento.momento_registro;",
    ] {
        assert!(
            sentencias_prohibidas_sobre_eventos(sql).is_empty(),
            "no debía detectarse: {sql}"
        );
    }
}

#[test]
fn detecta_literales_de_coma_flotante() {
    for codigo in [
        "let x = 1.5;",
        "let x = 2e10;",
        "let x = 1_000.25;",
        "let x = 3.0f64;",
        "let x = 1e-5;",
        "foo(1.5, dec!(2.5))",
    ] {
        assert!(
            !literales_flotantes(codigo).is_empty(),
            "debía detectarse: {codigo}"
        );
    }
}

#[test]
fn no_confunde_otros_numeros_con_literales_flotantes() {
    for codigo in [
        "let t = a.0.1;",
        "let r = 0..5;",
        "let r = 1..=5;",
        "let h = 0x1e5;",
        "let n = 10;",
        "let z = 1.max(2);",
        "let d = dec!(10.50);",
        "let d = rust_decimal_macros::dec!(1250.50);",
        "let d = dec!(\n  3.5\n);",
    ] {
        assert!(
            literales_flotantes(codigo).is_empty(),
            "no debía detectarse: {codigo}"
        );
    }
}

#[test]
fn los_literales_flotantes_informan_su_linea() {
    let hallazgos = literales_flotantes("a\nb\nlet x = 1.5;");
    assert_eq!(hallazgos, vec![(3, "1.5".to_string())]);
}

#[test]
fn el_limpiador_omite_solo_el_bloque_de_pruebas() {
    let fuente = "fn a() {}\n#[cfg(test)]\nmod pruebas {\n    fn t() { let s = \"}{\"; if x { y } }\n    const Q: &str = \"DELETE FROM eventos\";\n}\nconst P: &str = \"UPDATE eventos SET x = 1\";\n";
    let limpio = limpiar_codigo(fuente, false, true);
    assert!(!limpio.contains("pruebas"), "quedó: {limpio}");
    assert!(!limpio.contains("DELETE FROM eventos"), "quedó: {limpio}");
    // Lo que viene DESPUÉS del módulo de pruebas sí se revisa.
    assert!(
        limpio.contains("UPDATE eventos SET x = 1"),
        "quedó: {limpio}"
    );
    assert!(limpio.contains("fn a() {}"));
    assert_eq!(limpio.lines().count(), fuente.lines().count());
}

#[test]
fn el_limpiador_omite_elementos_de_pruebas_sin_cuerpo_o_anidados() {
    let sin_cuerpo = limpiar_codigo("#[cfg(test)]\nuse foo::bar;\nfn real() {}\n", false, true);
    assert!(sin_cuerpo.contains("fn real"));
    assert!(!sin_cuerpo.contains("foo"));

    let anidado = limpiar_codigo(
        "fn a() {}\n#[cfg(test)]\nmod t { mod interno { fn x() {} } fn y() {} }\nfn b() {}\n",
        false,
        true,
    );
    assert!(anidado.contains("fn a") && anidado.contains("fn b"));
    assert!(!anidado.contains("interno"));
}

#[test]
fn sin_omitir_las_pruebas_se_ve_todo() {
    let fuente = "#[cfg(test)]\nmod p { const Q: &str = \"DELETE FROM eventos\"; }\n";
    assert!(limpiar_codigo(fuente, false, false).contains("DELETE FROM eventos"));
}
