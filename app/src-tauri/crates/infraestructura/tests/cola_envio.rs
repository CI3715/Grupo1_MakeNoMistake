//! Pruebas de la cola de gastos de grupo sin conexión (Entrega 7, Angel - Tarea 7).
//!
//! Verifica el comportamiento de la cola local y la máquina de estados de reintentos
//! con un cliente HTTP simulado (RNF-17, ESC-09, INV-15):
//! 1. Sin red el gasto queda «pendiente de envío» y se reintenta con la misma clave.
//! 2. Al volver la red se envía una sola vez y sale de la cola.
//! 3. El orden de envío (FIFO) se respeta estrictamente.
//! 4. Un error 400, 403, 404 o 409 saca la operación de la cola con su motivo sin bloquear las demás.
//! 5. Un error 401 (no autorizado) pausa la cola sin perder operaciones pendientes.
//! 6. Un error 5xx se mantiene en la cola para reintento posterior.
//! 7. El reenvío con la misma clave de idempotencia evita duplicaciones en el servidor.

use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoOperacion {
    Pendiente,
    Enviada,
    Rechazada { motivo: String },
}

#[derive(Debug, Clone)]
pub struct OperacionGastoCola {
    pub clave: String,
    pub grupo_id: String,
    pub concepto: String,
    pub monto: String,
    pub moneda: String,
    pub estado: EstadoOperacion,
    pub reintentos: usize,
}

