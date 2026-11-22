//! Cambio del libro en caliente (Ruta B, ADR-014).
//!
//! Al arrancar, al iniciar sesión y al cerrarla, la app abre el libro que
//! corresponde a la sesión guardada (ver `libros.rs`) sin reiniciar Tauri.
//!
//! - Con la sesión vencida el libro del usuario sigue abierto: la sesión guardada
//!   conserva su id y es la misma persona.
//! - El libro anterior se suelta ANTES de pedir la ruta nueva, porque la adopción
//!   renombra el archivo y en Windows no se puede renombrar un archivo abierto.
//! - Si el libro nuevo no abre, se reabre el anterior y se devuelve el error.
//! - Los candados se toman siempre en el mismo orden (casos de uso, luego libro).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use aplicacion::{CasosDeUso, DispositivoOrigen, RepositorioEnMemoria};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;
use tauri::Manager;

use crate::EstadoCasosDeUso;
use crate::libros;
use crate::sesion::{Almacen, AlmacenSistema};

/// El libro abierto: su ruta y la base que mantiene la conexión.
type Actual = Option<(PathBuf, BaseDatos)>;

/// Estado de Tauri con el libro abierto. Reemplaza al `BaseDatos` que antes se
/// registraba solo para mantenerlo vivo.
pub struct LibroActual(Mutex<Actual>);

impl LibroActual {
    pub fn nuevo(ruta: PathBuf, base: BaseDatos) -> Self {
        Self(Mutex::new(Some((ruta, base))))
    }
}

/// Casos de uso vacíos y en memoria: solo existen mientras se cambia de libro,
/// con el candado tomado, así que ningún comando llega a verlos.
fn casos_provisionales() -> CasosDeUso {
    struct SinLibro;
    impl DispositivoOrigen for SinLibro {
        fn id_dispositivo(&self) -> String {
            "sin-libro".to_string()
        }
    }
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(SinLibro),
    )
}

/// Abre el libro de `ruta` (lo crea vacío si no existe) y arma sus casos de uso.
fn abrir(ruta: &Path) -> Result<(BaseDatos, CasosDeUso), String> {
    let base = BaseDatos::persistente(ruta).map_err(|e| e.to_string())?;
    let conexion = base.conexion();
    let repositorio = RepositorioEventosSqlite::with_conn(Arc::clone(&conexion));
    let dispositivo =
        DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion)).map_err(|e| e.to_string())?;
    let casos = CasosDeUso::nuevo(
        Box::new(repositorio),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(dispositivo),
    );
    Ok((base, casos))
}

/// Pone en `casos` y `actual` el libro que corresponde a `usuario_id`
/// (`None` = sin sesión). Si ya está abierto, no hace nada.
fn cambiar(
    carpeta: &Path,
    usuario_id: Option<&str>,
    casos: &mut CasosDeUso,
    actual: &mut Actual,
) -> Result<(), String> {
    let destino = libros::ruta_objetivo(carpeta, usuario_id)?;
    if actual.as_ref().is_some_and(|(ruta, _)| *ruta == destino) {
        return Ok(());
    }
    let anterior = actual.as_ref().map(|(ruta, _)| ruta.clone());

    // 1. Soltar el libro anterior (todas sus conexiones).
    *casos = casos_provisionales();
    *actual = None;

    // 2. Decidir la ruta (puede adoptar el libro existente) y abrir el nuevo.
    let resultado = libros::ruta_para_sesion(carpeta, usuario_id)
        .and_then(|ruta| abrir(&ruta).map(|(base, nuevos)| (ruta, base, nuevos)));
    match resultado {
        Ok((ruta, base, nuevos)) => {
            *casos = nuevos;
            *actual = Some((ruta, base));
            Ok(())
        }
        Err(error) => {
            // 3. Si falló, volver al libro anterior para no dejar la app sin libro.
            if let Some(ruta) = anterior
                && let Ok((base, previos)) = abrir(&ruta)
            {
                *casos = previos;
                *actual = Some((ruta, base));
            }
            Err(error)
        }
    }
}

