//! Adaptador Git del puerto `TransporteSincronizacion` (Entrega 8: John - Tarea 5;
//! RF-INT-03, RF-INT-07, RNF-10, RNF-14).
//!
//! El destino es un repositorio Git del usuario (GitHub, GitLab, un servidor propio o una
//! ruta local). El adaptador usa el programa `git` instalado en el equipo, sin bibliotecas
//! nativas: así funciona igual con HTTPS, SSH o una carpeta, y respeta la configuración de
//! red del usuario (proxy, certificados).
//!
//! # Disposición
//!
//! Cada dispositivo tiene una **copia local** del repositorio (una carpeta junto a su
//! libro). Dentro, la misma disposición que la carpeta compartida:
//!
//! ```text
//! <repositorio>/
//!   <id-del-dispositivo-A>/20261124T120000Z-0001.lote
//!   <id-del-dispositivo-B>/20261124T130000Z-0001.lote
//! ```
//!
//! - **Escribir** un lote lo deja en la carpeta del dispositivo y lo confirma (`commit`) en
//!   la copia local, sin red: funciona sin conexión y no se pierde (RNF-17).
//! - **Publicar** empuja (`push`) la rama `main`. Si el remoto cambió, vuelve a traer,
//!   coloca los lotes propios encima de los ajenos (`rebase`) y reintenta. Como cada
//!   dispositivo solo escribe en su carpeta y nunca modifica un lote, ese `rebase` no
//!   puede tener conflictos.
//! - **Traer** hace `fetch` e integra lo nuevo. Git solo transfiere los objetos que la copia
//!   local no tiene, y quien sincroniza solo lee los lotes que le faltan (RNF-10).
//! - El contenido ya llega cifrado desde el caso de uso (RF-INT-06): este adaptador solo
//!   mueve bytes. Los lotes se guardan sin conversión de fines de línea.
//!
//! # Credenciales
//!
//! Vienen del almacén de secretos del sistema (`CredencialesGit`) y nunca se escriben en la
//! copia local, en la URL ni en la línea de órdenes: se pasan a cada proceso `git` por
//! variables de entorno (`GIT_CONFIG_*`, cabecera HTTP de autenticación). Se desactivan
//! los ayudantes de credenciales y las preguntas por terminal, para que una credencial
//! rechazada dé un error claro en lugar de quedarse esperando.
//!
//! # Errores
//!
//! La salida de `git` se lee en inglés (`LC_ALL=C`) para distinguir: sin conexión (no se
//! resuelve el servidor), destino inaccesible (repositorio inexistente o caído),
//! credenciales rechazadas y destino que cambió (empuje rechazado).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aplicacion::secretos::CredencialesGit;
use aplicacion::transporte::{ErrorTransporte, TransporteSincronizacion, validar_nombre};

use crate::transporte_carpeta::TransporteCarpeta;

/// Rama del repositorio donde viven los lotes.
pub const RAMA: &str = "main";

/// Veces que `publicar` vuelve a traer y empujar si otro dispositivo publicó a la vez.
const REINTENTOS_EMPUJE: usize = 5;

/// Transporte de un dispositivo sobre un repositorio Git.
pub struct TransporteGit {
    remoto: String,
    copia: PathBuf,
    dispositivo: String,
    credenciales: Option<CredencialesGit>,
    /// Lectura y escritura de lotes sobre el árbol de trabajo de la copia local.
    carpeta: TransporteCarpeta,
    /// La copia local ya existe y apunta a `remoto` (se comprueba una vez por instancia).
    preparada: bool,
}

impl std::fmt::Debug for TransporteGit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransporteGit")
            .field("remoto", &self.remoto)
            .field("copia", &self.copia)
            .field("dispositivo", &self.dispositivo)
            .field("credenciales", &self.credenciales)
            .finish()
    }
}

