//! Comandos de registro e inicio de sesión (Entrega 5), conectados al servidor.
//!
//! Dos puertos para poder probarlos sin red ni sistema operativo:
//! - `ClienteSesiones`: habla HTTP con el servidor (adaptador real: `ClienteHttp`).
//! - `Almacen`: guarda la sesión (adaptador real: `AlmacenSistema`, que usa el
//!   almacén de secretos del sistema y, si falla, un archivo local con permisos 0600).
//!
//! Ninguna estructura con contraseña o token deriva `Debug` y no se registra
//! en logs el cuerpo de ninguna petición (RNF-05).

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::dto::{
    ErrorCampo, ErroresDto, EstadoSesion, IniciarSesionDto, RegistroDto, SesionDto, UsuarioDto,
};

const URL_POR_DEFECTO: &str = "http://localhost:8000";
const VARIABLE_URL: &str = "CUENTAS_CLARAS_URL";
const TIEMPO_MAXIMO: Duration = Duration::from_secs(5);
const SERVICIO: &str = "cuentas-claras";
const USUARIO_ALMACEN: &str = "sesion";
const ARCHIVO_RESPALDO: &str = "sesion.json";

// ---------- Mensajes ----------

pub(crate) fn sin_campo(mensaje: &str) -> ErroresDto {
    ErroresDto {
        errores: vec![ErrorCampo {
            campo: None,
            mensaje: mensaje.to_string(),
        }],
    }
}

pub(crate) fn sin_conexion() -> ErroresDto {
    sin_campo(
        "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión.",
    )
}

pub(crate) fn inesperado() -> ErroresDto {
    sin_campo("El servidor respondió algo inesperado. Intenta de nuevo más tarde.")
}

pub(crate) fn sesion_vencida() -> ErroresDto {
    sin_campo("Tu sesión venció, inicia sesión de nuevo.")
}

// ---------- Puerto HTTP ----------

/// Respuesta cruda del servidor: código y cuerpo.
pub struct Respuesta {
    pub codigo: u16,
    pub cuerpo: String,
}

/// No se pudo hablar con el servidor (apagado, sin red o tiempo agotado).
pub struct SinConexion;

pub trait ClienteSesiones {
    fn registrar(
        &self,
        datos: &RegistroDto,
    ) -> impl Future<Output = Result<Respuesta, SinConexion>> + Send;
    fn iniciar(
        &self,
        datos: &IniciarSesionDto,
    ) -> impl Future<Output = Result<Respuesta, SinConexion>> + Send;
    fn perfil(&self, token: &str) -> impl Future<Output = Result<Respuesta, SinConexion>> + Send;
}

/// ADAPTADOR REAL (reqwest). La URL base sale de `CUENTAS_CLARAS_URL`.
pub struct ClienteHttp {
    base: String,
}

impl ClienteHttp {
    pub fn desde_entorno() -> Self {
        let base = std::env::var(VARIABLE_URL)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| URL_POR_DEFECTO.to_string());
        ClienteHttp { base }
    }

    fn url(&self, ruta: &str) -> String {
        format!("{}{}", self.base.trim_end_matches('/'), ruta)
    }

    async fn enviar(&self, peticion: reqwest::RequestBuilder) -> Result<Respuesta, SinConexion> {
        let respuesta = peticion.send().await.map_err(|_| SinConexion)?;
        let codigo = respuesta.status().as_u16();
        let cuerpo = respuesta.text().await.map_err(|_| SinConexion)?;
        Ok(Respuesta { codigo, cuerpo })
    }

    fn cliente() -> Result<reqwest::Client, SinConexion> {
        reqwest::Client::builder()
            .timeout(TIEMPO_MAXIMO)
            .build()
            .map_err(|_| SinConexion)
    }
}

impl ClienteHttp {
    /// Petición con el token en el encabezado, para los módulos de amistades y grupos.
    pub async fn enviar_con_token(
        &self,
        metodo: &str,
        ruta: &str,
        token: &str,
        cuerpo: Option<&str>,
    ) -> Result<Respuesta, SinConexion> {
        let cliente = Self::cliente()?;
        let url = self.url(ruta);
        let base = match metodo {
            "GET" => cliente.get(url),
            "POST" => cliente.post(url),
            "DELETE" => cliente.delete(url),
            _ => return Err(SinConexion),
        };
        let mut peticion = base.bearer_auth(token);
        if let Some(texto) = cuerpo {
            peticion = peticion
                .header("Content-Type", "application/json")
                .body(texto.to_string());
        }
        self.enviar(peticion).await
    }
}

impl ClienteSesiones for ClienteHttp {
    async fn registrar(&self, datos: &RegistroDto) -> Result<Respuesta, SinConexion> {
        let p = Self::cliente()?.post(self.url("/usuarios")).json(datos);
        self.enviar(p).await
    }

    async fn iniciar(&self, datos: &IniciarSesionDto) -> Result<Respuesta, SinConexion> {
        let p = Self::cliente()?.post(self.url("/sesiones")).json(datos);
        self.enviar(p).await
    }

