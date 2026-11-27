//! Adaptador de carpeta compartida del puerto `TransporteSincronizacion`
//! (Entrega 8: Ernesto - Tarea 3; RF-INT-07, RNF-14).
//!
//! El destino es una carpeta que otro servicio mantiene sincronizada entre los
//! dispositivos del usuario (Syncthing, Dropbox, Google Drive). Este adaptador no
//! sincroniza nada por su cuenta: solo lee y escribe archivos con cuidado para que ese
//! servicio nunca genere «copias en conflicto» ni entregue un archivo a medio escribir.
//!
//! # Disposición en disco
//!
//! ```text
//! <carpeta elegida por el usuario>/
//!   <id-del-dispositivo-A>/
//!     20261124T120000Z-0001.lote
//!     20261124T180000Z-0002.lote
//!   <id-del-dispositivo-B>/
//!     20261124T130000Z-0001.lote
//! ```
//!
//! - **Cada dispositivo escribe solo en su propia carpeta**, y cada archivo se escribe
//!   una sola vez: dos dispositivos nunca tocan el mismo archivo.
//! - **Escritura atómica:** el lote se escribe primero con un nombre temporal
//!   (`<lote>.lote.tmp`) y solo cuando está completo y en disco se renombra a
//!   `<lote>.lote`. Quien lee ignora todo lo que no se llame exactamente `<lote>.lote`
//!   con un nombre de lote válido: temporales, archivos de otros programas y copias en
//!   conflicto (`lote (copia en conflicto).lote`).
//! - **Inmutables:** un lote existente nunca se sobrescribe.
//! - La carpeta elegida debe existir: si no existe (disco desmontado, carpeta
//!   borrada), el destino está inaccesible y no se crea en silencio.
//! - El contenido ya llega cifrado desde el caso de uso (RF-INT-06): este adaptador
//!   solo mueve bytes.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use aplicacion::transporte::{ErrorTransporte, TransporteSincronizacion, validar_nombre};

/// Extensión de un lote terminado.
const EXTENSION_LOTE: &str = "lote";

/// Sufijo del archivo mientras se escribe.
const SUFIJO_TEMPORAL: &str = ".tmp";

/// Transporte de un dispositivo sobre una carpeta compartida.
#[derive(Debug, Clone)]
pub struct TransporteCarpeta {
    raiz: PathBuf,
    dispositivo: String,
}

impl TransporteCarpeta {
    /// Crea el transporte de `dispositivo` sobre la carpeta `raiz`. No toca el disco:
    /// la carpeta se comprueba en cada operación, porque puede desaparecer.
    pub fn nuevo(
        raiz: impl Into<PathBuf>,
        dispositivo: &str,
    ) -> Result<TransporteCarpeta, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        Ok(TransporteCarpeta {
            raiz: raiz.into(),
            dispositivo: dispositivo.to_string(),
        })
    }

    /// La carpeta del destino debe existir.
    fn exigir_raiz(&self) -> Result<(), ErrorTransporte> {
        if self.raiz.is_dir() {
            Ok(())
        } else {
            Err(ErrorTransporte::DestinoInaccesible)
        }
    }

    fn carpeta_de(&self, dispositivo: &str) -> PathBuf {
        self.raiz.join(dispositivo)
    }

    /// Lotes terminados de la carpeta de un dispositivo, en orden de texto. Ignora lo
    /// que no sea exactamente `<lote válido>.lote`.
    fn lotes_de(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        let lectura = match fs::read_dir(self.carpeta_de(dispositivo)) {
            Ok(lectura) => lectura,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(error_de_io(&e)),
        };
        let mut lotes = Vec::new();
        for entrada in lectura {
            let ruta = entrada.map_err(|e| error_de_io(&e))?.path();
            if ruta.extension().and_then(|x| x.to_str()) != Some(EXTENSION_LOTE) {
                continue;
            }
            if !ruta.is_file() {
                continue;
            }
            let Some(lote) = ruta.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            if validar_nombre(lote).is_ok() {
                lotes.push(lote.to_string());
            }
        }
        lotes.sort();
        Ok(lotes)
    }
}