impl TransporteGit {
    /// Crea el transporte de `dispositivo` sobre el repositorio `remoto` (URL o ruta),
    /// con su copia local en `copia`. No toca el disco ni la red: la copia se crea al
    /// primer uso.
    pub fn nuevo(
        remoto: &str,
        copia: impl Into<PathBuf>,
        dispositivo: &str,
        credenciales: Option<CredencialesGit>,
    ) -> Result<TransporteGit, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        let remoto = remoto.trim();
        // Una ubicación que empieza con `-` se tomaría como una opción de `git`.
        if remoto.is_empty() || remoto.starts_with('-') || remoto.contains(['\n', '\r', '\0']) {
            return Err(ErrorTransporte::DestinoInaccesible);
        }
        let copia = copia.into();
        Ok(TransporteGit {
            remoto: remoto.to_string(),
            carpeta: TransporteCarpeta::nuevo(&copia, dispositivo)?,
            copia,
            dispositivo: dispositivo.to_string(),
            credenciales,
            preparada: false,
        })
    }

    /// Carpeta de la copia local.
    pub fn copia_local(&self) -> &Path {
        &self.copia
    }

    fn hay_copia(&self) -> bool {
        self.copia.join(".git").is_dir()
    }

    // ---------- Ejecución de git ----------

    /// Variables de configuración que se pasan a cada proceso por el entorno, nunca por
    /// la línea de órdenes (que otros procesos del equipo pueden ver).
    fn configuracion(&self) -> Vec<(String, String)> {
        let mut config = vec![
            // Los lotes son bytes: sin conversión de fines de línea.
            ("core.autocrlf".to_string(), "false".to_string()),
            ("core.safecrlf".to_string(), "false".to_string()),
            // Que una firma configurada por el usuario no pida su clave.
            ("commit.gpgsign".to_string(), "false".to_string()),
            // Sin ayudantes de credenciales: ni ventanas ni credenciales ajenas.
            ("credential.helper".to_string(), String::new()),
            // Una transferencia que se queda parada se corta en vez de colgar la app.
            ("http.lowSpeedLimit".to_string(), "1000".to_string()),
            ("http.lowSpeedTime".to_string(), "60".to_string()),
        ];
        if let Some(c) = &self.credenciales
            && es_http(&self.remoto)
        {
            let basica = base64(format!("{}:{}", c.usuario, c.token).as_bytes());
            config.push((
                "http.extraHeader".to_string(),
                format!("Authorization: Basic {basica}"),
            ));
        }
        config
    }

    fn git(&self, argumentos: &[&str]) -> Result<Output, ErrorTransporte> {
        let mut orden = Command::new("git");
        orden
            .arg("-C")
            .arg(&self.copia)
            .args(argumentos)
            .stdin(Stdio::null())
            .env("LC_ALL", "C")
            .env("LANGUAGE", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .env(
                "GIT_AUTHOR_NAME",
                format!("Cuentas Claras ({})", self.dispositivo),
            )
            .env(
                "GIT_AUTHOR_EMAIL",
                format!("{}@cuentas-claras.invalid", self.dispositivo),
            )
            .env(
                "GIT_COMMITTER_NAME",
                format!("Cuentas Claras ({})", self.dispositivo),
            )
            .env(
                "GIT_COMMITTER_EMAIL",
                format!("{}@cuentas-claras.invalid", self.dispositivo),
            )
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
        if std::env::var_os("GIT_SSH_COMMAND").is_none() {
            // SSH sin preguntas: una clave que falta es un error, no una espera.
            orden.env(
                "GIT_SSH_COMMAND",
                "ssh -o BatchMode=yes -o ConnectTimeout=20",
            );
        }
        let config = self.configuracion();
        orden.env("GIT_CONFIG_COUNT", config.len().to_string());
        for (i, (clave, valor)) in config.iter().enumerate() {
            orden.env(format!("GIT_CONFIG_KEY_{i}"), clave);
            orden.env(format!("GIT_CONFIG_VALUE_{i}"), valor);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            orden.creation_flags(CREATE_NO_WINDOW);
        }
        orden.output().map_err(|e| {
            ErrorTransporte::Interno(format!("no se pudo ejecutar git (¿está instalado?): {e}"))
        })
    }

    /// Ejecuta `git` y traduce un fallo a un error del transporte.
    fn git_ok(&self, argumentos: &[&str]) -> Result<Output, ErrorTransporte> {
        let salida = self.git(argumentos)?;
        if salida.status.success() {
            Ok(salida)
        } else {
            Err(clasificar_error_git(&String::from_utf8_lossy(
                &salida.stderr,
            )))
        }
    }

    /// `true` si la referencia existe.
    fn existe(&self, referencia: &str) -> Result<bool, ErrorTransporte> {
        let consulta = format!("{referencia}^{{commit}}");
        Ok(self
            .git(&["rev-parse", "-q", "--verify", &consulta])?
            .status
            .success())
    }

    fn rama_remota() -> String {
        format!("refs/remotes/origin/{RAMA}")
    }

    // ---------- Copia local ----------

    /// Crea la copia local si no existe (sin red) y comprueba que apunte al remoto.
    fn preparar(&mut self) -> Result<(), ErrorTransporte> {
        if self.preparada && self.hay_copia() {
            return Ok(());
        }
        if !self.hay_copia() {
            fs::create_dir_all(&self.copia).map_err(|e| ErrorTransporte::Interno(e.to_string()))?;
            self.git_ok(&["init", "-q"])?;
            self.git_ok(&["symbolic-ref", "HEAD", &format!("refs/heads/{RAMA}")])?;
        }
        // Solo se trae la rama de los lotes, aunque el repositorio tenga otras.
        let refspec = format!("+refs/heads/{RAMA}:refs/remotes/origin/{RAMA}");
        let actual = self.git(&["remote", "get-url", "origin"])?;
        if !actual.status.success() {
            self.git_ok(&["remote", "add", "origin", "--", &self.remoto])?;
        } else if String::from_utf8_lossy(&actual.stdout).trim() != self.remoto {
            self.git_ok(&["remote", "set-url", "origin", "--", &self.remoto])?;
        }
        self.git_ok(&["config", "remote.origin.fetch", &refspec])?;
        self.git_ok(&["config", "core.autocrlf", "false"])?;
        self.preparada = true;
        Ok(())
    }

    /// Coloca los lotes propios sin publicar encima de lo traído del remoto.
    fn integrar(&self) -> Result<(), ErrorTransporte> {
        let remota = Self::rama_remota();
        if !self.existe(&remota)? {
            return Ok(()); // remoto vacío: nada que integrar
        }
        if !self.existe("HEAD")? {
            // Primera vez y sin lotes propios: la copia pasa a ser la del remoto.
            self.git_ok(&["reset", "-q", "--hard", &remota])?;
            return Ok(());
        }
        let rebase = self.git(&["rebase", "-q", &remota])?;
        if !rebase.status.success() {
            // No debería ocurrir: cada dispositivo escribe en archivos distintos.
            let _ = self.git(&["rebase", "--abort"]);
            return Err(ErrorTransporte::Interno(format!(
                "no se pudieron integrar los lotes del remoto: {}",
                String::from_utf8_lossy(&rebase.stderr).trim()
            )));
        }
        Ok(())
    }

    fn fetch(&self) -> Result<(), ErrorTransporte> {
        let salida = self.git(&["fetch", "-q", "--no-tags", "origin"])?;
        if salida.status.success() {
            return Ok(());
        }
        let error = String::from_utf8_lossy(&salida.stderr);
        // Un repositorio recién creado todavía no tiene la rama: no hay nada que traer.
        if error.contains("couldn't find remote ref") {
            return Ok(());
        }
        Err(clasificar_error_git(&error))
    }

    /// Lotes propios confirmados que el remoto todavía no tiene.
    fn hay_pendientes(&self) -> Result<bool, ErrorTransporte> {
        if !self.existe("HEAD")? {
            return Ok(false);
        }
        let remota = Self::rama_remota();
        if !self.existe(&remota)? {
            return Ok(true);
        }
        let salida = self.git_ok(&["rev-list", "--count", &format!("{remota}..HEAD")])?;
        Ok(String::from_utf8_lossy(&salida.stdout).trim() != "0")
    }

    fn ruta_relativa(&self, lote: &str) -> String {
        format!("{}/{lote}.lote", self.dispositivo)
    }
}

