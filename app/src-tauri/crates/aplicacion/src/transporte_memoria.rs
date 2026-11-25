//! Adaptador falso, en memoria, del puerto [`TransporteSincronizacion`]
//! (Entrega 8: Ernesto - Tarea 2).
//!
//! Simula un destino compartido (el «repositorio» o la «carpeta») al que se conectan
//! varios dispositivos, cada uno con su propio [`TransporteEnMemoria`]. Se comporta
//! como Git, que es el caso más exigente: cada dispositivo tiene una copia local, lo
//! que escribe solo lo ven los demás después de `publicar`, y los demás solo lo
//! reciben cuando hacen `traer`. Así las pruebas de los casos de uso detectan a quien
//! olvide uno de los dos pasos.
//!
//! También simula los fallos de la sección 2.7 de la especificación:
//!
//! - **Sin conexión** (de un dispositivo): [`TransporteEnMemoria::quedar_sin_conexion`].
//! - **Destino inaccesible** (para todos): [`DestinoEnMemoria::dejar_inaccesible`].
//! - **Credenciales rechazadas**: [`DestinoEnMemoria::rechazar_credenciales`].
//! - **El destino cambió al publicar**: [`DestinoEnMemoria::hacer_que_el_destino_cambie`].
//!
//! Como en Git, lo local (listar, leer y escribir lotes propios) no necesita red; solo
//! `traer` y `publicar` fallan. Ningún fallo pierde lo ya escrito.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::transporte::{ErrorTransporte, TransporteSincronizacion, validar_nombre};

/// Lotes de un dispositivo: nombre del lote → contenido.
type Lotes = BTreeMap<String, Vec<u8>>;

/// Lo que guarda el destino compartido y los fallos que se le han pedido simular.
#[derive(Debug, Default)]
struct EstadoDestino {
    /// Lotes publicados, por dispositivo.
    publicados: BTreeMap<String, Lotes>,
    inaccesible: bool,
    credenciales_rechazadas: bool,
    /// Cuántas publicaciones más deben fallar con `DestinoCambio`.
    cambios_pendientes: u32,
    /// Cuántas publicaciones con contenido se han aceptado.
    publicaciones: u64,
}

fn bloquear(destino: &Mutex<EstadoDestino>) -> MutexGuard<'_, EstadoDestino> {
    destino.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Fallos de red, en el orden en que se comprueban: primero la conexión del propio
/// dispositivo, luego el destino y por último las credenciales.
fn comprobar_red(sin_conexion: bool, estado: &EstadoDestino) -> Result<(), ErrorTransporte> {
    if sin_conexion {
        return Err(ErrorTransporte::SinConexion);
    }
    if estado.inaccesible {
        return Err(ErrorTransporte::DestinoInaccesible);
    }
    if estado.credenciales_rechazadas {
        return Err(ErrorTransporte::CredencialesRechazadas);
    }
    Ok(())
}

/// Destino compartido simulado. Se clona barato: todos los clones son el mismo destino.
#[derive(Debug, Clone, Default)]
pub struct DestinoEnMemoria {
    estado: Arc<Mutex<EstadoDestino>>,
}

impl DestinoEnMemoria {
    /// Crea un destino vacío y sin fallos.
    pub fn nuevo() -> DestinoEnMemoria {
        DestinoEnMemoria::default()
    }

