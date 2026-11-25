//! Exportar el libro como copia de seguridad (Entrega 8: RF-INT-01, RF-INT-05, RF-INT-06).
//!
//! `Cifrador` es el puerto del cifrado con frase de paso: la implementación real
//! (Argon2id + cifrado autenticado) vive en `infraestructura`, porque esta capa no
//! puede usar librerías externas.
//!
//! PROVISIONAL: `libro_a_texto` es el formato de una línea por evento; cuando el
//! formato oficial de `docs/formato-exportacion.md` esté listo, se reemplaza solo esa función.

use serde_json::json;
use thiserror::Error;

use crate::puertos::EventoRegistrado;

/// Largo mínimo de la frase de paso, en caracteres.
pub const LARGO_MINIMO_FRASE: usize = 12;

/// Versión del formato del texto exportado.
pub const VERSION_FORMATO: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorRespaldo {
    #[error("La frase de paso debe tener al menos 12 caracteres.")]
    FraseCorta,
    #[error("Frase de paso incorrecta o archivo dañado.")]
    FraseIncorrectaOArchivoDanado,
    #[error("No se pudo leer el libro del dispositivo. Intenta de nuevo.")]
    LibroNoLegible,
}

/// Puerto del cifrado con frase de paso. Una frase incorrecta y un archivo alterado
/// deben dar el mismo error: `FraseIncorrectaOArchivoDanado`.
pub trait Cifrador {
    fn cifrar(&self, frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo>;
    fn descifrar(&self, frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo>;
}

/// Texto legible del libro: un encabezado y un evento por línea. No incluye
/// credenciales, tokens ni la frase de paso (RF-INT-05).
pub fn libro_a_texto(eventos: &[EventoRegistrado], exportado: &str) -> String {
    let mut salida = format!(
        "# cuentas-claras-respaldo version={} exportado={} eventos={}\n",
        VERSION_FORMATO,
        exportado,
        eventos.len()
    );
    for e in eventos {
        let linea = json!({
            "id": e.id,
            "tipo": e.tipo,
            "momento_registro": e.momento_registro,
            "dispositivo_origen": e.dispositivo_origen,
            "datos": e.datos,
        });
        salida.push_str(&linea.to_string());
        salida.push('\n');
    }
    salida
}

/// Arma el archivo de respaldo: texto plano si `frase` es `None`, cifrado si lleva frase.
/// No toca el libro: solo recibe los eventos ya leídos.
pub fn exportar_libro(
    eventos: &[EventoRegistrado],
    exportado: &str,
    frase: Option<&str>,
    cifrador: &dyn Cifrador,
) -> Result<Vec<u8>, ErrorRespaldo> {
    let texto = libro_a_texto(eventos, exportado);
    match frase {
        None => Ok(texto.into_bytes()),
        Some(f) => {
            if f.chars().count() < LARGO_MINIMO_FRASE {
                return Err(ErrorRespaldo::FraseCorta);
            }
            cifrador.cifrar(f, texto.as_bytes())
        }
    }
}