impl TransporteSincronizacion for TransporteGit {
    fn dispositivo_local(&self) -> &str {
        &self.dispositivo
    }

    fn traer(&mut self) -> Result<(), ErrorTransporte> {
        self.preparar()?;
        self.fetch()?;
        self.integrar()
    }

    fn listar_dispositivos(&self) -> Result<Vec<String>, ErrorTransporte> {
        if !self.hay_copia() {
            return Ok(Vec::new());
        }
        self.carpeta.listar_dispositivos()
    }

    fn listar_lotes(&self, dispositivo: &str) -> Result<Vec<String>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        if !self.hay_copia() {
            return Ok(Vec::new());
        }
        self.carpeta.listar_lotes(dispositivo)
    }

    fn leer_lote(&self, dispositivo: &str, lote: &str) -> Result<Vec<u8>, ErrorTransporte> {
        validar_nombre(dispositivo)?;
        validar_nombre(lote)?;
        if !self.hay_copia() {
            return Err(ErrorTransporte::LoteNoExiste {
                dispositivo: dispositivo.to_string(),
                lote: lote.to_string(),
            });
        }
        self.carpeta.leer_lote(dispositivo, lote)
    }

    /// Escribe el lote y lo confirma en la copia local. No usa la red.
    fn escribir_lote(&mut self, lote: &str, contenido: &[u8]) -> Result<(), ErrorTransporte> {
        validar_nombre(lote)?;
        self.preparar()?;
        self.carpeta.escribir_lote(lote, contenido)?;

        let ruta = self.ruta_relativa(lote);
        let mensaje = format!("Lote {ruta}");
        let confirmado = self.git_ok(&["add", "--", &ruta]).and_then(|_| {
            self.git_ok(&["commit", "-q", "--no-verify", "-m", &mensaje, "--", &ruta])
        });
        if let Err(e) = confirmado {
            // Todo o nada: sin confirmar, el lote no queda escrito.
            let _ = self.git(&["reset", "-q", "--", &ruta]);
            let _ = fs::remove_file(self.copia.join(&ruta));
            return Err(e);
        }
        Ok(())
    }

    /// Empuja los lotes confirmados. Si otro dispositivo publicó a la vez, vuelve a traer,
    /// integra y reintenta; tras varios intentos devuelve `DestinoCambio`.
    fn publicar(&mut self) -> Result<(), ErrorTransporte> {
        self.preparar()?;
        if !self.hay_pendientes()? {
            return Ok(());
        }
        let destino = format!("HEAD:refs/heads/{RAMA}");
        for _ in 0..REINTENTOS_EMPUJE {
            match self.git_ok(&["push", "-q", "--no-verify", "origin", &destino]) {
                Ok(_) => {
                    // Deja la rama remota local al día sin otra consulta.
                    let _ = self.git(&["update-ref", &Self::rama_remota(), "HEAD"]);
                    return Ok(());
                }
                Err(ErrorTransporte::DestinoCambio) => {
                    self.fetch()?;
                    self.integrar()?;
                }
                Err(e) => return Err(e),
            }
        }
        Err(ErrorTransporte::DestinoCambio)
    }
}

