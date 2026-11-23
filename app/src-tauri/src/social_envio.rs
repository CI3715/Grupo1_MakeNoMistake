//! Envío de un gasto de grupo que estaba en la cola (Entrega 7).
//!
//! Solo habla con el servidor: manda el gasto con su clave de idempotencia y dice qué
//! pasó. Guardar o quitar de la cola lo hace otro módulo. Si el mismo gasto se manda
//! dos veces con la misma clave, el servidor no lo duplica.

use crate::dto_social::GastoGrupoDto;
use crate::sesion::Almacen;
use crate::social_cliente::{ClienteSocial, Peticion, codificar, token_vigente};

#[derive(Debug, Clone)]
pub enum ResultadoEnvio {
    // El servidor lo aceptó.
    Enviado(GastoGrupoDto),
    // El servidor lo rechazó; no tiene sentido reintentar. Trae el motivo.
    Rechazado(String),
    // No se pudo (sin red, sin sesión, sesión vencida o falla del servidor): se reintenta luego.
    Reintentar,
}

fn motivo_de(cuerpo: &str, codigo: u16) -> String {
    serde_json::from_str::<serde_json::Value>(cuerpo)
        .ok()
        .and_then(|v| v["errores"][0]["mensaje"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("El servidor rechazó el gasto (código {codigo})."))
}

// Manda un gasto ya armado (`cuerpo` es el JSON) con su clave de idempotencia.
pub async fn enviar_gasto<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    grupo_id: &str,
    cuerpo: &str,
    clave: &str,
) -> ResultadoEnvio {
    let Ok(valor) = serde_json::from_str::<serde_json::Value>(cuerpo) else {
        return ResultadoEnvio::Rechazado("El gasto guardado está dañado.".to_string());
    };
    let Ok(token) = token_vigente(almacen, ahora) else {
        return ResultadoEnvio::Reintentar;
    };
    let ruta = format!("/grupos/{}/gastos", codificar(grupo_id));
    let peticion = Peticion::post(&ruta, &valor).con_clave(clave);
    let Ok(respuesta) = cliente.enviar_peticion(&peticion, &token).await else {
        return ResultadoEnvio::Reintentar;
    };
    match respuesta.codigo {
        200 | 201 => match serde_json::from_str::<GastoGrupoDto>(&respuesta.cuerpo) {
            Ok(gasto) => ResultadoEnvio::Enviado(gasto),
            Err(_) => ResultadoEnvio::Reintentar,
        },
        400 | 403 | 404 | 409 => {
            ResultadoEnvio::Rechazado(motivo_de(&respuesta.cuerpo, respuesta.codigo))
        }
        _ => ResultadoEnvio::Reintentar,
    }
}
