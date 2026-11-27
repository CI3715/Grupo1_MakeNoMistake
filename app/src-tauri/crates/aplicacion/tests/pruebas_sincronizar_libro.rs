//! Pruebas de sincronizar el libro entre dispositivos (Entrega 8: ESC-03, RF-INT-03/04, RNF-10).
//! El destino es un transporte falso en memoria que comparten los dispositivos.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use aplicacion::sincronizar_libro::{
    ErrorSincronizacion, EstadoSincronizacion, InformeSincronizacion, sincronizar,
};
use aplicacion::transporte::{ErrorTransporte, TransporteSincronizacion};
use aplicacion::{
    CasosDeUso, Cifrador, DispositivoFijo, ErrorRespaldo, GeneradorIdentificadores, NuevaCuenta,
    RelojFijo, RepositorioEnMemoria,
};

const FRASE: &str = "una frase de paso larga";
const AHORA: &str = "2026-11-26T12:00:00Z";

/// Cifrador de mentira que sí distingue la frase: lleva la frase al inicio del archivo.
struct CifradorConFrase;

impl Cifrador for CifradorConFrase {
    fn cifrar(&self, frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        let mut salida = frase.as_bytes().to_vec();
        salida.push(0);
        salida.extend(claro.iter().map(|b| b ^ 0x5A));
        Ok(salida)
    }
    fn descifrar(&self, frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        let prefijo = [frase.as_bytes(), &[0]].concat();
        match archivo.strip_prefix(prefijo.as_slice()) {
            Some(resto) => Ok(resto.iter().map(|b| b ^ 0x5A).collect()),
            None => Err(ErrorRespaldo::FraseIncorrectaOArchivoDanado),
        }
    }
}

/// Identificadores con prefijo propio: dos dispositivos reales nunca repiten ids.
struct Prefijo {
    prefijo: String,
    siguiente: u32,
}

impl GeneradorIdentificadores for Prefijo {
    fn nuevo_id(&mut self) -> String {
        self.siguiente += 1;
        format!("{}-{:04}", self.prefijo, self.siguiente)
    }
}

/// Lotes del destino: dispositivo → lote → contenido.
type LotesPorDispositivo = BTreeMap<String, BTreeMap<String, Vec<u8>>>;

/// El destino compartido: dispositivo → lote → contenido.
#[derive(Clone, Default)]
struct Red {
    lotes: Arc<Mutex<LotesPorDispositivo>>,
    caida: Arc<AtomicBool>,
}

struct TransporteFalso {
    dispositivo: String,
    red: Red,
    lecturas: Arc<AtomicUsize>,
}

impl TransporteSincronizacion for TransporteFalso {
    fn dispositivo_local(&self) -> &str {
        &self.dispositivo
    }

