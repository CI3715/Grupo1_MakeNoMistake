//! Derivación de clave con Argon2id y cifrado autenticado AES-256-GCM para copias
//! de seguridad y lotes de sincronización (Entrega 8: Angel - Tarea 2; RF-INT-05,
//! RF-INT-06, RNF-01, ADR-017).
//!
//! # Estructura del contenedor cifrado
//!
//! Todo archivo o lote cifrado consta de una cabecera en claro de 46 bytes seguida
//! del texto cifrado y la etiqueta de autenticación GCM de 16 bytes:
//!
//! | Bytes    | Campo         | Descripción                                              |
//! |----------|---------------|----------------------------------------------------------|
//! | `0..4`   | Magia         | Identificador fijo `b"CCBK"` (4 bytes ASCII)             |
//! | `4`      | Versión       | Versión del contenedor (`1u8`)                           |
//! | `5`      | Algoritmo     | `1u8` = Argon2id (v0x13) + AES-256-GCM                   |
//! | `6..10`  | Memoria (KiB) | `m_cost` en KiB (`u32` little-endian, por defecto 19456) |
//! | `10..14` | Iteraciones   | `t_cost` (`u32` little-endian, por defecto 2)            |
//! | `14..18` | Paralelismo   | `p_cost` (`u32` little-endian, por defecto 1)            |
//! | `18..34` | Sal           | 16 bytes aleatorios nuevos por cada cifrado              |
//! | `34..46` | Nonce         | 12 bytes aleatorios nuevos por cada cifrado              |
//! | `46..N-16` | Cuerpo      | Datos del libro cifrados con AES-256-GCM                 |
//! | `N-16..N`  | Etiqueta    | Etiqueta de autenticación GCM de 16 bytes (128 bits)     |
//!
//! La cabecera completa (`0..46`) se autentica como datos asociados (AAD) en
//! AES-256-GCM, de modo que cualquier alteración de un byte en la cabecera, en el
//! cuerpo o en la etiqueta se detecta al descifrar y devuelve
//! [`ErrorRespaldo::FraseIncorrectaOArchivoDanado`].

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use aplicacion::{Cifrador, ErrorRespaldo, LARGO_MINIMO_FRASE};
use argon2::{Algorithm, Argon2, Params, Version};

/// Identificador fijo de 4 bytes del contenedor cifrado de Cuentas Claras.
pub const MAGIA_CONTENEDOR: &[u8; 4] = b"CCBK";

/// Versión actual del formato de contenedor cifrado.
pub const VERSION_CONTENEDOR: u8 = 1;

/// Identificador de la suite criptográfica Argon2id + AES-256-GCM.
pub const ALGORITMO_ARGON2ID_AES256GCM: u8 = 1;

/// Costo de memoria por defecto para Argon2id en KiB (19 MiB = 19 456 KiB, recomendación OWASP).
pub const MEMORIA_ARGON2_KIB: u32 = 19_456;

/// Número de iteraciones (pasadas) por defecto para Argon2id.
pub const ITERACIONES_ARGON2: u32 = 2;

/// Grado de paralelismo (carriles) por defecto para Argon2id.
pub const PARALELISMO_ARGON2: u32 = 1;

/// Tamaño de la sal aleatoria en bytes (128 bits).
pub const LARGO_SAL: usize = 16;

/// Tamaño del nonce aleatorio de AES-256-GCM en bytes (96 bits).
pub const LARGO_NONCE: usize = 12;

/// Tamaño de la clave derivada para AES-256 en bytes (256 bits).
pub const LARGO_CLAVE: usize = 32;

/// Tamaño de la etiqueta de autenticación GCM en bytes (128 bits).
pub const LARGO_ETIQUETA: usize = 16;

/// Tamaño total de la cabecera en claro (4 + 1 + 1 + 4 + 4 + 4 + 16 + 12 = 46 bytes).
pub const LARGO_CABECERA: usize = 46;

/// Tamaño mínimo de un archivo cifrado válido (cabecera de 46 bytes + etiqueta de 16 bytes).
pub const LARGO_MINIMO_ARCHIVO: usize = LARGO_CABECERA + LARGO_ETIQUETA;