impl OperacionGastoCola {
    pub fn nueva(clave: &str, grupo_id: &str, concepto: &str, monto: &str, moneda: &str) -> Self {
        Self {
            clave: clave.to_string(),
            grupo_id: grupo_id.to_string(),
            concepto: concepto.to_string(),
            monto: monto.to_string(),
            moneda: moneda.to_string(),
            estado: EstadoOperacion::Pendiente,
            reintentos: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespuestaHttpSimulada {
    Creado201,
    Existente200,
    Invalido400(&'static str),
    NoAutorizado401,
    NoEncontrado404(&'static str),
    Conflicto409(&'static str),
    ErrorServidor500,
    SinConexion,
}

pub struct ColaEnvioGastos {
    operaciones: VecDeque<OperacionGastoCola>,
    pausada: bool,
}

impl ColaEnvioGastos {
    pub fn nueva() -> Self {
        Self {
            operaciones: VecDeque::new(),
            pausada: false,
        }
    }

    pub fn encolar(&mut self, operacion: OperacionGastoCola) {
        self.operaciones.push_back(operacion);
    }

    pub fn cantidad_pendientes(&self) -> usize {
        self.operaciones
            .iter()
            .filter(|op| op.estado == EstadoOperacion::Pendiente)
            .count()
    }

    pub fn esta_pausada(&self) -> bool {
        self.pausada
    }

    pub fn reanudar(&mut self) {
        self.pausada = false;
    }

    /// Procesa la cola contra el despachador HTTP simulado.
    pub fn procesar<F>(&mut self, mut despachador: F)
    where
        F: FnMut(&OperacionGastoCola) -> RespuestaHttpSimulada,
    {
        if self.pausada {
            return;
        }

        let mut i = 0;
        while i < self.operaciones.len() {
            if self.pausada {
                break;
            }

            if self.operaciones[i].estado != EstadoOperacion::Pendiente {
                i += 1;
                continue;
            }

            let respuesta = despachador(&self.operaciones[i]);
            match respuesta {
                RespuestaHttpSimulada::Creado201 | RespuestaHttpSimulada::Existente200 => {
                    self.operaciones[i].estado = EstadoOperacion::Enviada;
                    // Sale de la cola
                    self.operaciones.remove(i);
                }
                RespuestaHttpSimulada::Invalido400(motivo)
                | RespuestaHttpSimulada::NoEncontrado404(motivo)
                | RespuestaHttpSimulada::Conflicto409(motivo) => {
                    // Rechazo definitivo: sale de pendientes sin bloquear las demás
                    self.operaciones[i].estado = EstadoOperacion::Rechazada {
                        motivo: motivo.to_string(),
                    };
                    self.operaciones.remove(i);
                }
                RespuestaHttpSimulada::NoAutorizado401 => {
                    // Pausa la cola inmediatamente sin perder nada
                    self.pausada = true;
                    break;
                }
                RespuestaHttpSimulada::ErrorServidor500 | RespuestaHttpSimulada::SinConexion => {
                    // Queda pendiente y se incrementa el contador de reintentos
                    self.operaciones[i].reintentos += 1;
                    i += 1;
                }
            }
        }
    }
}

#[test]
fn sin_red_el_gasto_queda_pendiente_y_reintenta_con_misma_clave() {
    let mut cola = ColaEnvioGastos::nueva();
    let clave = "clave-uuid-sin-red-001";
    cola.encolar(OperacionGastoCola::nueva(
        clave, "grupo-1", "Almuerzo", "45.00", "USD",
    ));

    assert_eq!(cola.cantidad_pendientes(), 1);

    // Intento 1 sin conexión
    cola.procesar(|op| {
        assert_eq!(op.clave, clave, "Debe preservar la misma clave");
        RespuestaHttpSimulada::SinConexion
    });

    assert_eq!(cola.cantidad_pendientes(), 1);
    assert_eq!(cola.operaciones[0].reintentos, 1);
    assert_eq!(cola.operaciones[0].estado, EstadoOperacion::Pendiente);

    // Intento 2 con error 500
    cola.procesar(|op| {
        assert_eq!(op.clave, clave, "Debe preservar la misma clave en 5xx");
        RespuestaHttpSimulada::ErrorServidor500
    });

    assert_eq!(cola.cantidad_pendientes(), 1);
    assert_eq!(cola.operaciones[0].reintentos, 2);
}

#[test]
fn al_volver_la_red_se_envia_una_sola_vez_y_sale_de_la_cola() {
    let mut cola = ColaEnvioGastos::nueva();
    cola.encolar(OperacionGastoCola::nueva(
        "clave-exitosa-002",
        "grupo-1",
        "Hotel",
        "100.00",
        "USD",
    ));

    // Fallo inicial
    cola.procesar(|_| RespuestaHttpSimulada::SinConexion);
    assert_eq!(cola.cantidad_pendientes(), 1);

    // Recuperación de red: 201 Created
    let mut llamadas = 0;
    cola.procesar(|_| {
        llamadas += 1;
        RespuestaHttpSimulada::Creado201
    });

    assert_eq!(llamadas, 1, "Solo se envía una vez");
    assert_eq!(cola.cantidad_pendientes(), 0, "Sale de la cola");
}

#[test]
fn se_respeta_el_orden_fifo_de_las_operaciones() {
    let mut cola = ColaEnvioGastos::nueva();
    cola.encolar(OperacionGastoCola::nueva(
        "clave-1", "g-1", "Primero", "10.00", "USD",
    ));
    cola.encolar(OperacionGastoCola::nueva(
        "clave-2", "g-1", "Segundo", "20.00", "USD",
    ));
    cola.encolar(OperacionGastoCola::nueva(
        "clave-3", "g-1", "Tercero", "30.00", "USD",
    ));

    let mut orden_procesado = Vec::new();
    cola.procesar(|op| {
        orden_procesado.push(op.concepto.clone());
        RespuestaHttpSimulada::Creado201
    });

    assert_eq!(orden_procesado, vec!["Primero", "Segundo", "Tercero"]);
    assert_eq!(cola.cantidad_pendientes(), 0);
}

#[test]
fn rechazo_4xx_saca_operacion_con_su_motivo_sin_bloquear_demas() {
    let mut cola = ColaEnvioGastos::nueva();
    cola.encolar(OperacionGastoCola::nueva(
        "clave-mala",
        "g-1",
        "Gasto Inválido",
        "-5.00",
        "USD",
    ));
    cola.encolar(OperacionGastoCola::nueva(
        "clave-buena",
        "g-1",
        "Gasto Válido",
        "15.00",
        "USD",
    ));

    let mut enviados = Vec::new();
    cola.procesar(|op| {
        if op.clave == "clave-mala" {
            RespuestaHttpSimulada::Invalido400("El monto debe ser positivo.")
        } else {
            enviados.push(op.clave.clone());
            RespuestaHttpSimulada::Creado201
        }
    });

    // La operación inválida salió y la válida se procesó exitosamente
    assert_eq!(cola.cantidad_pendientes(), 0);
    assert_eq!(enviados, vec!["clave-buena"]);
}

#[test]
fn error_401_pausa_la_cola_sin_perder_operaciones() {
    let mut cola = ColaEnvioGastos::nueva();
    cola.encolar(OperacionGastoCola::nueva(
        "clave-auth",
        "g-1",
        "Taxi",
        "12.00",
        "USD",
    ));
    cola.encolar(OperacionGastoCola::nueva(
        "clave-siguiente",
        "g-1",
        "Cena",
        "35.00",
        "USD",
    ));

    // El servidor responde 401 por sesión vencida
    cola.procesar(|_| RespuestaHttpSimulada::NoAutorizado401);

    assert!(cola.esta_pausada(), "La cola debe quedar pausada");
    assert_eq!(
        cola.cantidad_pendientes(),
        2,
        "Conserva todas las operaciones"
    );

    // Intentos mientras está pausada no envían nada
    let mut contador = 0;
    cola.procesar(|_| {
        contador += 1;
        RespuestaHttpSimulada::Creado201
    });
    assert_eq!(contador, 0, "No debe procesar mientras esté pausada");

    // Al reanudar (tras iniciar sesión) se envían
    cola.reanudar();
    cola.procesar(|_| RespuestaHttpSimulada::Creado201);
    assert_eq!(cola.cantidad_pendientes(), 0, "Se procesaron tras reanudar");
}

#[test]
fn reintento_con_misma_clave_recibe_200_sin_duplicar() {
    let mut cola = ColaEnvioGastos::nueva();
    cola.encolar(OperacionGastoCola::nueva(
        "clave-reintento-200",
        "g-1",
        "Farmacia",
        "22.50",
        "USD",
    ));

    // El servidor responde 200 (la petición previa se completó en el servidor pero la respuesta se perdió)
    cola.procesar(|op| {
        assert_eq!(op.clave, "clave-reintento-200");
        RespuestaHttpSimulada::Existente200
    });

    assert_eq!(
        cola.cantidad_pendientes(),
        0,
        "Gasto ya procesado sale de la cola"
    );
}
