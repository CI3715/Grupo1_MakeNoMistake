//! Fusión del libro como conjunto de eventos inmutables (Entrega 8, sección 2.1).
//!
//! El libro es un conjunto: dos libros se unen por identificador. Un evento que ya
//! existe no se vuelve a agregar y nunca se borra ni se modifica uno local
//! (INV-13, INV-14). Si un identificador llega con contenido distinto, ese evento
//! se rechaza con motivo y los demás se aplican normalmente.
//!
//! Esta función no toca el disco ni habla con la red: solo calcula un informe.
//! Quien lo aplica es `CasosDeUso::fusionar_eventos`, que escribe los eventos nuevos
//! en una sola transacción a través del repositorio.

use std::collections::HashMap;

use serde_json::Value;

use crate::puertos::EventoRegistrado;

/// Motivo por el que un evento entrante no se incorporó al libro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoRechazo {
    /// El identificador está vacío: no se puede unir por identificador.
    IdVacio,
    /// El identificador ya existía pero el tipo o la carga son distintos.
    ContenidoDistinto { id: String },
}

impl MotivoRechazo {
    /// Texto para el informe y la interfaz, sin trazas técnicas (RNF-18).
    pub fn mensaje(&self) -> String {
        match self {
            MotivoRechazo::IdVacio => "El evento no tiene identificador.".to_string(),
            MotivoRechazo::ContenidoDistinto { id } => {
                format!("El evento {id} ya existía con un contenido distinto.")
            }
        }
    }
}

/// Evento que no se aplicó, junto con el motivo por el que se rechazó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoRechazado {
    pub evento: EventoRegistrado,
    pub motivo: MotivoRechazo,
}

/// Resultado de una fusión: qué se agregó, qué ya estaba y qué se rechazó.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformeFusion {
    /// Eventos nuevos, en orden determinista y listos para guardar.
    pub nuevos: Vec<EventoRegistrado>,
    /// Cantidad de eventos entrantes que ya estaban en el libro.
    pub ya_existian: usize,
    /// Eventos no aplicados, con su motivo. El resto sí se aplica.
    pub rechazados: Vec<EventoRechazado>,
}

/// Clave de orden determinista: momento de registro, dispositivo e identificador.
///
/// La usan la fusión y la proyección de saldos para que dos dispositivos con el
/// mismo conjunto de eventos muestren los mismos saldos sin importar el orden
/// en que recibieron los eventos.
pub fn clave_orden(evento: &EventoRegistrado) -> (String, String, String) {
    (
        evento.momento_registro.clone(),
        evento.dispositivo_origen.clone(),
        evento.id.clone(),
    )
}

/// Ordena los eventos por la clave canónica, sin tocar su contenido.
///
/// Es la única definición del orden en que se recorren los eventos para derivar
/// el libro: lo mismo da que hayan llegado en un orden u otro.
pub fn ordenar_eventos(eventos: &mut [EventoRegistrado]) {
    eventos.sort_by_key(clave_orden);
}

/// Compara el contenido del evento, no su procedencia.
///
/// `momento_registro` y `dispositivo_origen` quedan fuera: las partes de gasto de
/// grupo (Entrega 7) identifican su evento con el id del servidor y cada
/// dispositivo las sella con su propio momento y dispositivo. Comparar esos dos
/// campos haría que la misma parte llegara como un conflicto falso.
fn mismo_contenido(uno: &EventoRegistrado, otro: &EventoRegistrado) -> bool {
    if uno.tipo != otro.tipo {
        return false;
    }
    match (
        serde_json::from_str::<Value>(&uno.datos),
        serde_json::from_str::<Value>(&otro.datos),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => uno.datos == otro.datos,
    }
}