/// `true` si el remoto se alcanza por HTTP(S): solo ahí se envía la cabecera con las
/// credenciales.
fn es_http(remoto: &str) -> bool {
    let minusculas = remoto.to_ascii_lowercase();
    minusculas.starts_with("https:") || minusculas.starts_with("http:")
}

/// Traduce la salida de error de `git` (en inglés, `LC_ALL=C`) a un error del transporte.
/// El orden importa: un fallo de autenticación suele venir acompañado de «could not read
/// from remote repository», y un servidor que no se resuelve de «unable to access».
pub fn clasificar_error_git(stderr: &str) -> ErrorTransporte {
    let texto = stderr.to_lowercase();
    let contiene = |patrones: &[&str]| patrones.iter().any(|p| texto.contains(p));

    if contiene(&[
        "authentication failed",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "invalid username or password",
        "invalid credentials",
        "http basic: access denied",
        "access denied",
        "permission denied (publickey",
        "permission denied, please try again",
        "host key verification failed",
        "the requested url returned error: 401",
        "the requested url returned error: 403",
        "write access to repository not granted",
    ]) || (texto.contains("permission to ") && texto.contains(" denied"))
    {
        return ErrorTransporte::CredencialesRechazadas;
    }
    // Se llegó al servidor, pero no atiende: el destino está caído, no la red.
    if contiene(&["connection refused", "connection reset"]) {
        return ErrorTransporte::DestinoInaccesible;
    }
    if contiene(&[
        "could not resolve host",
        "could not resolve hostname",
        "temporary failure in name resolution",
        "name or service not known",
        "nodename nor servname",
        "no such host is known",
        "network is unreachable",
        "no route to host",
        "failed to connect",
        "connection timed out",
        "operation timed out",
        "timed out after",
    ]) {
        return ErrorTransporte::SinConexion;
    }
    if contiene(&[
        "[rejected]",
        "fetch first",
        "non-fast-forward",
        "updates were rejected",
        "cannot lock ref",
        "incorrect old value provided",
    ]) {
        return ErrorTransporte::DestinoCambio;
    }
    if contiene(&[
        "does not appear to be a git repository",
        "repository not found",
        "not found",
        "could not read from remote repository",
        "unable to access",
        "remote end hung up",
        "no such file or directory",
        "returned error: 5",
        "rpc failed",
    ]) {
        return ErrorTransporte::DestinoInaccesible;
    }
    ErrorTransporte::Interno(stderr.trim().to_string())
}

