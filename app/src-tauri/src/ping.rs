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
    let url = format!("{}/ping", base.trim_end_matches('/'));

    let cliente = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
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
}
