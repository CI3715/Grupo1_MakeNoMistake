//! Comando de exportar la copia de seguridad (Entrega 8, RF-INT-01).
//!
//! Aquí no hay reglas de negocio: el caso de uso arma los bytes y este archivo solo
//! los escribe en la ruta que la persona eligió. Sin red: nada del libro sale del
//! dispositivo (RNF-01, INV-16).

use std::fs;
use std::path::{Path, PathBuf};

use aplicacion::{CasosDeUso, Cifrador, ErrorRespaldo};
use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::ErroresDto;
use crate::sesion::sin_campo;

/// PROVISIONAL: se reemplaza por el cifrador real (Argon2id + cifrado autenticado)
/// cuando esté en `infraestructura`.
struct CifradoPendiente;

impl Cifrador for CifradoPendiente {
    fn cifrar(&self, _frase: &str, _claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Err(ErrorRespaldo::CifradoNoDisponible)
    }
    fn descifrar(&self, _frase: &str, _archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Err(ErrorRespaldo::CifradoNoDisponible)
    }
}

/// Escribe la copia en `ruta`. Primero va a un archivo temporal y luego se renombra,
/// para no dejar nunca un respaldo a medio escribir.
pub fn exportar_a_archivo(
    casos: &CasosDeUso,
    ruta: &Path,
    frase: Option<&str>,
    cifrador: &dyn Cifrador,
) -> Result<(), ErroresDto> {
    if !ruta.is_absolute() || ruta.file_name().is_none() {
        return Err(sin_campo(
            "La ubicación elegida para el archivo no es válida.",
        ));
    }
    if !ruta.parent().is_some_and(Path::is_dir) {
        return Err(sin_campo("La carpeta elegida no existe."));
    }
    let bytes = casos
        .exportar_copia(frase, cifrador)
        .map_err(|e| sin_campo(&e.to_string()))?;

    let mut nombre_temporal = ruta.as_os_str().to_owned();
    nombre_temporal.push(".tmp");
    let temporal = PathBuf::from(nombre_temporal);
    let no_guardado = || sin_campo("No se pudo guardar el archivo en la ubicación elegida.");

    if fs::write(&temporal, &bytes).is_err() {
        let _ = fs::remove_file(&temporal);
        return Err(no_guardado());
    }
    if fs::rename(&temporal, ruta).is_err() {
        let _ = fs::remove_file(&temporal);
        return Err(no_guardado());
    }
    Ok(())
}

/// Exporta el libro a `ruta`. Sin `frase` el archivo queda en texto legible; con
/// `frase` va cifrado. Se invoca con `invoke('exportar_copia', { ruta, frase })`.
#[tauri::command(rename_all = "snake_case")]
pub fn exportar_copia(
    estado: State<'_, EstadoCasosDeUso>,
    ruta: String,
    frase: Option<String>,
) -> Result<(), ErroresDto> {
    let casos = estado
        .lock()
        .map_err(|_| sin_campo("Ocurrió un problema interno. Intenta de nuevo."))?;
    exportar_a_archivo(
        &casos,
        Path::new(&ruta),
        frase.as_deref(),
        &CifradoPendiente,
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use aplicacion::{
        DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria,
    };

    const FRASE: &str = "una frase de paso larga";

    struct CifradorFalso;

    impl Cifrador for CifradorFalso {
        fn cifrar(&self, _frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
            Ok(claro.iter().map(|b| b ^ 0x5A).collect())
        }
        fn descifrar(&self, _frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
            Ok(archivo.iter().map(|b| b ^ 0x5A).collect())
        }
    }

    fn casos() -> CasosDeUso {
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
        app
    }

    fn carpeta(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("cc-respaldo-{}-{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    #[test]
    fn sin_cifrar_escribe_texto_legible_y_no_deja_temporales() {
        let dir = carpeta("claro");
        let ruta = dir.join("copia.txt");
        assert!(exportar_a_archivo(&casos(), &ruta, None, &CifradorFalso).is_ok());
        let texto = fs::read_to_string(&ruta).unwrap();
        assert!(texto.contains("eventos=1") && texto.contains("Efectivo"));
        assert!(!dir.join("copia.txt.tmp").exists());
    }

    #[test]
    fn cifrado_no_deja_datos_legibles_en_el_archivo() {
        let dir = carpeta("cifrado");
        let ruta = dir.join("copia.bin");
        assert!(exportar_a_archivo(&casos(), &ruta, Some(FRASE), &CifradorFalso).is_ok());
        let bytes = fs::read(&ruta).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("Efectivo"));
    }

    #[test]
    fn frase_corta_no_crea_archivo() {
        let dir = carpeta("corta");
        let ruta = dir.join("copia.bin");
        assert!(exportar_a_archivo(&casos(), &ruta, Some("corta"), &CifradorFalso).is_err());
        assert!(!ruta.exists());
    }

    #[test]
    fn ruta_relativa_o_carpeta_inexistente_se_rechazan() {
        let dir = carpeta("rutas");
        let relativa = Path::new("copia.txt");
        assert!(exportar_a_archivo(&casos(), relativa, None, &CifradorFalso).is_err());
        let sin_carpeta = dir.join("no-existe").join("copia.txt");
        assert!(exportar_a_archivo(&casos(), &sin_carpeta, None, &CifradorFalso).is_err());
    }

    #[test]
    fn con_el_cifrado_pendiente_falla_sin_dejar_archivos() {
        let dir = carpeta("pendiente");
        let ruta = dir.join("copia.bin");
        assert!(exportar_a_archivo(&casos(), &ruta, Some(FRASE), &CifradoPendiente).is_err());
        assert!(!ruta.exists() && !dir.join("copia.bin.tmp").exists());
    }
}