    async fn perfil(&self, token: &str) -> Result<Respuesta, SinConexion> {
        let p = Self::cliente()?
            .get(self.url("/usuarios/yo"))
            .bearer_auth(token);
        self.enviar(p).await
    }
}

// ---------- Puerto del almacén ----------

/// Lo que se guarda en el dispositivo. Sin `Debug`: lleva el token.
#[derive(Serialize, Deserialize, Clone)]
pub struct SesionGuardada {
    pub token: String,
    pub vence_en: String,
    pub nombre_visible: String,
    /// El servidor respondió 401 con este token: se trata como vencida.
    #[serde(default)]
    pub invalidada: bool,
}

pub trait Almacen {
    fn guardar(&self, sesion: &SesionGuardada) -> Result<(), String>;
    fn leer(&self) -> Option<SesionGuardada>;
    fn borrar(&self);
}

/// ADAPTADOR REAL: almacén de secretos del sistema con respaldo en archivo 0600
/// (deuda registrada en docs/deuda-tecnica.md).
pub struct AlmacenSistema {
    archivo: PathBuf,
}

impl AlmacenSistema {
    pub fn nuevo(carpeta_datos: &Path) -> Self {
        AlmacenSistema {
            archivo: carpeta_datos.join(ARCHIVO_RESPALDO),
        }
    }

    fn entrada() -> Result<keyring::v1::Entry, keyring::Error> {
        keyring::v1::Entry::new(SERVICIO, USUARIO_ALMACEN)
    }
}

fn escribir_privado(ruta: &Path, texto: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(carpeta) = ruta.parent() {
        std::fs::create_dir_all(carpeta)?;
    }
    let mut opciones = std::fs::OpenOptions::new();
    opciones.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opciones.mode(0o600);
    }
    opciones.open(ruta)?.write_all(texto.as_bytes())
}

impl Almacen for AlmacenSistema {
    fn guardar(&self, sesion: &SesionGuardada) -> Result<(), String> {
        let texto = serde_json::to_string(sesion).map_err(|_| "no se pudo serializar")?;
        let en_sistema = Self::entrada().and_then(|e| e.set_password(&texto));
        match en_sistema {
            Ok(()) => {
                let _ = std::fs::remove_file(&self.archivo);
                Ok(())
            }
            Err(_) => {
                log::warn!("Almacén de secretos no disponible: se usa el archivo local.");
                escribir_privado(&self.archivo, &texto)
                    .map_err(|_| "no se pudo escribir el respaldo".to_string())
            }
        }
    }

    fn leer(&self) -> Option<SesionGuardada> {
        let del_sistema = Self::entrada().and_then(|e| e.get_password()).ok();
        let texto = del_sistema.or_else(|| std::fs::read_to_string(&self.archivo).ok())?;
        serde_json::from_str(&texto).ok()
    }

    fn borrar(&self) {
        if let Ok(e) = Self::entrada() {
            let _ = e.delete_credential();
        }
        let _ = std::fs::remove_file(&self.archivo);
    }
}

// ---------- Hora y estado ----------

/// Hora actual en UTC con el mismo formato que `vence_en` (AAAA-MM-DDThh:mm:ssZ),
/// que se puede comparar como texto.
pub(crate) fn ahora_iso() -> String {
    let segundos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let anio = yoe + era * 400 + i64::from(mes <= 2);
    format!(
        "{anio:04}-{mes:02}-{dia:02}T{:02}:{:02}:{:02}Z",
        resto / 3_600,
        resto % 3_600 / 60,
        resto % 60
    )
}

pub(crate) fn estado_de(sesion: &SesionGuardada, ahora: &str) -> EstadoSesion {
    if sesion.invalidada || sesion.vence_en.as_str() <= ahora {
        EstadoSesion::Vencida
    } else {
        EstadoSesion::Vigente
    }
}

fn vista(sesion: &SesionGuardada, ahora: &str) -> SesionDto {
    SesionDto {
        estado: estado_de(sesion, ahora),
        nombre_visible: Some(sesion.nombre_visible.clone()),
        vence_en: Some(sesion.vence_en.clone()),
    }
}

// ---------- Traducción de respuestas ----------

#[derive(Deserialize)]
struct ErroresServidor {
    errores: Vec<ErrorServidor>,
}

#[derive(Deserialize)]
struct ErrorServidor {
    campo: Option<String>,
    mensaje: String,
}

#[derive(Deserialize)]
struct MensajeServidor {
    mensaje: String,
}

#[derive(Deserialize)]
struct TokenServidor {
    token: String,
    vence_en: String,
}

/// Convierte un 400, 401 o 409 del servidor en errores por campo o un mensaje general.
pub(crate) fn errores_del_servidor(r: &Respuesta) -> ErroresDto {
    if let Ok(e) = serde_json::from_str::<ErroresServidor>(&r.cuerpo) {
        return ErroresDto {
            errores: e
                .errores
                .into_iter()
                .map(|x| ErrorCampo {
                    campo: x.campo,
                    mensaje: x.mensaje,
                })
                .collect(),
        };
    }
    if let Ok(m) = serde_json::from_str::<MensajeServidor>(&r.cuerpo) {
        return sin_campo(&m.mensaje);
    }
    inesperado()
}

