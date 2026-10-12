use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const URL_POR_DEFECTO: &str = "http://localhost:8000";

#[derive(Debug, Deserialize)]
struct ServidorPingResponse {
    mensaje: String,
    version: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
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

pub fn url_servidor() -> String {
    std::env::var("CUENTAS_CLARAS_URL").unwrap_or_else(|_| URL_POR_DEFECTO.to_string())
}

pub async fn hacer_ping(base: &str) -> PingResult {
    hacer_ping_con_timeout(base, Duration::from_secs(5)).await
}

async fn hacer_ping_con_timeout(base: &str, timeout: Duration) -> PingResult {
    let url = format!("{}/ping", base.trim_end_matches('/'));

    let cliente = match reqwest::Client::builder().timeout(timeout).build() {
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

#[tauri::command]
pub async fn ping_servidor() -> PingResult {
    hacer_ping(&url_servidor()).await
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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

    #[tokio::test]
    async fn puerto_cerrado_da_sin_conexion() {
        let r = hacer_ping("http://127.0.0.1:1").await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("sin_conexion"));
    }

    #[tokio::test]
    async fn url_invalida_da_error_claro() {
        let r = hacer_ping("esto no es una url").await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("url_invalida"));
    }

    #[tokio::test]
    async fn respuesta_valida_da_exito() {
        let base = servidor_falso(http("200 OK", JSON_VALIDO), Duration::ZERO).await;
        let r = hacer_ping(&base).await;
        assert!(r.exito);
        assert_eq!(r.mensaje, "pong");
        assert_eq!(r.version.as_deref(), Some("0.1.0"));
        assert_eq!(r.timestamp.as_deref(), Some("2026-10-08T03:03:04Z"));
        assert_eq!(r.error, None);
    }

    #[tokio::test]
    async fn error_500_da_error_del_servidor() {
        let base = servidor_falso(http("500 Internal Server Error", "{}"), Duration::ZERO).await;
        let r = hacer_ping(&base).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("error_del_servidor"));
    }

    #[tokio::test]
    async fn formato_roto_da_respuesta_inesperada() {
        let base = servidor_falso(http("200 OK", "hola"), Duration::ZERO).await;
        let r = hacer_ping(&base).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("respuesta_inesperada"));
    }

    #[tokio::test]
    async fn servidor_lento_da_tiempo_agotado() {
        let base = servidor_falso(http("200 OK", JSON_VALIDO), Duration::from_millis(500)).await;
        let r = hacer_ping_con_timeout(&base, Duration::from_millis(100)).await;
        assert!(!r.exito);
        assert_eq!(r.error.as_deref(), Some("tiempo_agotado"));
    }
}
