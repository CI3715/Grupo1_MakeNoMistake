//! Exportar el libro como copia de seguridad (Entrega 8: RF-INT-01, RF-INT-05, RF-INT-06,
//! RNF-04).
//!
//! `Cifrador` es el puerto del cifrado con frase de paso: la implementación real
//! (Argon2id + cifrado autenticado) vive en `infraestructura`, porque esta capa no
//! puede usar librerías externas.
//!
//! # Formato del texto (versión 1)
//!
//! El formato completo, con cada tipo de evento y un ejemplo, está en
//! `docs/formato-exportacion.md`. En resumen:
//!
//! ```text
//! # cuentas-claras-respaldo version=1 exportado=2026-11-26T10:00:00Z eventos=2
//! {"id":"…","tipo":"CuentaCreada","momento_registro":"…","dispositivo_origen":"…","datos":{…}}
//! {"id":"…","tipo":"MovimientoRegistrado","momento_registro":"…","dispositivo_origen":"…","datos":{…}}
//! ```
//!
//! - La primera línea es el encabezado: versión del formato, instante de la exportación
//!   (UTC) y cantidad de eventos que siguen.
//! - Después, un evento por línea en JSON (UTF-8, `\n` como fin de línea), en el orden del
//!   libro. `datos` es la carga del evento tal como se guardó, insertada como objeto JSON
//!   para que se pueda leer sin desescapar nada.
//! - La lectura es estricta y todo o nada: un encabezado mal formado, una versión que no se
//!   soporta, una cantidad que no coincide, una línea mal formada o un `id` repetido
//!   rechazan el archivo completo.
//! - Ida y vuelta sin pérdidas: `texto_a_libro(libro_a_texto(e))` devuelve exactamente `e`,
//!   byte por byte en cada campo (incluido `datos`).
//! - No lleva credenciales, tokens ni la frase de paso (RF-INT-05): solo eventos del libro.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use serde_json::value::RawValue;
use thiserror::Error;

use crate::puertos::EventoRegistrado;

/// Largo mínimo de la frase de paso, en caracteres.
pub const LARGO_MINIMO_FRASE: usize = 12;

/// Versión del formato del texto exportado.
pub const VERSION_FORMATO: u32 = 1;

/// Comienzo de la primera línea de toda exportación en texto.
pub const MARCA_FORMATO: &str = "# cuentas-claras-respaldo";

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

/// Encabezado de una exportación en texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncabezadoExportacion {
    /// Versión del formato con que se escribió el archivo.
    pub version: u32,
    /// Instante de la exportación, en UTC (`AAAA-MM-DDTHH:MM:SSZ`).
    pub exportado: String,
    /// Cantidad de eventos que siguen al encabezado.
    pub eventos: usize,
}

// ---------- Escritura ----------

/// `true` si `datos` se puede insertar tal cual como objeto JSON en la línea y leerse de
/// vuelta idéntico: un objeto JSON válido, sin espacios alrededor y sin saltos de línea.
fn datos_como_objeto(datos: &str) -> bool {
    datos == datos.trim()
        && !datos.contains(['\n', '\r'])
        && serde_json::from_str::<Value>(datos).is_ok_and(|v| v.is_object())
}

/// Texto JSON de una cadena (con comillas y escapes).
fn cadena_json(texto: &str) -> String {
    Value::String(texto.to_string()).to_string()
}

/// Una línea del archivo con el evento. Las claves van siempre en el mismo orden.
fn evento_a_linea(e: &EventoRegistrado) -> String {
    // Una carga que no es un objeto JSON (no debería ocurrir) viaja como cadena: así la
    // ida y vuelta sigue siendo exacta.
    let datos = if datos_como_objeto(&e.datos) {
        e.datos.clone()
    } else {
        cadena_json(&e.datos)
    };
    format!(
        "{{\"id\":{},\"tipo\":{},\"momento_registro\":{},\"dispositivo_origen\":{},\"datos\":{}}}",
        cadena_json(&e.id),
        cadena_json(&e.tipo),
        cadena_json(&e.momento_registro),
        cadena_json(&e.dispositivo_origen),
        datos
    )
}

