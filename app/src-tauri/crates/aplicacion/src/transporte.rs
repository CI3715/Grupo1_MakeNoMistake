//! Puerto del transporte de sincronización (Entrega 8: RF-INT-03, RF-INT-07, RNF-14, RNF-15).
//!
//! El transporte lleva **lotes** de un dispositivo a otro: un lote es una secuencia
//! opaca de bytes (ya cifrada por quien llama) con un nombre. Este puerto no sabe qué
//! hay dentro de un lote, ni qué es un evento, ni qué es Git o una carpeta: eso
//! mantiene el dominio y los casos de uso intactos al agregar un destino nuevo
//! (RNF-15). Los adaptadores son tres, y los tres superan la misma batería de
//! contrato (`contrato_transporte`):
//!
//! - `TransporteEnMemoria` (adaptador falso, en esta capa): simula varios
//!   dispositivos y fallos de red.
//! - `TransporteCarpeta` (`infraestructura`): una carpeta que mantiene sincronizada
//!   otro servicio (Syncthing, Dropbox, Google Drive).
//! - El adaptador Git (`infraestructura`, a cargo de John): un repositorio del usuario.
//!
//! # Reglas del contrato
//!
//! 1. **Cada dispositivo escribe solo en su propia carpeta.** El transporte se crea
//!    para un dispositivo local ([`TransporteSincronizacion::dispositivo_local`]) y
//!    `escribir_lote` solo escribe ahí. Como dos dispositivos nunca escriben el mismo
//!    archivo, Git no produce conflictos de fusión y las carpetas compartidas no
//!    generan «copias en conflicto».
//! 2. **Un lote es inmutable.** Escribir un lote con un nombre que ya existe en la
//!    carpeta del dispositivo falla con [`ErrorTransporte::LoteYaExiste`] y no cambia
//!    nada. Un lote nunca se borra ni se reescribe.
//! 3. **Orden de los pasos.** Para que otro dispositivo vea un lote, quien lo escribió
//!    llama a `publicar` y quien lo quiere leer llama a `traer`. Un adaptador puede
//!    hacer visible el lote antes (la carpeta lo hace al instante), pero quien usa el
//!    puerto no debe contar con ello.
//! 4. **Todo o nada.** Si `escribir_lote` falla, no queda un lote a medias: ni visible
//!    ni con contenido parcial. Un reintento posterior con el mismo nombre funciona.
//! 5. **Los fallos de red no pierden nada.** Con el destino inaccesible o sin
//!    conexión, lo escrito se conserva y se puede volver a intentar `publicar` más
//!    tarde (RNF-17, sección 2.5 de la especificación).
//! 6. **Solo se transfiere lo que falta (RNF-10).** `listar_lotes` devuelve solo
//!    nombres; el contenido se pide lote por lote con `leer_lote`. Quien sincroniza
//!    guarda qué lotes ya leyó y no vuelve a pedirlos: [`lotes_por_leer`] devuelve solo
//!    los que faltan.
//! 7. **Los nombres son seguros.** Los identificadores de dispositivo y de lote usan
//!    solo letras ASCII, números, guion y guion bajo (ver [`validar_nombre`]): nunca
//!    pueden salirse de la carpeta del destino.
//!
//! # Cómo agregar un destino nuevo
//!
//! Se implementa `TransporteSincronizacion` en `infraestructura` y se pasa la batería
//! de contrato implementando `BancoDeDestino` para el destino nuevo. No se toca
//! `dominio` ni los casos de uso (RNF-15). La guía completa está en
//! `docs/adr/ADR-018-puerto-transporte-sincronizacion.md`.

use std::collections::BTreeSet;

use thiserror::Error;

/// Largo máximo, en caracteres, de un identificador de dispositivo o de lote.
pub const LARGO_MAXIMO_NOMBRE: usize = 128;

/// Errores del transporte de sincronización. Los mensajes están pensados para
/// mostrarse tal cual al usuario: dicen qué ocurrió y qué puede hacer, sin trazas
/// técnicas (RNF-18). El detalle técnico de [`ErrorTransporte::Interno`] no se muestra.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorTransporte {
    /// Este dispositivo no tiene conexión. No se pierde nada.
    #[error("Sin conexión: no se pudo llegar al repositorio o a la carpeta. Se reintentará.")]
    SinConexion,

    /// El repositorio o la carpeta no responden (caído, desmontado, sin permisos).
    #[error("No se pudo llegar al repositorio o a la carpeta; se reintentará.")]
    DestinoInaccesible,

    /// El repositorio rechazó las credenciales guardadas. Hay que actualizarlas.
    #[error(
        "Las credenciales fueron rechazadas. Actualízalas en la configuración del destino de sincronización."
    )]
    CredencialesRechazadas,

    /// Otro dispositivo publicó mientras este publicaba. Hay que volver a traer y
    /// reintentar (paso 4 de la sincronización).
    #[error("El destino cambió mientras se publicaba; se volverá a traer y a intentar.")]
    DestinoCambio,

    /// Ya existe un lote con ese nombre en la carpeta del dispositivo: los lotes
    /// no se modifican.
    #[error("El lote {lote} del dispositivo {dispositivo} ya existe y no se puede modificar.")]
    LoteYaExiste { dispositivo: String, lote: String },

    /// No existe el lote pedido.
    #[error("No se encontró el lote {lote} del dispositivo {dispositivo}.")]
    LoteNoExiste { dispositivo: String, lote: String },

    /// Un identificador de dispositivo o de lote no cumple [`validar_nombre`].
    #[error("El nombre «{0}» no es válido: usa solo letras, números, guion y guion bajo.")]
    NombreInvalido(String),

    /// Fallo inesperado del adaptador. El texto es para el registro, no para el usuario.
    #[error("No se pudo leer o escribir en el destino de sincronización. Intenta de nuevo.")]
    Interno(String),
}

