//! Adaptador del almacén de secretos del sistema operativo mediante `keyring`
//! (Entrega 8: Angel - Tarea 5; RF-INT-05).
//!
//! Utiliza el llavero nativo del sistema operativo (Credential Manager en Windows,
//! Secret Service en Linux, Keychain en macOS) para guardar la frase de paso de
//! sincronización y las credenciales de Git. Nada se escribe en texto plano en el
//! libro de eventos, en el destino de sincronización ni en la exportación.

use aplicacion::secretos::{
    AlmacenSecretos, CredencialesGit, ErrorSecretos, validar_credenciales_git,
    validar_frase_de_paso,
};

/// Nombre del servicio por defecto en el almacén de secretos del sistema operativo.
pub const SERVICIO_SECRETOS_POR_DEFECTO: &str = "cuentas-claras-sincronizacion";

const CUENTA_FRASE_DE_PASO: &str = "frase_de_paso";
const CUENTA_CREDENCIALES_GIT: &str = "credenciales_git";

/// Adaptador real del puerto [`AlmacenSecretos`] respaldado por el almacén de
/// secretos nativo del sistema operativo (`keyring`).
#[derive(Debug, Clone)]
pub struct AlmacenSecretosSistema {
    servicio: String,
}

/// Alias de [`AlmacenSecretosSistema`].
pub type AlmacenSecretosKeyring = AlmacenSecretosSistema;

impl Default for AlmacenSecretosSistema {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl AlmacenSecretosSistema {
    /// Crea un adaptador sobre el servicio por defecto (`"cuentas-claras-sincronizacion"`).
    pub fn nuevo() -> Self {
        Self {
            servicio: SERVICIO_SECRETOS_POR_DEFECTO.to_string(),
        }
    }

    /// Crea un adaptador con un nombre de servicio personalizado (útil para aislar
    /// entornos o instancias de prueba).
    pub fn con_servicio(servicio: impl Into<String>) -> Self {
        Self {
            servicio: servicio.into(),
        }
    }

    fn entrada(&self, cuenta: &str) -> Result<keyring::v1::Entry, ErrorSecretos> {
        keyring::v1::Entry::new(&self.servicio, cuenta)
            .map_err(|e| ErrorSecretos::AlmacenNoDisponible(e.to_string()))
    }

    fn guardar_entrada(&self, cuenta: &str, secreto: &str) -> Result<(), ErrorSecretos> {
        let entrada = self.entrada(cuenta)?;
        entrada
            .set_password(secreto)
            .map_err(|e| ErrorSecretos::AlmacenNoDisponible(e.to_string()))
    }

    fn leer_entrada(&self, cuenta: &str) -> Result<Option<String>, ErrorSecretos> {
        let entrada = self.entrada(cuenta)?;
        match entrada.get_password() {
            Ok(valor) => Ok(Some(valor)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ErrorSecretos::AlmacenNoDisponible(e.to_string())),
        }
    }

    fn borrar_entrada(&self, cuenta: &str) -> Result<(), ErrorSecretos> {
        let entrada = self.entrada(cuenta)?;
        match entrada.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(ErrorSecretos::AlmacenNoDisponible(e.to_string())),
        }
    }
}

impl AlmacenSecretos for AlmacenSecretosSistema {
    fn guardar_frase_de_paso(&self, frase: &str) -> Result<(), ErrorSecretos> {
        validar_frase_de_paso(frase)?;
        self.guardar_entrada(CUENTA_FRASE_DE_PASO, frase)
    }

    fn leer_frase_de_paso(&self) -> Result<Option<String>, ErrorSecretos> {
        self.leer_entrada(CUENTA_FRASE_DE_PASO)
    }

    fn borrar_frase_de_paso(&self) -> Result<(), ErrorSecretos> {
        self.borrar_entrada(CUENTA_FRASE_DE_PASO)
    }

    fn guardar_credenciales_git(
        &self,
        credenciales: &CredencialesGit,
    ) -> Result<(), ErrorSecretos> {
        validar_credenciales_git(credenciales)?;
        let json = credenciales.a_json()?;
        self.guardar_entrada(CUENTA_CREDENCIALES_GIT, &json)
    }

    fn leer_credenciales_git(&self) -> Result<Option<CredencialesGit>, ErrorSecretos> {
        let Some(json) = self.leer_entrada(CUENTA_CREDENCIALES_GIT)? else {
            return Ok(None);
        };
        let credenciales = CredencialesGit::desde_json(&json)?;
        Ok(Some(credenciales))
    }

    fn borrar_credenciales_git(&self) -> Result<(), ErrorSecretos> {
        self.borrar_entrada(CUENTA_CREDENCIALES_GIT)
    }
}