/// Texto legible del libro: el encabezado y un evento por línea, en el orden recibido.
/// No incluye credenciales, tokens ni la frase de paso (RF-INT-05).
///
/// `exportado` es el instante de la exportación en UTC, como lo da `Reloj::ahora_utc`.
pub fn libro_a_texto(eventos: &[EventoRegistrado], exportado: &str) -> String {
    let mut salida = format!(
        "{MARCA_FORMATO} version={VERSION_FORMATO} exportado={exportado} eventos={}\n",
        eventos.len()
    );
    for e in eventos {
        salida.push_str(&evento_a_linea(e));
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

// ---------- Lectura ----------

/// `AAAA-MM-DDTHH:MM:SSZ`, con fracción de segundo opcional (`…:SS.123Z`).
fn es_instante_utc(texto: &str) -> bool {
    let b = texto.as_bytes();
    if b.len() < 20 || b[b.len() - 1] != b'Z' {
        return false;
    }
    let digito = |i: usize| b[i].is_ascii_digit();
    let base = (0..4).all(digito)
        && b[4] == b'-'
        && (5..7).all(digito)
        && b[7] == b'-'
        && (8..10).all(digito)
        && b[10] == b'T'
        && (11..13).all(digito)
        && b[13] == b':'
        && (14..16).all(digito)
        && b[16] == b':'
        && (17..19).all(digito);
    let fraccion = &b[19..b.len() - 1];
    let fraccion_valida = fraccion.is_empty()
        || (fraccion.len() >= 2
            && fraccion[0] == b'.'
            && fraccion[1..].iter().all(u8::is_ascii_digit));
    base && fraccion_valida
}

/// Un número decimal sin signo ni ceros a la izquierda (`0`, `1`, `42`).
fn numero(texto: &str) -> Option<u64> {
    let canonico = !texto.is_empty()
        && texto.bytes().all(|c| c.is_ascii_digit())
        && (texto == "0" || !texto.starts_with('0'));
    if canonico { texto.parse().ok() } else { None }
}

/// Lee y valida la primera línea. Debe ser exactamente
/// `# cuentas-claras-respaldo version=N exportado=AAAA-MM-DDTHH:MM:SSZ eventos=M`.
pub fn leer_encabezado(linea: &str) -> Result<EncabezadoExportacion, ErrorRespaldo> {
    let invalido = || ErrorRespaldo::ArchivoInvalido;
    let resto = linea.strip_prefix(MARCA_FORMATO).ok_or_else(invalido)?;
    let partes: Vec<&str> = resto.split(' ').collect();
    // `resto` empieza con un espacio: la primera parte es vacía.
    let [vacio, version, exportado, eventos] = partes.as_slice() else {
        return Err(invalido());
    };
    if !vacio.is_empty() {
        return Err(invalido());
    }
    // La versión se mira primero: un formato futuro puede cambiar el resto del encabezado.
    let version = version
        .strip_prefix("version=")
        .and_then(numero)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(invalido)?;
    if version != VERSION_FORMATO {
        return Err(ErrorRespaldo::VersionNoSoportada {
            archivo: version,
            soportada: VERSION_FORMATO,
        });
    }
    let exportado = exportado
        .strip_prefix("exportado=")
        .filter(|e| es_instante_utc(e))
        .ok_or_else(invalido)?;
    let eventos = eventos
        .strip_prefix("eventos=")
        .and_then(numero)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(invalido)?;
    Ok(EncabezadoExportacion {
        version,
        exportado: exportado.to_string(),
        eventos,
    })
}

/// Una línea de evento tal como está en el archivo. Rechaza claves desconocidas o
/// repetidas y campos que falten.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineaEvento<'a> {
    id: String,
    tipo: String,
    momento_registro: String,
    dispositivo_origen: String,
    #[serde(borrow)]
    datos: &'a RawValue,
}

fn linea_a_evento(linea: &str) -> Option<EventoRegistrado> {
    let leida: LineaEvento = serde_json::from_str(linea).ok()?;
    if leida.id.is_empty() || leida.tipo.is_empty() || leida.momento_registro.is_empty() {
        return None;
    }
    let crudo = leida.datos.get();
    let datos = if crudo.starts_with('{') {
        crudo.to_string()
    } else if crudo.starts_with('"') {
        serde_json::from_str::<String>(crudo).ok()?
    } else {
        return None;
    };
    Some(EventoRegistrado {
        id: leida.id,
        tipo: leida.tipo,
        momento_registro: leida.momento_registro,
        dispositivo_origen: leida.dispositivo_origen,
        datos,
    })
}

/// Lee el texto completo de una exportación, todo o nada: devuelve los eventos en el orden
/// del archivo o `ArchivoInvalido` / `VersionNoSoportada` sin ningún evento.
///
/// El tipo de cada evento no se comprueba aquí: un tipo desconocido se rechaza al
/// fusionarlo, evento por evento, con su motivo.
pub fn texto_a_libro(texto: &str) -> Result<Vec<EventoRegistrado>, ErrorRespaldo> {
    let invalido = || ErrorRespaldo::ArchivoInvalido;
    // Cada línea, incluida la última, termina en `\n`.
    let cuerpo = texto.strip_suffix('\n').ok_or_else(invalido)?;
    let mut lineas = cuerpo.split('\n');
    let encabezado = leer_encabezado(lineas.next().ok_or_else(invalido)?)?;

    let mut eventos = Vec::with_capacity(encabezado.eventos.min(100_000));
    let mut ids = BTreeSet::new();
    for linea in lineas {
        let evento = linea_a_evento(linea).ok_or_else(invalido)?;
        if !ids.insert(evento.id.clone()) {
            return Err(invalido());
        }
        eventos.push(evento);
        if eventos.len() > encabezado.eventos {
            return Err(invalido());
        }
    }
    if eventos.len() != encabezado.eventos {
        return Err(invalido());
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
    let texto = if bytes.starts_with(MARCA_FORMATO.as_bytes()) {
        String::from_utf8(bytes.to_vec()).map_err(|_| ErrorRespaldo::ArchivoInvalido)?
    } else {
        let frase = frase.ok_or(ErrorRespaldo::FraseRequerida)?;
        let claro = cifrador.descifrar(frase, bytes)?;
        String::from_utf8(claro).map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)?
    };
    texto_a_libro(&texto)
}