/// Une `entrantes` con `locales` por identificador y devuelve el informe.
///
/// Es pura: no modifica ningún libro. Un mismo id dentro de `entrantes` se cuenta
/// una sola vez; las repeticiones posteriores se clasifican como ya existían o
/// como rechazadas según su contenido.
pub fn fusionar(locales: &[EventoRegistrado], entrantes: &[EventoRegistrado]) -> InformeFusion {
    let mut por_id: HashMap<&str, &EventoRegistrado> = HashMap::with_capacity(locales.len());
    for evento in locales {
        por_id.entry(evento.id.as_str()).or_insert(evento);
    }

    let mut informe = InformeFusion::default();
    for entrante in entrantes {
        if entrante.id.trim().is_empty() {
            informe.rechazados.push(EventoRechazado {
                evento: entrante.clone(),
                motivo: MotivoRechazo::IdVacio,
            });
            continue;
        }

        match por_id.get(entrante.id.as_str()) {
            Some(existente) if mismo_contenido(existente, entrante) => {
                informe.ya_existian += 1;
            }
            Some(_) => {
                informe.rechazados.push(EventoRechazado {
                    evento: entrante.clone(),
                    motivo: MotivoRechazo::ContenidoDistinto {
                        id: entrante.id.clone(),
                    },
                });
            }
            None => {
                por_id.insert(entrante.id.as_str(), entrante);
                informe.nuevos.push(entrante.clone());
            }
        }
    }

    ordenar_eventos(&mut informe.nuevos);
    informe
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn evento(id: &str, momento: &str, dispositivo: &str, datos: &str) -> EventoRegistrado {
        EventoRegistrado {
            id: id.to_string(),
            tipo: "CuentaCreada".to_string(),
            momento_registro: momento.to_string(),
            dispositivo_origen: dispositivo.to_string(),
            datos: datos.to_string(),
        }
    }

    const DATOS_UNO: &str = r#"{"id":"c1","nombre":"Uno"}"#;
    const DATOS_DOS: &str = r#"{"id":"c2","nombre":"Dos"}"#;
    const DATOS_DISTINTOS: &str = r#"{"id":"c1","nombre":"Otro"}"#;

    #[test]
    fn fusionar_dos_veces_equivale_a_una() {
        let locales = vec![evento("e1", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO)];
        let primera = fusionar(&locales, &locales.clone());
        assert!(primera.nuevos.is_empty());
        assert_eq!(primera.ya_existian, 1);
        assert!(primera.rechazados.is_empty());

        let resultado = fusionar(&locales, &primera.nuevos);
        assert_eq!(resultado, InformeFusion::default());
    }

    #[test]
    fn el_orden_de_llegada_no_cambia_el_resultado() {
        let a = evento("e-a", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO);
        let b = evento("e-b", "2026-11-24T11:00:00Z", "teléfono", DATOS_DOS);

        let uno = fusionar(&[], &[b.clone(), a.clone()]);
        let otro = fusionar(&[], &[a.clone(), b.clone()]);

        assert_eq!(uno, otro);
        assert_eq!(uno.nuevos, vec![a, b]);
    }

    #[test]
    fn un_id_repetido_con_contenido_distinto_se_rechaza_y_el_resto_se_aplica() {
        let locales = vec![evento("e1", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO)];
        let entrantes = vec![
            evento("e1", "2026-11-24T12:00:00Z", "teléfono", DATOS_DISTINTOS),
            evento("e2", "2026-11-24T11:00:00Z", "teléfono", DATOS_DOS),
        ];

        let informe = fusionar(&locales, &entrantes);

        assert_eq!(informe.nuevos.len(), 1);
        assert_eq!(informe.nuevos[0].id, "e2");
        assert_eq!(informe.ya_existian, 0);
        assert_eq!(informe.rechazados.len(), 1);
        assert_eq!(informe.rechazados[0].evento.id, "e1");
        assert_eq!(
            informe.rechazados[0].motivo,
            MotivoRechazo::ContenidoDistinto {
                id: "e1".to_string()
            }
        );
        assert_eq!(locales.len(), 1, "el libro local no se modifica");
    }

    #[test]
    fn un_lote_antiguo_o_parcial_no_elimina_eventos_locales() {
        let locales = vec![
            evento("e1", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO),
            evento("e2", "2026-11-24T11:00:00Z", "laptop", DATOS_DOS),
        ];
        let entrantes = vec![locales[0].clone()];

        let informe = fusionar(&locales, &entrantes);

        assert!(informe.nuevos.is_empty());
        assert_eq!(informe.ya_existian, 1);
        assert!(informe.rechazados.is_empty());
    }

    #[test]
    fn la_misma_parte_de_grupo_de_otro_dispositivo_ya_existia() {
        // La entregó el servidor con el mismo id en los dos dispositivos, pero
        // cada uno la selló con su momento y su dispositivo (Entrega 7).
        let local = evento(
            "parte-del-servidor",
            "2026-11-24T10:00:00Z",
            "laptop",
            DATOS_UNO,
        );
        let entrante = evento(
            "parte-del-servidor",
            "2026-11-24T15:00:00Z",
            "teléfono",
            DATOS_UNO,
        );

        let informe = fusionar(&[local], &[entrante]);

        assert!(informe.nuevos.is_empty());
        assert_eq!(informe.ya_existian, 1);
        assert!(informe.rechazados.is_empty());
    }

    #[test]
    fn un_evento_con_id_vacio_se_rechaza() {
        let entrante = evento("", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO);
        let informe = fusionar(&[], &[entrante]);

        assert!(informe.nuevos.is_empty());
        assert_eq!(informe.rechazados[0].motivo, MotivoRechazo::IdVacio);
    }

    #[test]
    fn un_id_repetido_dentro_del_lote_se_cuenta_una_sola_vez() {
        let primero = evento("e1", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO);
        let repetido = evento("e1", "2026-11-24T11:00:00Z", "laptop", DATOS_UNO);

        let informe = fusionar(&[], &[primero, repetido]);

        assert_eq!(informe.nuevos.len(), 1);
        assert_eq!(informe.ya_existian, 1);
    }

    #[test]
    fn el_json_se_compara_sin_importar_el_orden_de_las_claves() {
        let local = evento("e1", "2026-11-24T10:00:00Z", "laptop", r#"{"a":1,"b":2}"#);
        let entrante = evento("e1", "2026-11-24T11:00:00Z", "tel", r#"{"b":2,"a":1}"#);

        let informe = fusionar(&[local], &[entrante]);

        assert_eq!(informe.ya_existian, 1);
        assert!(informe.rechazados.is_empty());
    }

    #[test]
    fn ordenar_eventos_usa_momento_dispositivo_e_id() {
        let mut eventos = vec![
            evento("b", "2026-11-24T10:00:00Z", "teléfono", DATOS_UNO),
            evento("a", "2026-11-24T10:00:00Z", "laptop", DATOS_DOS),
            evento("c", "2026-11-24T09:00:00Z", "laptop", DATOS_UNO),
            evento("a", "2026-11-24T10:00:00Z", "laptop", DATOS_UNO),
        ];

        ordenar_eventos(&mut eventos);

        let claves: Vec<_> = eventos.iter().map(clave_orden).collect();
        let mut esperadas = claves.clone();
        esperadas.sort();
        assert_eq!(claves, esperadas);
        assert_eq!(eventos[0].id, "c");
        assert_eq!(eventos[1].dispositivo_origen, "laptop");
    }
}
