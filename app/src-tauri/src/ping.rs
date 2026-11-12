//! Comando `ping_servidor`: comprueba que el servidor responde y mide la latencia.
//!
//! Se organiza con un puerto (`ClientePing`) y dos adaptadores: `ClienteHttp`, el real
//! (reqwest), y un cliente falso que solo existe en las pruebas.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::{Duration, Instant};

const URL_POR_DEFECTO: &str = "http://localhost:8000";

/// Resultado que recibe la interfaz. Se devuelve siempre, tanto si funciona como si falla.
#[derive(Debug, Clone, Serialize)]
pub struct PingResult {
    pub exito: bool,
    pub mensaje: String,
    pub version: Option<String>,
    pub timestamp: Option<String>,
    pub latencia_ms: u64,
    pub error: Option<String>,
}

impl PingResult {
    fn fallo(mensaje: &str, causa: &str, latencia_ms: u64) -> Self {
        PingResult {
            exito: false,
            mensaje: mensaje.to_string(),
            version: None,
            timestamp: None,
            latencia_ms,
            error: Some(causa.to_string()),
        }
    }
}

/// PUERTO: lo que debe saber hacer cualquier cliente que consulte al servidor.
pub trait ClientePing {
    fn ping(&self) -> impl Future<Output = PingResult> + Send;
}

/// Caso de uso: verificar la conexión con el cliente que le den.
pub async fn verificar_conexion<C: ClientePing>(cliente: &C) -> PingResult {
    cliente.ping().await
}

/// Forma del JSON que responde el servidor en GET /ping.
#[derive(Debug, Deserialize)]
struct ServidorPingResponse {
    mensaje: String,
    version: String,
    timestamp: String,
}

/// ADAPTADOR REAL: consulta al servidor por HTTP.
pub struct ClienteHttp {
    base: String,
    timeout: Duration,
}

impl ClienteHttp {
    pub fn new(base: &str, timeout: Duration) -> Self {
        ClienteHttp {
            base: base.to_string(),
            timeout,
        }
    }
}

impl ClientePing for ClienteHttp {
    async fn ping(&self) -> PingResult {
        let url = format!("{}/ping", self.base.trim_end_matches('/'));

        let cliente = match reqwest::Client::builder().timeout(self.timeout).build() {
            Ok(c) => c,
            Err(_) => return PingResult::fallo("No se pudo preparar la conexión.", "cliente", 0),
        };

        let inicio = Instant::now();
        let respuesta = cliente.get(&url).send().await;
        let latencia_ms = inicio.elapsed().as_millis() as u64;

        let res = match respuesta {
            Ok(r) => r,
            Err(e) => {
                let (msg, causa) = if e.is_builder() {
                    (
                        "La dirección del servidor no es válida. Revisa la configuración.",
                        "url_invalida",
                    )
                } else if e.is_timeout() {
                    (
                        "El servidor tardó demasiado en responder. Intenta de nuevo.",
                        "tiempo_agotado",
                    )
                } else {
                    (
                        "No se pudo conectar con el servidor. Verifica que esté encendido.",
                        "sin_conexion",
                    )
                };
                return PingResult::fallo(msg, causa, latencia_ms);
            }
        };

        if !res.status().is_success() {
            return PingResult::fallo(
                "El servidor respondió con un error.",
                "error_del_servidor",
                latencia_ms,
            );
        }

        match res.json::<ServidorPingResponse>().await {
            Ok(data) => PingResult {
                exito: true,
                mensaje: data.mensaje,
                version: Some(data.version),
                timestamp: Some(data.timestamp),
                latencia_ms,
                error: None,
            },
            Err(_) => PingResult::fallo(
                "El servidor respondió con un formato inesperado.",
                "respuesta_inesperada",
                latencia_ms,
            ),
        }
    }
}

pub fn url_servidor() -> String {
    std::env::var("CUENTAS_CLARAS_URL").unwrap_or_else(|_| URL_POR_DEFECTO.to_string())
}