fn leer_usuario(r: &Respuesta) -> Result<UsuarioDto, ErroresDto> {
    serde_json::from_str(&r.cuerpo).map_err(|_| inesperado())
}

// ---------- Casos de uso ----------

pub async fn registrar<C: ClienteSesiones>(
    cliente: &C,
    datos: &RegistroDto,
) -> Result<UsuarioDto, ErroresDto> {
    let r = cliente.registrar(datos).await.map_err(|_| sin_conexion())?;
    match r.codigo {
        201 => leer_usuario(&r),
        400 | 409 => Err(errores_del_servidor(&r)),
        _ => Err(inesperado()),
    }
}

pub async fn iniciar<C: ClienteSesiones, A: Almacen>(
    cliente: &C,
    almacen: &A,
    datos: &IniciarSesionDto,
) -> Result<SesionDto, ErroresDto> {
    let r = cliente.iniciar(datos).await.map_err(|_| sin_conexion())?;
    let emitido: TokenServidor = match r.codigo {
        200 => serde_json::from_str(&r.cuerpo).map_err(|_| inesperado())?,
        400 | 401 => return Err(errores_del_servidor(&r)),
        _ => return Err(inesperado()),
    };
    let rp = cliente
        .perfil(&emitido.token)
        .await
        .map_err(|_| sin_conexion())?;
    if rp.codigo != 200 {
        return Err(inesperado());
    }
    let usuario = leer_usuario(&rp)?;
    let sesion = SesionGuardada {
        token: emitido.token,
        vence_en: emitido.vence_en,
        nombre_visible: usuario.nombre_visible,
        invalidada: false,
    };
    almacen
        .guardar(&sesion)
        .map_err(|_| sin_campo("No se pudo guardar la sesión en este dispositivo."))?;
    Ok(vista(&sesion, &ahora_iso()))
}

pub async fn perfil<C: ClienteSesiones, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
) -> Result<UsuarioDto, ErroresDto> {
    let Some(sesion) = almacen.leer() else {
        return Err(sin_campo("No has iniciado sesión."));
    };
    if estado_de(&sesion, ahora) == EstadoSesion::Vencida {
        return Err(sesion_vencida());
    }
    let r = cliente
        .perfil(&sesion.token)
        .await
        .map_err(|_| sin_conexion())?;
    match r.codigo {
        200 => leer_usuario(&r),
        401 => {
            let mut invalida = sesion;
            invalida.invalidada = true;
            let _ = almacen.guardar(&invalida);
            Err(sesion_vencida())
        }
        _ => Err(inesperado()),
    }
}

pub fn estado<A: Almacen>(almacen: &A, ahora: &str) -> SesionDto {
    match almacen.leer() {
        None => SesionDto {
            estado: EstadoSesion::SinSesion,
            nombre_visible: None,
            vence_en: None,
        },
        Some(s) => vista(&s, ahora),
    }
}

// ---------- Comandos de Tauri ----------

fn almacen_de(app: &tauri::AppHandle) -> Result<AlmacenSistema, ErroresDto> {
    let carpeta = app
        .path()
        .app_data_dir()
        .map_err(|_| sin_campo("No se pudo acceder a los datos de la aplicación."))?;
    Ok(AlmacenSistema::nuevo(&carpeta))
}

/// POST /usuarios. Errores por campo: nombre_usuario, nombre_visible, correo, contrasena.
#[tauri::command]
pub async fn registrar_usuario(datos: RegistroDto) -> Result<UsuarioDto, ErroresDto> {
    registrar(&ClienteHttp::desde_entorno(), &datos).await
}

/// POST /sesiones + GET /usuarios/yo. Guarda el token en el dispositivo y devuelve el estado.
#[tauri::command]
pub async fn iniciar_sesion(
    app: tauri::AppHandle,
    datos: IniciarSesionDto,
) -> Result<SesionDto, ErroresDto> {
    iniciar(&ClienteHttp::desde_entorno(), &almacen_de(&app)?, &datos).await
}

/// GET /usuarios/yo con el token guardado.
#[tauri::command]
pub async fn obtener_perfil(app: tauri::AppHandle) -> Result<UsuarioDto, ErroresDto> {
    perfil(
        &ClienteHttp::desde_entorno(),
        &almacen_de(&app)?,
        &ahora_iso(),
    )
    .await
}

/// El servidor no tiene endpoint de cierre: solo borra el token del dispositivo.
#[tauri::command]
pub async fn cerrar_sesion(app: tauri::AppHandle) -> Result<(), ErroresDto> {
    almacen_de(&app)?.borrar();
    Ok(())
}

/// Al abrir la app: ¿hay sesión vigente, vencida o ninguna? No necesita red.
#[tauri::command]
pub async fn estado_sesion(app: tauri::AppHandle) -> Result<SesionDto, ErroresDto> {
    Ok(estado(&almacen_de(&app)?, &ahora_iso()))
}

#[cfg(test)]
mod pruebas;