// Cotas superiores de seguridad al leer parámetros de la cabecera para evitar
// denegación de servicio por agotamiento de memoria o CPU ante cabeceras corruptas.
const MEMORIA_MIN_KIB: u32 = 8;
const MEMORIA_MAX_KIB: u32 = 262_144; // 256 MiB
const ITERACIONES_MIN: u32 = 1;
const ITERACIONES_MAX: u32 = 32;
const PARALELISMO_MIN: u32 = 1;
const PARALELISMO_MAX: u32 = 16;

/// Parámetros de derivación de clave para Argon2id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParametrosArgon2 {
    /// Memoria en KiB (`m_cost`).
    pub memoria_kib: u32,
    /// Número de iteraciones (`t_cost`).
    pub iteraciones: u32,
    /// Grado de paralelismo (`p_cost`).
    pub paralelismo: u32,
}

impl Default for ParametrosArgon2 {
    fn default() -> Self {
        Self {
            memoria_kib: MEMORIA_ARGON2_KIB,
            iteraciones: ITERACIONES_ARGON2,
            paralelismo: PARALELISMO_ARGON2,
        }
    }
}

/// Adaptador real del puerto [`Cifrador`] que emplea Argon2id y AES-256-GCM.
#[derive(Debug, Clone, Copy, Default)]
pub struct CifradorArgon2 {
    parametros: ParametrosArgon2,
}

impl CifradorArgon2 {
    /// Crea un cifrador con los parámetros de seguridad de producción (ADR-017).
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Crea un cifrador con parámetros de Argon2id personalizados.
    pub fn con_parametros(parametros: ParametrosArgon2) -> Self {
        Self { parametros }
    }
}

impl Cifrador for CifradorArgon2 {
    fn cifrar(&self, frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        cifrar_con_parametros(frase, claro, self.parametros)
    }

    fn descifrar(&self, frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        descifrar(frase, archivo)
    }
}

/// Deriva una clave de 256 bits con Argon2id a partir de `frase` y `sal`.
fn derivar_clave(
    frase: &str,
    sal: &[u8; LARGO_SAL],
    params: ParametrosArgon2,
) -> Result<[u8; LARGO_CLAVE], ErrorRespaldo> {
    if !(MEMORIA_MIN_KIB..=MEMORIA_MAX_KIB).contains(&params.memoria_kib)
        || !(ITERACIONES_MIN..=ITERACIONES_MAX).contains(&params.iteraciones)
        || !(PARALELISMO_MIN..=PARALELISMO_MAX).contains(&params.paralelismo)
    {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    let parametros = Params::new(
        params.memoria_kib,
        params.iteraciones,
        params.paralelismo,
        Some(LARGO_CLAVE),
    )
    .map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, parametros);
    let mut clave = [0u8; LARGO_CLAVE];
    argon2
        .hash_password_into(frase.as_bytes(), sal, &mut clave)
        .map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)?;
    Ok(clave)
}

/// Cifra `claro` con `frase` usando los parámetros por defecto de Argon2id y AES-256-GCM.
///
/// Genera una sal aleatoria de 16 bytes y un nonce aleatorio de 12 bytes en cada
/// invocación. Rechaza frases de menos de [`LARGO_MINIMO_FRASE`] caracteres con
/// [`ErrorRespaldo::FraseCorta`].
pub fn cifrar(frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
    cifrar_con_parametros(frase, claro, ParametrosArgon2::default())
}

