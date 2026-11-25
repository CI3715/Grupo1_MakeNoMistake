//! Exportar el libro como copia de seguridad (Entrega 8: RF-INT-01, RF-INT-05, RF-INT-06).
//!
//! `Cifrador` es el puerto del cifrado con frase de paso: la implementación real
//! (Argon2id + cifrado autenticado) vive en `infraestructura`, porque esta capa no
//! puede usar librerías externas.
//!
//! PROVISIONAL: `libro_a_texto` es el formato de una línea por evento; cuando el
//! formato oficial de `docs/formato-exportacion.md` esté listo, se reemplaza solo esa función.

use serde_json::{Value, json};
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
    #[error("El archivo no tiene el formato de una copia de seguridad de Cuentas Claras.")]
    ArchivoInvalido,
    #[error("Este archivo está cifrado: escribe la frase de paso.")]
    FraseRequerida,
    #[error(
        "El archivo es de la versión {archivo} del formato y esta aplicación soporta la {soportada}."
    )]
    VersionNoSoportada { archivo: u32, soportada: u32 },
    #[error("El cifrado de la copia todavía no está disponible.")]
    CifradoNoDisponible,
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

const MARCA: &str = "# cuentas-claras-respaldo";

fn valor<'a>(encabezado: &'a str, nombre: &str) -> Option<&'a str> {
    encabezado
        .split_whitespace()
        .find_map(|p| p.strip_prefix(nombre)?.strip_prefix('='))
}

fn texto_de(obj: &serde_json::Map<String, Value>, clave: &str) -> Option<String> {
    obj.get(clave)?.as_str().map(str::to_string)
}

fn linea_a_evento(linea: &str) -> Option<EventoRegistrado> {
    let json: Value = serde_json::from_str(linea).ok()?;
    let obj = json.as_object()?;
    let id = texto_de(obj, "id")?;
    if id.is_empty() {
        return None;
    }
    Some(EventoRegistrado {
        id,
        tipo: texto_de(obj, "tipo")?,
        momento_registro: texto_de(obj, "momento_registro")?,
        dispositivo_origen: texto_de(obj, "dispositivo_origen")?,
        datos: texto_de(obj, "datos")?,
    })
}

fn texto_a_libro(texto: &str) -> Result<Vec<EventoRegistrado>, ErrorRespaldo> {
    let invalido = ErrorRespaldo::ArchivoInvalido;
    let mut lineas = texto.lines();
    let encabezado = lineas.next().ok_or(invalido.clone())?;
    let resto = encabezado.strip_prefix(MARCA).ok_or(invalido.clone())?;
    let version: u32 = valor(resto, "version")
        .and_then(|v| v.parse().ok())
        .ok_or(invalido.clone())?;
    if version != VERSION_FORMATO {
        return Err(ErrorRespaldo::VersionNoSoportada {
            archivo: version,
            soportada: VERSION_FORMATO,
        });
    }
    let cantidad: usize = valor(resto, "eventos")
        .and_then(|v| v.parse().ok())
        .ok_or(invalido.clone())?;
    let eventos = lineas
        .map(linea_a_evento)
        .collect::<Option<Vec<_>>>()
        .ok_or(invalido.clone())?;
    if eventos.len() != cantidad {
        return Err(invalido);
    }
    Ok(eventos)
}

/// Lee y valida una copia de seguridad sin tocar el libro: todo o nada. Un archivo en
/// texto legible se lee tal cual; si no, se descifra con `frase`. Si algo falla (frase
/// incorrecta, archivo alterado, versión que no se soporta, líneas mal formadas), no
/// devuelve ningún evento (RF-INT-02).
pub fn leer_libro(
    bytes: &[u8],
    frase: Option<&str>,
    cifrador: &dyn Cifrador,
) -> Result<Vec<EventoRegistrado>, ErrorRespaldo> {
    let texto = if bytes.starts_with(MARCA.as_bytes()) {
        String::from_utf8(bytes.to_vec()).map_err(|_| ErrorRespaldo::ArchivoInvalido)?
    } else {
        let frase = frase.ok_or(ErrorRespaldo::FraseRequerida)?;
        let claro = cifrador.descifrar(frase, bytes)?;
        String::from_utf8(claro).map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)?
    };
    texto_a_libro(&texto)
}