/// Traduce un error del sistema de archivos. Una carpeta que desaparece o que no se
/// puede leer es un destino inaccesible (reintentable); lo demás es inesperado.
fn error_de_io(e: &std::io::Error) -> ErrorTransporte {
    match e.kind() {
        ErrorKind::NotFound | ErrorKind::PermissionDenied => ErrorTransporte::DestinoInaccesible,
        _ => ErrorTransporte::Interno(e.to_string()),
    }
}

/// Escribe todo el contenido y lo fuerza a disco antes de devolver.
fn escribir_archivo(ruta: &Path, contenido: &[u8]) -> std::io::Result<()> {
    let mut archivo = fs::File::create(ruta)?;
    archivo.write_all(contenido)?;
    archivo.sync_all()
}

impl TransporteSincronizacion for TransporteCarpeta {
    fn dispositivo_local(&self) -> &str {
        &self.dispositivo
    }

    /// En una carpeta no hay nada que traer: el servicio de sincronización ya dejó
    /// los archivos ahí. Solo se comprueba que la carpeta sea accesible.
    fn traer(&mut self) -> Result<(), ErrorTransporte> {
        self.exigir_raiz()
    }

    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte> {
        self.exigir_raiz()?;
        let lectura = fs::read_dir(&self.raiz).map_err(|e| error_de_io(&e))?;
        let mut dispositivos = Vec::new();
        for entrada in lectura {
            let entrada = entrada.map_err(|e| error_de_io(&e))?;
            let Some(nombre) = entrada.file_name().to_str().map(str::to_string) else {
                continue;
            };
            // Los nombres con punto (`.git`, `.stfolder`, ...) y los archivos sueltos
            // no son carpetas de dispositivos.
            if validar_nombre(&nombre).is_err() || !entrada.path().is_dir() {
                continue;
            }
            if !self.lotes_de(&nombre)?.is_empty() {
                dispositivos.push(nombre);
            }
        }
        dispositivos.sort();
        Ok(dispositivos)
    }

    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        self.exigir_raiz()?;
        self.lotes_de(dispositivo)
    }

    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        validar_nombre(lote)?;
        self.exigir_raiz()?;
        let ruta = self
            .carpeta_de(dispositivo)
            .join(format!("{lote}.{EXTENSION_LOTE}"));
        match fs::read(&ruta) {
            Ok(contenido) => Ok(contenido),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(ErrorTransporte::LoteNoExiste {
                dispositivo: dispositivo.to_string(),
                lote: lote.to_string(),
            }),
            Err(e) => Err(error_de_io(&e)),
        }
    }

    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte> {
        validar_nombre(lote)?;
        self.exigir_raiz()?;
        let carpeta = self.carpeta_de(&self.dispositivo);
        fs::create_dir_all(&carpeta).map_err(|e| error_de_io(&e))?;

        let definitivo = carpeta.join(format!("{lote}.{EXTENSION_LOTE}"));
        if definitivo.exists() {
            return Err(ErrorTransporte::LoteYaExiste {
                dispositivo: self.dispositivo.clone(),
                lote: lote.to_string(),
            });
        }

        // Primero con nombre temporal; el renombrado final es lo que hace visible el lote.
        let temporal = carpeta.join(format!("{lote}.{EXTENSION_LOTE}{SUFIJO_TEMPORAL}"));
        if let Err(e) = escribir_archivo(&temporal, contenido) {
            let _ = fs::remove_file(&temporal);
            return Err(error_de_io(&e));
        }
        if let Err(e) = fs::rename(&temporal, &definitivo) {
            let _ = fs::remove_file(&temporal);
            return Err(error_de_io(&e));
        }
        Ok(())
    }

    /// En una carpeta no hay paso de publicación: el lote ya es visible al renombrarse
    /// y el servicio de sincronización lo reparte. Solo se comprueba que la carpeta
    /// siga accesible, para que quien sincroniza no dé por enviado algo que no lo está.
    fn publicar(&mut self) -> Result<(), ErrorTransporte> {
        self.exigir_raiz()
    }
}