#[tauri::command]
pub async fn ping_servidor() -> PingResult {
    let cliente = ClienteHttp::new(&url_servidor(), Duration::from_secs(5));
    verificar_conexion(&cliente).await
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // ---------- Adaptador de prueba y contrato ----------

    /// ADAPTADOR DE PRUEBA: devuelve el resultado que se le indique, sin red.
    struct ClienteFalso {
        resultado: PingResult,
    }

    impl ClientePing for ClienteFalso {
        async fn ping(&self) -> PingResult {
            self.resultado.clone()
        }
    }

    /// CONTRATO: reglas que TODA implementación del puerto debe cumplir.
    fn verificar_contrato(r: &PingResult) {
        if r.exito {
            assert!(r.version.is_some(), "un éxito debe traer versión");
            assert!(r.timestamp.is_some(), "un éxito debe traer timestamp");
            assert!(r.error.is_none(), "un éxito no debe traer código de error");
        } else {
            assert!(r.version.is_none(), "un fallo no debe traer versión");
            assert!(r.timestamp.is_none(), "un fallo no debe traer timestamp");
            assert!(r.error.is_some(), "un fallo debe traer código de error");
            assert!(
                !r.mensaje.is_empty(),
                "un fallo debe traer un mensaje legible"
            );
        }
    }

    fn exito_de_ejemplo() -> PingResult {
        PingResult {
            exito: true,
            mensaje: "pong".to_string(),
            version: Some("0.1.0".to_string()),
            timestamp: Some("2026-10-08T03:03:04Z".to_string()),
            latencia_ms: 3,
            error: None,
        }
    }

    #[tokio::test]
    async fn el_caso_de_uso_devuelve_lo_que_responde_el_cliente() {
        let cliente = ClienteFalso {
            resultado: exito_de_ejemplo(),
        };
        let r = verificar_conexion(&cliente).await;
        assert!(r.exito);
        assert_eq!(r.mensaje, "pong");
        assert_eq!(r.latencia_ms, 3);
    }

    #[tokio::test]
    async fn el_cliente_falso_cumple_el_contrato() {
        let exito = ClienteFalso {
            resultado: exito_de_ejemplo(),
        };
        verificar_contrato(&exito.ping().await);

        let fallo = ClienteFalso {
            resultado: PingResult::fallo("No se pudo conectar con el servidor.", "sin_conexion", 0),
        };
        verificar_contrato(&fallo.ping().await);
    }

    // ---------- Adaptador real, contra un servidor falso ----------

    // Arma una respuesta HTTP completa con su largo calculado.
    fn http(estado: &str, cuerpo: &str) -> String {
        format!(
            "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
    }

    // Levanta un servidor falso que atiende UNA petición, espera `demora`
    // y responde con `respuesta`. Devuelve su dirección base.
    async fn servidor_falso(respuesta: String, demora: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer).await;
                tokio::time::sleep(demora).await;
                let _ = socket.write_all(respuesta.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        format!("http://{direccion}")
    }

    const JSON_VALIDO: &str =
        r#"{"estado":"ok","mensaje":"pong","version":"0.1.0","timestamp":"2026-10-08T03:03:04Z"}"#;

    // Hace el ping con el cliente real y comprueba el contrato en cada caso.
    async fn ping_a(base: &str, timeout: Duration) -> PingResult {
        let r = ClienteHttp::new(base, timeout).ping().await;
        verificar_contrato(&r);
        r
    }

    #[tokio::test]
    async fn puerto_cerrado_da_sin_conexion() {
        let r = ping_a("http://127.0.0.1:1", Duration::from_secs(5)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("sin_conexion"));
    }

    #[tokio::test]
    async fn url_invalida_da_error_claro() {
        let r = ping_a("esto no es una url", Duration::from_secs(5)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("url_invalida"));
    }

    #[tokio::test]
    async fn respuesta_valida_da_exito() {
        let base = servidor_falso(http("200 OK", JSON_VALIDO), Duration::ZERO).await;
        let r = ping_a(&base, Duration::from_secs(5)).await;
        assert!(r.exito);
        assert_eq!(r.mensaje, "pong");
        assert_eq!(r.version.as_deref(), Some("0.1.0"));
        assert_eq!(r.timestamp.as_deref(), Some("2026-10-08T03:03:04Z"));
        assert_eq!(r.error, None);
    }

    #[tokio::test]
    async fn error_500_da_error_del_servidor() {
        let base = servidor_falso(http("500 Internal Server Error", "{}"), Duration::ZERO).await;
        let r = ping_a(&base, Duration::from_secs(5)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("error_del_servidor"));
    }

    #[tokio::test]
    async fn formato_roto_da_respuesta_inesperada() {
        let base = servidor_falso(http("200 OK", "hola"), Duration::ZERO).await;
        let r = ping_a(&base, Duration::from_secs(5)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("respuesta_inesperada"));
    }

    #[tokio::test]
    async fn servidor_lento_da_tiempo_agotado() {
        let base = servidor_falso(http("200 OK", JSON_VALIDO), Duration::from_millis(500)).await;
        let r = ping_a(&base, Duration::from_millis(100)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("tiempo_agotado"));
    }
}