    fn traer(&mut self) -> Result<(), ErrorTransporte> {
        if self.red.caida.load(Ordering::SeqCst) {
            Err(ErrorTransporte::SinConexion)
        } else {
            Ok(())
        }
    }

    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte> {
        let todo = self.red.lotes.lock().unwrap();
        Ok(todo
            .iter()
            .filter(|(_, lotes)| !lotes.is_empty())
            .map(|(d, _)| d.clone())
            .collect())
    }

    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        let todo = self.red.lotes.lock().unwrap();
        Ok(todo
            .get(dispositivo)
            .map(|lotes| lotes.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte> {
        self.lecturas.fetch_add(1, Ordering::SeqCst);
        let todo = self.red.lotes.lock().unwrap();
        todo.get(dispositivo)
            .and_then(|lotes| lotes.get(lote))
            .cloned()
            .ok_or(ErrorTransporte::LoteNoExiste {
                dispositivo: dispositivo.to_string(),
                lote: lote.to_string(),
            })
    }

    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte> {
        let mut todo = self.red.lotes.lock().unwrap();
        let propios = todo.entry(self.dispositivo.clone()).or_default();
        if propios.contains_key(lote) {
            return Err(ErrorTransporte::LoteYaExiste {
                dispositivo: self.dispositivo.clone(),
                lote: lote.to_string(),
            });
        }
        propios.insert(lote.to_string(), contenido.to_vec());
        Ok(())
    }

    fn publicar(&mut self) -> Result<(), ErrorTransporte> {
        Ok(())
    }
}

/// Una instalación de la app: su libro, su transporte y lo que recuerda de la sincronización.
struct Dispositivo {
    casos: CasosDeUso,
    transporte: TransporteFalso,
    estado: EstadoSincronizacion,
    lecturas: Arc<AtomicUsize>,
}

impl Dispositivo {
    fn nuevo(id: &str, prefijo: &str, red: &Red) -> Dispositivo {
        let lecturas = Arc::new(AtomicUsize::new(0));
        Dispositivo {
            casos: CasosDeUso::nuevo(
                Box::new(RepositorioEnMemoria::nuevo()),
                Box::new(RelojFijo::nuevo(AHORA)),
                Box::new(Prefijo {
                    prefijo: prefijo.to_string(),
                    siguiente: 0,
                }),
                Box::new(DispositivoFijo::nuevo(id)),
            ),
            transporte: TransporteFalso {
                dispositivo: id.to_string(),
                red: red.clone(),
                lecturas: Arc::clone(&lecturas),
            },
            estado: EstadoSincronizacion::default(),
            lecturas,
        }
    }

    fn crear_cuenta(&mut self, nombre: &str) {
        self.casos
            .crear_cuenta(NuevaCuenta {
                nombre: nombre.to_string(),
                tipo: "efectivo".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "10.00".to_string(),
            })
            .unwrap();
    }

    /// Sincroniza y, solo si sale bien, recuerda el estado nuevo (como hará la app).
    fn sincronizar(&mut self, frase: &str) -> Result<InformeSincronizacion, ErrorSincronizacion> {
        let (informe, estado) = sincronizar(
            &mut self.casos,
            &mut self.transporte,
            &CifradorConFrase,
            frase,
            AHORA,
            &self.estado,
        )?;
        self.estado = estado;
        Ok(informe)
    }

    fn ids(&self) -> BTreeSet<String> {
        self.casos
            .eventos_del_libro()
            .unwrap()
            .iter()
            .map(|e| e.id.clone())
            .collect()
    }

    fn saldos(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .casos
            .listar_cuentas()
            .unwrap()
            .cuentas
            .iter()
            .map(|c| (c.id.clone(), c.saldo_actual.clone()))
            .collect();
        v.sort();
        v
    }

    fn lecturas(&self) -> usize {
        self.lecturas.load(Ordering::SeqCst)
    }
}

/// (enviados, recibidos) de un informe, como en la tabla del ejemplo 2.6.
fn resumen(informe: &InformeSincronizacion) -> (usize, usize) {
    (informe.enviados, informe.recibidos)
}

/// El ejemplo de convergencia de beto, en el orden que se pida (sección 2.6).
fn escenario_de_beto(telefono_primero: bool) {
    let red = Red::default();
    let mut laptop = Dispositivo::nuevo("laptop", "lap", &red);
    let mut telefono = Dispositivo::nuevo("telefono", "tel", &red);

    // Parten del mismo libro: la laptop lo sube y el teléfono lo baja.
    for nombre in ["Efectivo", "Banco", "Ahorros"] {
        laptop.crear_cuenta(nombre);
    }
    assert_eq!(resumen(&laptop.sincronizar(FRASE).unwrap()), (3, 0));
    assert_eq!(resumen(&telefono.sincronizar(FRASE).unwrap()), (0, 3));
    assert_eq!(laptop.ids(), telefono.ids());

    // Sin conexión, cada uno registra lo suyo.
    laptop.crear_cuenta("Almuerzo");
    laptop.crear_cuenta("Cafe");
    telefono.crear_cuenta("Ingreso");
    telefono.crear_cuenta("Gasto");

    {
        let (primero, segundo) = if telefono_primero {
            (&mut telefono, &mut laptop)
        } else {
            (&mut laptop, &mut telefono)
        };
        assert_eq!(resumen(&primero.sincronizar(FRASE).unwrap()), (2, 0));
        assert_eq!(resumen(&segundo.sincronizar(FRASE).unwrap()), (2, 2));
        assert_eq!(resumen(&primero.sincronizar(FRASE).unwrap()), (0, 2));
    }

    // Mismos eventos y mismos saldos, sin perder ni duplicar nada.
    assert_eq!(laptop.ids().len(), 7);
    assert_eq!(laptop.ids(), telefono.ids());
    assert_eq!(laptop.saldos(), telefono.saldos());

    // Repetir no cambia nada y no vuelve a leer ningún lote (RNF-10).
    let lecturas = (laptop.lecturas(), telefono.lecturas());
    assert_eq!(resumen(&laptop.sincronizar(FRASE).unwrap()), (0, 0));
    assert_eq!(resumen(&telefono.sincronizar(FRASE).unwrap()), (0, 0));
    assert_eq!((laptop.lecturas(), telefono.lecturas()), lecturas);
    assert_eq!(laptop.ids().len(), 7);
}

#[test]
fn convergen_si_sincroniza_primero_la_laptop() {
    escenario_de_beto(false);
}

#[test]
fn convergen_si_sincroniza_primero_el_telefono() {
    escenario_de_beto(true);
}

#[test]
fn sin_conexion_no_pierde_nada_y_lo_pendiente_sale_despues() {
    let red = Red::default();
    let mut laptop = Dispositivo::nuevo("laptop", "lap", &red);
    let mut telefono = Dispositivo::nuevo("telefono", "tel", &red);
    laptop.crear_cuenta("Efectivo");

    red.caida.store(true, Ordering::SeqCst);
    let antes = laptop.estado.clone();
    let error = laptop.sincronizar(FRASE).unwrap_err();
    assert_eq!(
        error,
        ErrorSincronizacion::Transporte(ErrorTransporte::SinConexion)
    );
    assert_eq!(laptop.estado, antes);
    assert_eq!(laptop.ids().len(), 1);

    red.caida.store(false, Ordering::SeqCst);
    assert_eq!(resumen(&laptop.sincronizar(FRASE).unwrap()), (1, 0));
    assert_eq!(resumen(&telefono.sincronizar(FRASE).unwrap()), (0, 1));
}

#[test]
fn frase_incorrecta_no_importa_nada_ni_cambia_el_estado() {
    let red = Red::default();
    let mut laptop = Dispositivo::nuevo("laptop", "lap", &red);
    let mut telefono = Dispositivo::nuevo("telefono", "tel", &red);
    laptop.crear_cuenta("Efectivo");
    laptop.sincronizar(FRASE).unwrap();

    let error = telefono.sincronizar("otra frase distinta").unwrap_err();
    assert_eq!(
        error,
        ErrorSincronizacion::Respaldo(ErrorRespaldo::FraseIncorrectaOArchivoDanado)
    );
    assert!(telefono.ids().is_empty());
    assert_eq!(telefono.estado, EstadoSincronizacion::default());
}

#[test]
fn mismo_id_con_contenido_distinto_se_rechaza_y_el_resto_se_aplica() {
    let red = Red::default();
    // Mismo prefijo a propósito: los dos libros generan los mismos ids.
    let mut laptop = Dispositivo::nuevo("laptop", "x", &red);
    let mut telefono = Dispositivo::nuevo("telefono", "x", &red);
    laptop.crear_cuenta("Uno");
    laptop.crear_cuenta("Dos");
    telefono.crear_cuenta("Tres");
    laptop.sincronizar(FRASE).unwrap();

    let antes = telefono.ids();
    let informe = telefono.sincronizar(FRASE).unwrap();
    assert_eq!(informe.rechazados.len(), 1);
    assert!(informe.rechazados[0].motivo.contains("contenido distinto"));
    assert_eq!(informe.recibidos, 1);
    assert!(antes.is_subset(&telefono.ids()));
}
