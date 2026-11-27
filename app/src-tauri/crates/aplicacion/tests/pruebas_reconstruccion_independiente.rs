//! Reconstrucción de saldos con una herramienta independiente de la aplicación
//! (Entrega 8: John - Tarea 4; RNF-04).
//!
//! `herramientas/reconstruir_saldos.py` lee una exportación sin cifrar solo con la
//! biblioteca estándar de Python y siguiendo `docs/formato-exportacion.md`. Estas pruebas
//! lo ejecutan sobre exportaciones reales de la aplicación y comparan sus saldos con los
//! que calcula el núcleo. También comprueban que el ejemplo de la documentación se puede
//! leer y que sus saldos son los que la documentación dice.
//!
//! Necesitan Python 3.9 o posterior. Se busca en `CUENTAS_CLARAS_PYTHON`, luego `python3`
//! y luego `python`.

mod comun;

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use aplicacion::{CasosDeUso, texto_a_libro};
use comun::{CifradorFalso, libro_completo, libro_vacio, saldos_de_la_app, totales_de_la_app};
use serde_json::Value;

fn raiz_del_repositorio() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("no se encontró la raíz del repositorio")
}

fn script() -> PathBuf {
    raiz_del_repositorio().join("herramientas/reconstruir_saldos.py")
}

/// Un intérprete de Python 3 que funcione.
fn python() -> String {
    let candidatos: Vec<String> = std::env::var("CUENTAS_CLARAS_PYTHON")
        .into_iter()
        .chain(["python3".to_string(), "python".to_string()])
        .collect();
    for candidato in &candidatos {
        let salida = Command::new(candidato)
            .args(["-c", "import sys; assert sys.version_info >= (3, 9)"])
            .output();
        if salida.is_ok_and(|s| s.status.success()) {
            return candidato.clone();
        }
    }
    panic!(
        "Estas pruebas necesitan Python 3.9 o posterior (probé {candidatos:?}). \
         Instálalo o indica el intérprete en CUENTAS_CLARAS_PYTHON."
    );
}

/// Ejecuta el script con `entrada` por la entrada estándar. Devuelve (código, salida, errores).
fn ejecutar(entrada: &[u8], argumentos: &[&str]) -> (i32, String, String) {
    let mut hijo = Command::new(python())
        .arg("-I") // aislado: no carga nada del directorio actual
        .arg(script())
        .arg("-")
        .args(argumentos)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("no se pudo ejecutar el script");
    hijo.stdin.take().unwrap().write_all(entrada).unwrap();
    let salida = hijo.wait_with_output().unwrap();
    (
        salida.status.code().unwrap_or(-1),
        String::from_utf8(salida.stdout).expect("la salida debe ser UTF-8"),
        String::from_utf8_lossy(&salida.stderr).into_owned(),
    )
}

/// Saldos y totales que reconstruye el script, en el mismo orden que los de la app.
#[allow(clippy::type_complexity)]
fn reconstruir_con_python(
    exportacion: &[u8],
) -> (Vec<(String, String, String, String)>, Vec<(String, String)>) {
    let (codigo, salida, errores) = ejecutar(exportacion, &["--json"]);
    assert_eq!(codigo, 0, "el script falló: {errores}");
    let json: Value = serde_json::from_str(&salida).expect("el script debe imprimir JSON");
    let texto = |v: &Value, clave: &str| v[clave].as_str().unwrap().to_string();
    let mut cuentas: Vec<_> = json["cuentas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                texto(c, "id"),
                texto(c, "nombre"),
                texto(c, "moneda"),
                texto(c, "saldo"),
            )
        })
        .collect();
    cuentas.sort();
    let mut totales: Vec<_> = json["totales"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (texto(t, "moneda"), texto(t, "total")))
        .collect();
    totales.sort();
    (cuentas, totales)
}

fn comparar(app: &CasosDeUso) {
    let exportacion = app.exportar_copia(None, &CifradorFalso).unwrap();
    let (cuentas, totales) = reconstruir_con_python(&exportacion);
    assert_eq!(cuentas, saldos_de_la_app(app), "saldos por cuenta");
    assert_eq!(totales, totales_de_la_app(app), "totales por moneda");
}

#[test]
fn los_saldos_del_script_coinciden_con_los_de_la_app() {
    let app = libro_completo();
    // Que la comparación no pase en vacío: hay cuentas en cinco monedas y saldos movidos.
    let saldos = saldos_de_la_app(&app);
    assert_eq!(saldos.len(), 5);
    assert!(saldos.iter().any(|(_, _, moneda, _)| moneda == "BTC"));
    comparar(&app);
}

#[test]
fn los_saldos_coinciden_tambien_tras_importar_en_otro_dispositivo() {
    // La importación ordena los eventos con la fusión: el script debe dar lo mismo.
    let origen = libro_completo();
    let mut destino = libro_vacio("otra-instalacion");
    destino
        .importar_copia(
            &origen.exportar_copia(None, &CifradorFalso).unwrap(),
            None,
            &CifradorFalso,
        )
        .unwrap();
    comparar(&destino);
}