/// Base64 estándar (RFC 4648), para la cabecera de autenticación básica.
fn base64(bytes: &[u8]) -> String {
    const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut salida = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for trozo in bytes.chunks(3) {
        let b = [
            trozo[0],
            trozo.get(1).copied().unwrap_or(0),
            trozo.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= trozo.len() {
                salida.push(char::from(ALFABETO[((n >> (18 - 6 * i)) & 0x3F) as usize]));
            } else {
                salida.push('=');
            }
        }
    }
    salida
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn base64_coincide_con_el_estandar() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"ana:ghp_token"), "YW5hOmdocF90b2tlbg==");
        assert_eq!(base64("ñ:€".as_bytes()), "w7E64oKs");
    }

    #[test]
    fn solo_http_lleva_la_cabecera_de_credenciales() {
        assert!(es_http("https://github.com/ana/libro.git"));
        assert!(es_http("HTTP://servidor.local/libro.git"));
        assert!(!es_http("git@github.com:ana/libro.git"));
        assert!(!es_http("ssh://git@github.com/ana/libro.git"));
        assert!(!es_http("/home/ana/libro.git"));
    }

    #[test]
    fn el_debug_no_muestra_el_token() {
        let t = TransporteGit::nuevo(
            "https://example.invalid/libro.git",
            "/tmp/no-se-usa",
            "laptop",
            Some(CredencialesGit::nuevas("ana", "tok-super-secreto")),
        )
        .unwrap();
        let texto = format!("{t:?}");
        assert!(!texto.contains("tok-super-secreto"), "{texto}");
        assert!(texto.contains("ana"));
    }
}