/// Abre el libro que corresponde a la sesión guardada en el dispositivo.
/// Una sesión guardada antes de la Entrega 7 no trae id: se trata como sin sesión.
pub fn alinear_con_sesion(app: &tauri::AppHandle) -> Result<(), String> {
    let carpeta = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let usuario_id = AlmacenSistema::nuevo(&carpeta)
        .leer()
        .map(|s| s.usuario_id)
        .filter(|id| !id.is_empty());

    let estado = app.state::<EstadoCasosDeUso>();
    let libro = app.state::<LibroActual>();
    const OCUPADO: &str = "El libro está ocupado, inténtalo de nuevo.";
    let mut casos = estado.lock().map_err(|_| OCUPADO.to_string())?;
    let mut actual = libro.0.lock().map_err(|_| OCUPADO.to_string())?;
    cambiar(&carpeta, usuario_id.as_deref(), &mut casos, &mut actual)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::fs;

    fn carpeta_temporal(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("cc_cambio_{}_{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    /// Estado inicial como el de `setup`: el libro sin sesión abierto.
    fn arrancar(c: &Path) -> (CasosDeUso, Actual) {
        let ruta = libros::ruta_sin_sesion(c);
        let (base, casos) = abrir(&ruta).unwrap();
        (casos, Some((ruta, base)))
    }

    fn ruta_abierta(actual: &Actual) -> PathBuf {
        actual
            .as_ref()
            .expect("debía haber un libro abierto")
            .0
            .clone()
    }

    #[test]
    fn iniciar_sesion_adopta_el_libro_y_cambia_de_archivo() {
        let c = carpeta_temporal("adopta");
        let (mut casos, mut actual) = arrancar(&c);
        cambiar(&c, Some("u-1"), &mut casos, &mut actual).unwrap();
        assert_eq!(ruta_abierta(&actual), c.join("cuentas_claras_u-1.db"));
        assert!(c.join("cuentas_claras_u-1.db").exists());
        assert!(
            !c.join("cuentas_claras.db").exists(),
            "el libro sin sesión se adoptó"
        );
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn el_mismo_destino_no_cambia_nada() {
        let c = carpeta_temporal("igual");
        let (mut casos, mut actual) = arrancar(&c);
        cambiar(&c, Some("u-1"), &mut casos, &mut actual).unwrap();
        cambiar(&c, Some("u-1"), &mut casos, &mut actual).unwrap();
        assert_eq!(ruta_abierta(&actual), c.join("cuentas_claras_u-1.db"));
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn cerrar_sesion_abre_un_libro_sin_sesion_y_otro_usuario_va_a_su_archivo() {
        let c = carpeta_temporal("cerrar");
        let (mut casos, mut actual) = arrancar(&c);
        cambiar(&c, Some("u-1"), &mut casos, &mut actual).unwrap();
        cambiar(&c, None, &mut casos, &mut actual).unwrap();
        assert_eq!(ruta_abierta(&actual), c.join("cuentas_claras.db"));
        cambiar(&c, Some("u-2"), &mut casos, &mut actual).unwrap();
        assert_eq!(ruta_abierta(&actual), c.join("cuentas_claras_u-2.db"));
        assert!(
            c.join("cuentas_claras_u-1.db").exists(),
            "el libro de u-1 sigue intacto"
        );
        let _ = fs::remove_dir_all(&c);
    }

    #[test]
    fn un_id_invalido_falla_y_conserva_el_libro_abierto() {
        let c = carpeta_temporal("invalido");
        let (mut casos, mut actual) = arrancar(&c);
        assert!(cambiar(&c, Some("../x"), &mut casos, &mut actual).is_err());
        assert_eq!(ruta_abierta(&actual), c.join("cuentas_claras.db"));
        let _ = fs::remove_dir_all(&c);
    }
}