#[test]
fn un_libro_vacio_no_tiene_saldos() {
    let (cuentas, totales) = reconstruir_con_python(
        &libro_vacio("vacio")
            .exportar_copia(None, &CifradorFalso)
            .unwrap(),
    );
    assert!(cuentas.is_empty());
    assert!(totales.is_empty());
}

#[test]
fn el_script_rechaza_un_archivo_invalido_o_cifrado() {
    let app = libro_completo();
    let texto = String::from_utf8(app.exportar_copia(None, &CifradorFalso).unwrap()).unwrap();
    let cifrado = app
        .exportar_copia(Some("una frase de paso larga"), &CifradorFalso)
        .unwrap();
    let primera_linea = texto.lines().next().unwrap().to_string();
    let casos: Vec<Vec<u8>> = vec![
        cifrado,
        texto.replace("version=1", "version=2").into_bytes(),
        texto
            .replace(&primera_linea, &format!("{primera_linea}0"))
            .into_bytes(),
        texto.trim_end().as_bytes().to_vec(),
        format!("{texto}{{roto}}\n").into_bytes(),
    ];
    for entrada in casos {
        let (codigo, salida, errores) = ejecutar(&entrada, &["--json"]);
        assert_eq!(codigo, 2, "debía rechazarse; salida: {salida}");
        assert!(salida.is_empty(), "no debe imprimir saldos parciales");
        assert!(errores.contains("Archivo inválido"), "errores: {errores}");
    }
}

#[test]
fn la_tabla_legible_muestra_cada_cuenta_y_los_totales() {
    let app = libro_completo();
    let (codigo, salida, errores) =
        ejecutar(&app.exportar_copia(None, &CifradorFalso).unwrap(), &[]);
    assert_eq!(codigo, 0, "{errores}");
    for (_, nombre, _, saldo) in saldos_de_la_app(&app) {
        assert!(salida.contains(&nombre), "falta {nombre} en:\n{salida}");
        assert!(salida.contains(&saldo), "falta {saldo} en:\n{salida}");
    }
    assert!(salida.contains("Total por moneda"));
}

// ---------- El ejemplo de docs/formato-exportacion.md ----------

/// El bloque de código que sigue a la marca `<!-- ejemplo-completo -->` en la documentación.
fn ejemplo_de_la_documentacion() -> String {
    let doc = std::fs::read_to_string(raiz_del_repositorio().join("docs/formato-exportacion.md"))
        .expect("falta docs/formato-exportacion.md");
    let desde = doc
        .split("<!-- ejemplo-completo -->")
        .nth(1)
        .expect("falta la marca <!-- ejemplo-completo --> en la documentación");
    let bloque = desde
        .split("```text\n")
        .nth(1)
        .and_then(|resto| resto.split("```").next())
        .expect("falta el bloque ```text del ejemplo");
    bloque.to_string()
}

/// Los saldos que la documentación anuncia para el ejemplo: filas `| Nombre | MON | saldo |`
/// después de la marca `<!-- saldos-del-ejemplo -->`.
fn saldos_anunciados() -> Vec<(String, String, String)> {
    let doc = std::fs::read_to_string(raiz_del_repositorio().join("docs/formato-exportacion.md"))
        .unwrap();
    let desde = doc
        .split("<!-- saldos-del-ejemplo -->")
        .nth(1)
        .expect("falta la marca <!-- saldos-del-ejemplo -->");
    let mut filas: Vec<_> = desde
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .skip(2) // encabezado y separador de la tabla
        .map(|l| {
            let celdas: Vec<String> = l
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('`').to_string())
                .collect();
            (celdas[0].clone(), celdas[1].clone(), celdas[2].clone())
        })
        .collect();
    filas.sort();
    filas
}

#[test]
fn el_ejemplo_de_la_documentacion_es_valido_y_sus_saldos_son_los_anunciados() {
    let ejemplo = ejemplo_de_la_documentacion();
    let eventos = texto_a_libro(&ejemplo).expect("el ejemplo de la documentación no es válido");

    let mut app = libro_vacio("lector");
    let informe = app.fusionar_eventos(&eventos).unwrap();
    assert!(informe.rechazados.is_empty(), "{:?}", informe.rechazados);

    let mut de_la_app: Vec<_> = saldos_de_la_app(&app)
        .into_iter()
        .map(|(_, nombre, moneda, saldo)| (nombre, moneda, saldo))
        .collect();
    de_la_app.sort();
    assert_eq!(
        de_la_app,
        saldos_anunciados(),
        "saldos de la app vs. documentación"
    );

    let (cuentas, _) = reconstruir_con_python(ejemplo.as_bytes());
    let mut del_script: Vec<_> = cuentas
        .into_iter()
        .map(|(_, nombre, moneda, saldo)| (nombre, moneda, saldo))
        .collect();
    del_script.sort();
    assert_eq!(del_script, de_la_app, "saldos del script vs. app");
}