/// Cifra `claro` con `frase` y los parámetros de Argon2id indicados.
pub fn cifrar_con_parametros(
    frase: &str,
    claro: &[u8],
    params: ParametrosArgon2,
) -> Result<Vec<u8>, ErrorRespaldo> {
    if frase.chars().count() < LARGO_MINIMO_FRASE {
        return Err(ErrorRespaldo::FraseCorta);
    }

    let mut sal = [0u8; LARGO_SAL];
    let mut nonce_bytes = [0u8; LARGO_NONCE];
    getrandom::getrandom(&mut sal).map_err(|_| ErrorRespaldo::CifradoNoDisponible)?;
    getrandom::getrandom(&mut nonce_bytes).map_err(|_| ErrorRespaldo::CifradoNoDisponible)?;

    let clave = derivar_clave(frase, &sal, params)?;

    let mut cabecera = Vec::with_capacity(LARGO_CABECERA);
    cabecera.extend_from_slice(MAGIA_CONTENEDOR);
    cabecera.push(VERSION_CONTENEDOR);
    cabecera.push(ALGORITMO_ARGON2ID_AES256GCM);
    cabecera.extend_from_slice(&params.memoria_kib.to_le_bytes());
    cabecera.extend_from_slice(&params.iteraciones.to_le_bytes());
    cabecera.extend_from_slice(&params.paralelismo.to_le_bytes());
    cabecera.extend_from_slice(&sal);
    cabecera.extend_from_slice(&nonce_bytes);
    debug_assert_eq!(cabecera.len(), LARGO_CABECERA);

    let cifrador = Aes256Gcm::new_from_slice(&clave)
        .map_err(|_| ErrorRespaldo::CifradoNoDisponible)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let carga = Payload {
        msg: claro,
        aad: &cabecera,
    };

    let cuerpo_y_etiqueta = cifrador
        .encrypt(nonce, carga)
        .map_err(|_| ErrorRespaldo::CifradoNoDisponible)?;

    let mut salida = Vec::with_capacity(LARGO_CABECERA + cuerpo_y_etiqueta.len());
    salida.extend_from_slice(&cabecera);
    salida.extend_from_slice(&cuerpo_y_etiqueta);
    Ok(salida)
}

/// Descifra y verifica la autenticidad de `archivo` con `frase`.
///
/// Si la versión del contenedor es mayor a la soportada (y la cabecera tiene el
/// identificador y algoritmo válidos), devuelve [`ErrorRespaldo::VersionNoSoportada`].
/// Ante cualquier otro fallo (frase incorrecta, frase corta, byte alterado en la
/// cabecera, en el cuerpo o en la etiqueta, o archivo truncado), devuelve
/// [`ErrorRespaldo::FraseIncorrectaOArchivoDanado`].
pub fn descifrar(frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
    if archivo.len() < LARGO_MINIMO_ARCHIVO {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    let cabecera = &archivo[..LARGO_CABECERA];
    if &cabecera[0..4] != MAGIA_CONTENEDOR {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    let version = cabecera[4];
    let algoritmo = cabecera[5];
    let memoria_kib = u32::from_le_bytes([cabecera[6], cabecera[7], cabecera[8], cabecera[9]]);
    let iteraciones = u32::from_le_bytes([cabecera[10], cabecera[11], cabecera[12], cabecera[13]]);
    let paralelismo = u32::from_le_bytes([cabecera[14], cabecera[15], cabecera[16], cabecera[17]]);

    if algoritmo != ALGORITMO_ARGON2ID_AES256GCM {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    if !(MEMORIA_MIN_KIB..=MEMORIA_MAX_KIB).contains(&memoria_kib)
        || !(ITERACIONES_MIN..=ITERACIONES_MAX).contains(&iteraciones)
        || !(PARALELISMO_MIN..=PARALELISMO_MAX).contains(&paralelismo)
    {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    if version == 0 {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }
    if version != VERSION_CONTENEDOR {
        return Err(ErrorRespaldo::VersionNoSoportada {
            archivo: u32::from(version),
            soportada: u32::from(VERSION_CONTENEDOR),
        });
    }

    if frase.chars().count() < LARGO_MINIMO_FRASE {
        return Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    }

    let mut sal = [0u8; LARGO_SAL];
    sal.copy_from_slice(&cabecera[18..34]);

    let nonce_bytes = &cabecera[34..46];
    let params = ParametrosArgon2 {
        memoria_kib,
        iteraciones,
        paralelismo,
    };

    let clave = derivar_clave(frase, &sal, params)?;
    let cifrador = Aes256Gcm::new_from_slice(&clave)
        .map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let carga = Payload {
        msg: &archivo[LARGO_CABECERA..],
        aad: cabecera,
    };

    cifrador
        .decrypt(nonce, carga)
        .map_err(|_| ErrorRespaldo::FraseIncorrectaOArchivoDanado)
}