    /// Conecta un dispositivo al destino. La copia local empieza vacía: hasta que
    /// el dispositivo haga `traer` no ve nada de lo publicado.
    pub fn conectar(&self, dispositivo: &str) -> Result<TransporteEnMemoria, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        Ok(TransporteEnMemoria {
            dispositivo: dispositivo.to_string(),
            destino: Arc::clone(&self.estado),
            copia: BTreeMap::new(),
            pendientes: BTreeSet::new(),
            sin_conexion: false,
            lotes_traidos: 0,
            lecturas: AtomicU64::new(0),
        })
    }

    /// El destino deja de responder para todos los dispositivos (`traer` y `publicar`
    /// fallan con `DestinoInaccesible`).
    pub fn dejar_inaccesible(&self) {
        bloquear(&self.estado).inaccesible = true;
    }

    /// El destino vuelve a responder.
    pub fn restablecer(&self) {
        bloquear(&self.estado).inaccesible = false;
    }

    /// Con `true`, `traer` y `publicar` fallan con `CredencialesRechazadas` hasta que
    /// se vuelva a poner en `false` (el usuario las actualizó).
    pub fn rechazar_credenciales(&self, rechazar: bool) {
        bloquear(&self.estado).credenciales_rechazadas = rechazar;
    }

    /// Las próximas `veces` publicaciones fallan con `DestinoCambio`, como si otro
    /// dispositivo hubiera publicado en medio. La siguiente tiene éxito. Sirve para
    /// probar el reintento del paso 4 (traer de nuevo y volver a publicar).
    pub fn hacer_que_el_destino_cambie(&self, veces: u32) {
        bloquear(&self.estado).cambios_pendientes = veces;
    }

    /// Nombres de los lotes publicados de un dispositivo, en orden.
    pub fn lotes_publicados_de(&self, dispositivo: &str) -> Vec<String> {
        bloquear(&self.estado)
            .publicados
            .get(dispositivo)
            .map(|lotes| lotes.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Contenido de todos los lotes publicados (de cualquier dispositivo). Sirve para
    /// comprobar que el destino solo guarda bytes cifrados (RNF-01).
    pub fn contenidos_publicados(&self) -> Vec<Vec<u8>> {
        bloquear(&self.estado)
            .publicados
            .values()
            .flat_map(|lotes| lotes.values().cloned())
            .collect()
    }

    /// Cuántas veces se publicó algo (publicar sin nada pendiente no cuenta).
    pub fn publicaciones(&self) -> u64 {
        bloquear(&self.estado).publicaciones
    }
}

/// Transporte de un dispositivo conectado a un [`DestinoEnMemoria`].
#[derive(Debug)]
pub struct TransporteEnMemoria {
    dispositivo: String,
    destino: Arc<Mutex<EstadoDestino>>,
    /// Lo que este dispositivo ve: sus propios lotes (publicados o no) más lo que
    /// trajo del destino con `traer`.
    copia: BTreeMap<String, Lotes>,
    /// Lotes propios escritos que todavía no se publicaron.
    pendientes: BTreeSet<String>,
    sin_conexion: bool,
    lotes_traidos: u64,
    lecturas: AtomicU64,
}

impl TransporteEnMemoria {
    /// Este dispositivo pierde la conexión: `traer` y `publicar` fallan con
    /// `SinConexion`; escribir y leer lo local sigue funcionando.
    pub fn quedar_sin_conexion(&mut self) {
        self.sin_conexion = true;
    }

    /// Este dispositivo recupera la conexión.
    pub fn recuperar_conexion(&mut self) {
        self.sin_conexion = false;
    }

    /// Cuántos lotes de otros dispositivos (o propios ya publicados, si se reinstaló)
    /// ha copiado `traer` desde que se creó este transporte. Un `traer` sin novedades
    /// no lo aumenta: sirve para comprobar que solo se transfiere lo que falta (RNF-10).
    pub fn lotes_traidos(&self) -> u64 {
        self.lotes_traidos
    }

    /// Cuántas veces se ha llamado a `leer_lote` en este transporte.
    pub fn lecturas(&self) -> u64 {
        self.lecturas.load(Ordering::Relaxed)
    }

    /// Cuántos lotes propios esperan a `publicar`.
    pub fn pendientes_de_publicar(&self) -> usize {
        self.pendientes.len()
    }
}

impl TransporteSincronizacion for TransporteEnMemoria {
    fn dispositivo_local(&self) -> &str {
        &self.dispositivo
    }

    fn traer(&mut self) -> Result<(), ErrorTransporte> {
        let estado = bloquear(&self.destino);
        comprobar_red(self.sin_conexion, &estado)?;
        for (dispositivo, lotes) in &estado.publicados {
            for (lote, contenido) in lotes {
                let local = self.copia.entry(dispositivo.clone()).or_default();
                if !local.contains_key(lote) {
                    local.insert(lote.clone(), contenido.clone());
                    self.lotes_traidos += 1;
                }
            }
        }
        Ok(())
    }

    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte> {
        Ok(self
            .copia
            .iter()
            .filter(|(_, lotes)| !lotes.is_empty())
            .map(|(dispositivo, _)| dispositivo.clone())
            .collect())
    }

    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        Ok(self
            .copia
            .get(dispositivo)
            .map(|lotes| lotes.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        validar_nombre(lote)?;
        self.lecturas.fetch_add(1, Ordering::Relaxed);
        self.copia
            .get(dispositivo)
            .and_then(|lotes| lotes.get(lote))
            .cloned()
            .ok_or_else(|| ErrorTransporte::LoteNoExiste {
                dispositivo: dispositivo.to_string(),
                lote: lote.to_string(),
            })
    }

    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte> {
        validar_nombre(lote)?;
        let propios = self.copia.entry(self.dispositivo.clone()).or_default();
        if propios.contains_key(lote) {
            return Err(ErrorTransporte::LoteYaExiste {
                dispositivo: self.dispositivo.clone(),
                lote: lote.to_string(),
            });
        }
        propios.insert(lote.to_string(), contenido.to_vec());
        self.pendientes.insert(lote.to_string());
        Ok(())
    }

    fn publicar(&mut self) -> Result<(), ErrorTransporte> {
        let mut estado = bloquear(&self.destino);
        comprobar_red(self.sin_conexion, &estado)?;
        if estado.cambios_pendientes > 0 {
            estado.cambios_pendientes -= 1;
            return Err(ErrorTransporte::DestinoCambio);
        }
        if self.pendientes.is_empty() {
            return Ok(());
        }
        {
            let propios = self.copia.get(&self.dispositivo);
            let publicados = estado.publicados.entry(self.dispositivo.clone()).or_default();
            // Todo o nada: si algún lote ya está publicado con ese nombre, no se publica ninguno.
            if let Some(lote) = self
                .pendientes
                .iter()
                .find(|lote| publicados.contains_key(lote.as_str()))
            {
                return Err(ErrorTransporte::LoteYaExiste {
                    dispositivo: self.dispositivo.clone(),
                    lote: lote.clone(),
                });
            }
            for lote in &self.pendientes {
                if let Some(contenido) = propios.and_then(|lotes| lotes.get(lote)) {
                    publicados.insert(lote.clone(), contenido.clone());
                }
            }
        }
        estado.publicaciones += 1;
        self.pendientes.clear();
        Ok(())
    }
}
