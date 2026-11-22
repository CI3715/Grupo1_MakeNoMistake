//! Gestor de libros personales (Ruta B, ADR-014): un archivo SQLite por usuario.
//!
//! - Sin sesión se usa `cuentas_claras.db`.
//! - Con sesión se usa `cuentas_claras_<id>.db`, con el id del usuario y no su
//!   nombre de usuario, que podría escribirse con caracteres raros.
//! - El primer usuario que inicia sesión donde aún no hay libros de usuario adopta
//!   el `cuentas_claras.db` que ya existía, para no perder lo hecho en las
//!   Entregas 2 a 4. Los demás empiezan con un libro vacío.
//!
//! Sin red y sin tocar el contenido del libro: solo decide qué archivo se abre.
//! La adopción renombra el archivo, así que hay que llamar a `ruta_para_sesion`
//! ANTES de abrir el libro nuevo y con el libro anterior ya cerrado.

use std::fs;
use std::path::{Path, PathBuf};

const SIN_SESION: &str = "cuentas_claras.db";
const PREFIJO: &str = "cuentas_claras_";
const SUFIJO: &str = ".db";

/// Solo letras, dígitos y guiones (el id del servidor es un UUID): nada que pueda
/// salirse de la carpeta de datos.
fn id_seguro(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn ruta_sin_sesion(carpeta: &Path) -> PathBuf {
    carpeta.join(SIN_SESION)
}

pub fn ruta_de_usuario(carpeta: &Path, id: &str) -> Result<PathBuf, String> {
    if !id_seguro(id) {
        return Err("El identificador del usuario no es válido.".to_string());
    }
    Ok(carpeta.join(format!("{PREFIJO}{id}{SUFIJO}")))
}

fn hay_libros_de_usuario(carpeta: &Path) -> bool {
    fs::read_dir(carpeta)
        .map(|it| {
            it.flatten().any(|e| {
                let nombre = e.file_name();
                let nombre = nombre.to_string_lossy();
                nombre.starts_with(PREFIJO) && nombre.ends_with(SUFIJO)
            })
        })
        .unwrap_or(false)
}

fn adoptar(origen: &Path, destino: &Path) -> Result<(), String> {
    let error = |e: std::io::Error| format!("No se pudo adoptar el libro existente: {e}");
    fs::rename(origen, destino).map_err(error)?;
    // Si SQLite dejó archivos auxiliares, viajan con el principal.
    for extra in ["-wal", "-shm"] {
        let o = PathBuf::from(format!("{}{extra}", origen.display()));
        if o.exists() {
            let d = PathBuf::from(format!("{}{extra}", destino.display()));
            fs::rename(&o, &d).map_err(error)?;
        }
    }
    Ok(())
}

/// Ruta que corresponde a la sesión, sin adoptar ni crear nada.
pub fn ruta_objetivo(carpeta: &Path, usuario_id: Option<&str>) -> Result<PathBuf, String> {
    match usuario_id {
        None => Ok(ruta_sin_sesion(carpeta)),
        Some(id) => ruta_de_usuario(carpeta, id),
    }
}

/// Ruta del libro que corresponde a la sesión (`None` = sin sesión).
pub fn ruta_para_sesion(carpeta: &Path, usuario_id: Option<&str>) -> Result<PathBuf, String> {
    let Some(id) = usuario_id else {
        return Ok(ruta_sin_sesion(carpeta));
    };
    let destino = ruta_de_usuario(carpeta, id)?;
    let anonimo = ruta_sin_sesion(carpeta);
    if !destino.exists() && anonimo.exists() && !hay_libros_de_usuario(carpeta) {
        adoptar(&anonimo, &destino)?;
    }
    Ok(destino)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta_temporal(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("cc_libros_{}_{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    #[test]
    fn sin_sesion_se_abre_el_libro_de_siempre() {
        let c = carpeta_temporal("sin_sesion");
        assert_eq!(
            ruta_para_sesion(&c, None).unwrap(),
            c.join("cuentas_claras.db")
        );
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn el_primer_usuario_adopta_el_libro_existente() {
        let c = carpeta_temporal("adopta");
        fs::write(c.join("cuentas_claras.db"), "datos de las entregas 2 a 4").unwrap();
        let ruta = ruta_para_sesion(&c, Some("u-1")).unwrap();
        assert_eq!(ruta, c.join("cuentas_claras_u-1.db"));
        assert_eq!(
            fs::read_to_string(&ruta).unwrap(),
            "datos de las entregas 2 a 4"
        );
        assert!(!c.join("cuentas_claras.db").exists());
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn el_segundo_usuario_empieza_vacio_y_no_adopta() {
        let c = carpeta_temporal("segundo");
        fs::write(c.join("cuentas_claras.db"), "de ana").unwrap();
        ruta_para_sesion(&c, Some("u-1")).unwrap();
        fs::write(c.join("cuentas_claras.db"), "sin sesion").unwrap();
        let ruta2 = ruta_para_sesion(&c, Some("u-2")).unwrap();
        assert!(
            !ruta2.exists(),
            "el libro de u-2 lo crea la base al abrirlo, vacío"
        );
        assert_eq!(
            fs::read_to_string(c.join("cuentas_claras.db")).unwrap(),
            "sin sesion"
        );
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn el_mismo_usuario_vuelve_a_su_libro() {
        let c = carpeta_temporal("mismo");
        fs::write(c.join("cuentas_claras.db"), "de ana").unwrap();
        let a = ruta_para_sesion(&c, Some("u-1")).unwrap();
        let b = ruta_para_sesion(&c, Some("u-1")).unwrap();
        assert_eq!(a, b);
        assert_eq!(fs::read_to_string(&b).unwrap(), "de ana");
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn un_id_que_se_sale_de_la_carpeta_se_rechaza() {
        let c = carpeta_temporal("inseguro");
        for malo in ["", "../x", "a/b", "a b", "..", "x.db"] {
            assert!(
                ruta_para_sesion(&c, Some(malo)).is_err(),
                "debía rechazar {malo:?}"
            );
        }
        let _ = fs::remove_dir_all(&c);
    }
}
