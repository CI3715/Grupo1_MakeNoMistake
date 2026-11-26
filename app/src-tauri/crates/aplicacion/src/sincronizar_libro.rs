//! Sincronización del libro con un destino propio (Entrega 8: RF-INT-03, RF-INT-04, RNF-10).
//!
//! Los pasos son los de la sección 2.5: traer, fusionar, enviar y publicar. Si al publicar
//! el destino cambió, se vuelve a traer y se reintenta. Esta función no sabe si el destino
//! es una carpeta o un repositorio Git: solo habla con el puerto `TransporteSincronizacion`.
//!
//! Nada se guarda aquí. `sincronizar` recibe el estado de la sincronización anterior y
//! devuelve el nuevo solo si todo salió bien; quien la llama lo guarda. Si algo falla, el
//! estado viejo sigue valiendo y lo pendiente sale en la siguiente sincronización: la
//! fusión es idempotente (INV-13), así que repetir un lote no daña nada.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::casos_de_uso::CasosDeUso;
use crate::puertos::EventoRegistrado;
use crate::respaldo::{Cifrador, ErrorRespaldo, exportar_libro};
use crate::transporte::{
    ErrorTransporte, LoteDeDispositivo, TransporteSincronizacion, lotes_por_leer, nombre_de_lote,
};

/// Veces que se vuelve a traer y publicar si el destino cambió mientras tanto.
const REINTENTOS_PUBLICAR: usize = 3;

/// Intentos de nombre de lote antes de rendirse (un nombre ya usado se salta).
const INTENTOS_NOMBRE_LOTE: usize = 1000;

/// Lo que este dispositivo recuerda de su última sincronización con el destino.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstadoSincronizacion {
    /// Lotes ya aplicados (propios y ajenos): no se vuelven a leer (RNF-10).
    pub leidos: BTreeSet<LoteDeDispositivo>,
    /// Cuántos eventos del libro, contados en orden de inserción, ya están en el destino.
    pub enviados_hasta: usize,
    /// Número del próximo lote propio: desempata lotes del mismo instante.
    pub secuencia: u32,
}

/// Un evento recibido que no se aplicó, con su motivo en español.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RechazoSincronizacion {
    pub id: String,
    pub motivo: String,
}

/// Lo que se informa tras cada sincronización (RF-INT-04).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformeSincronizacion {
    pub enviados: usize,
    pub recibidos: usize,
    pub rechazados: Vec<RechazoSincronizacion>,
}

/// Por qué falló una sincronización. Los mensajes son los de cada error, sin trazas.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorSincronizacion {
    #[error("{0}")]
    Transporte(#[from] ErrorTransporte),
    #[error("{0}")]
    Respaldo(#[from] ErrorRespaldo),
}

/// Escribe un lote nuevo con un nombre libre y lo marca como leído, para no releerlo.
fn escribir_lote_nuevo(
    transporte: &mut dyn TransporteSincronizacion,
    estado: &mut EstadoSincronizacion,
    ahora: &str,
    contenido: &[u8],
) -> Result<(), ErrorTransporte> {
    let dispositivo = transporte.dispositivo_local().to_string();
    let mut ultimo = None;
    for _ in 0..INTENTOS_NOMBRE_LOTE {
        let nombre = nombre_de_lote(ahora, estado.secuencia);
        estado.secuencia += 1;
        match transporte.escribir_lote(&nombre, contenido) {
            Ok(()) => {
                estado.leidos.insert((dispositivo.clone(), nombre));
                return Ok(());
            }
            Err(e @ ErrorTransporte::LoteYaExiste { .. }) => ultimo = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(ultimo.unwrap_or_else(|| ErrorTransporte::Interno("sin nombre de lote libre".into())))
}

/// Sincroniza el libro con el destino: trae, fusiona, envía y publica.
///
/// Solo lee los lotes que faltan y solo envía los eventos que el destino todavía no tiene.
/// Devuelve el informe y el estado nuevo; si falla, no devuelve estado y no se pierde nada.
pub fn sincronizar(
    casos: &mut CasosDeUso,
    transporte: &mut dyn TransporteSincronizacion,
    cifrador: &dyn Cifrador,
    frase: &str,
    ahora: &str,
    estado: &EstadoSincronizacion,
) -> Result<(InformeSincronizacion, EstadoSincronizacion), ErrorSincronizacion> {
    let mut nuevo = estado.clone();
    let mut informe = InformeSincronizacion::default();
    let mut recibidos_ids: BTreeSet<String> = BTreeSet::new();
    let mut escrito = false;
    let mut reintentos = 0;

    loop {
        // 1. Traer lo que otros dispositivos publicaron.
        transporte.traer()?;

        // 2. Fusionar solo los lotes que faltan.
        for (dispositivo, lote) in lotes_por_leer(&*transporte, &nuevo.leidos)? {
            let bytes = transporte.leer_lote(&dispositivo, &lote)?;
            let resumen = casos.importar_copia(&bytes, Some(frase), cifrador)?;
            informe.recibidos += resumen.nuevos.len();
            recibidos_ids.extend(resumen.nuevos.iter().map(|e| e.id.clone()));
            informe
                .rechazados
                .extend(resumen.rechazados.iter().map(|r| RechazoSincronizacion {
                    id: r.evento.id.clone(),
                    motivo: r.motivo.mensaje(),
                }));
            nuevo.leidos.insert((dispositivo, lote));
        }

        // 3. Enviar, en un lote nuevo, los eventos locales que el destino no tiene.
        if !escrito {
            let eventos = casos
                .eventos_del_libro()
                .map_err(|_| ErrorRespaldo::LibroNoLegible)?;
            let a_enviar: Vec<EventoRegistrado> = eventos
                .into_iter()
                .skip(nuevo.enviados_hasta)
                .filter(|e| !recibidos_ids.contains(&e.id))
                .collect();
            if !a_enviar.is_empty() {
                let contenido = exportar_libro(&a_enviar, ahora, Some(frase), cifrador)?;
                escribir_lote_nuevo(transporte, &mut nuevo, ahora, &contenido)?;
                informe.enviados = a_enviar.len();
            }
            escrito = true;
        }

        // 4. Publicar; si el destino cambió, volver a traer y reintentar.
        match transporte.publicar() {
            Ok(()) => break,
            Err(ErrorTransporte::DestinoCambio) if reintentos < REINTENTOS_PUBLICAR => {
                reintentos += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }

    nuevo.enviados_hasta = casos
        .eventos_del_libro()
        .map_err(|_| ErrorRespaldo::LibroNoLegible)?
        .len();
    Ok((informe, nuevo))
}