impl ErrorTransporte {
    /// `true` si el fallo es transitorio y basta con reintentar más tarde sin que el
    /// usuario haga nada: sin conexión, destino inaccesible o destino que cambió.
    /// Las credenciales rechazadas no lo son: requieren que el usuario las actualice.
    pub fn es_reintentable(&self) -> bool {
        matches!(
            self,
            ErrorTransporte::SinConexion
                | ErrorTransporte::DestinoInaccesible
                | ErrorTransporte::DestinoCambio
        )
    }
}

/// Comprueba que `nombre` sirve como identificador de dispositivo o de lote: de 1 a
/// [`LARGO_MAXIMO_NOMBRE`] caracteres, solo letras ASCII, dígitos, `-` y `_`. Con eso
/// ningún nombre puede contener separadores de ruta, puntos ni espacios, y un
/// archivo temporal (`.tmp`) o de conflicto nunca se confunde con un lote.
pub fn validar_nombre(nombre: &str) -> Result<(), ErrorTransporte> {
    let valido = !nombre.is_empty()
        && nombre.chars().count() <= LARGO_MAXIMO_NOMBRE
        && nombre
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valido {
        Ok(())
    } else {
        Err(ErrorTransporte::NombreInvalido(nombre.to_string()))
    }
}

/// Nombre de lote que se ordena igual por texto que por tiempo: el momento de
/// registro sin separadores (`2026-11-24T12:00:00Z` pasa a `20261124T120000Z`) más una
/// secuencia de cuatro cifras que desempata lotes del mismo instante
/// (`20261124T120000Z-0001`). Los adaptadores devuelven los lotes en orden de texto.
pub fn nombre_de_lote(momento_utc: &str, secuencia: u32) -> String {
    let marca: String = momento_utc
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    format!("{marca}-{secuencia:04}")
}

/// Un lote identificado por su dispositivo y su nombre: `(dispositivo, lote)`.
pub type LoteDeDispositivo = (String, String);

/// Lotes visibles en el transporte que todavía no están en `ya_leidos`, en orden de
/// dispositivo y luego de lote (orden de texto, que coincide con el orden de tiempo).
///
/// Solo consulta nombres (`listar_dispositivos` y `listar_lotes`): no lee contenido.
/// Así quien sincroniza pide con `leer_lote` únicamente los lotes que faltan y no
/// vuelve a leer los que ya aplicó (RNF-10). Los archivos temporales, las copias en
/// conflicto y cualquier otra cosa que no sea un lote terminado no aparecen aquí
/// porque el adaptador ya los descarta al listar.
///
/// Quien llama añade cada `(dispositivo, lote)` a `ya_leidos` solo después de aplicar
/// su contenido: si algo falla a mitad, el lote vuelve a aparecer en la siguiente
/// sincronización y la fusión (idempotente, INV-13) lo deja una sola vez. Los lotes
/// del propio dispositivo también se listan; para no releerlos, se marcan como
/// leídos al escribirlos.
pub fn lotes_por_leer(
    transporte: &dyn TransporteSincronizacion,
    ya_leidos: &BTreeSet<LoteDeDispositivo>,
) -> Result<Vec<LoteDeDispositivo>, ErrorTransporte> {
    let mut faltan = Vec::new();
    for dispositivo in transporte.listar_dispositivos()? {
        for lote in transporte.listar_lotes(&dispositivo)? {
            let clave = (dispositivo.clone(), lote);
            if !ya_leidos.contains(&clave) {
                faltan.push(clave);
            }
        }
    }
    Ok(faltan)
}

/// Puerto del transporte de sincronización. Ver la documentación del módulo para las
/// reglas del contrato.
///
/// Exige `Send` como el resto de los puertos, para poder vivir dentro del
/// `Mutex` del cableado de Tauri.
pub trait TransporteSincronizacion: Send {
    /// Identificador del dispositivo para el que se creó este transporte. Es el único
    /// que puede escribir lotes con él.
    fn dispositivo_local(&self) -> &str;

    /// Paso 1 de la sincronización: trae del destino lo que otros dispositivos han
    /// publicado (en Git, `fetch`; en una carpeta, solo comprueba que es accesible).
    /// No modifica los lotes propios.
    fn traer(&mut self) -> Result<(), ErrorTransporte>;

    /// Dispositivos que tienen al menos un lote visible, en orden de texto. Incluye
    /// al dispositivo local si ya escribió alguno.
    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte>;

    /// Nombres de los lotes de `dispositivo`, en orden de texto. Un dispositivo
    /// desconocido no es un error: devuelve la lista vacía. No lee el contenido.
    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte>;

    /// Contenido de un lote.
    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte>;

    /// Paso 3 de la sincronización: escribe un lote nuevo e inmutable en la carpeta
    /// del dispositivo local. Falla con [`ErrorTransporte::LoteYaExiste`] si ya hay
    /// uno con ese nombre. Un lote vacío es válido.
    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte>;

    /// Paso 4 de la sincronización: hace visibles para los demás los lotes escritos
    /// (en Git, confirmar y empujar; en una carpeta no hace falta nada más). Publicar
    /// sin nada pendiente es válido y no cambia nada. Si el destino cambió mientras
    /// tanto devuelve [`ErrorTransporte::DestinoCambio`]: se vuelve a `traer` y se
    /// reintenta.
    fn publicar(&mut self) -> Result<(), ErrorTransporte>;
}
