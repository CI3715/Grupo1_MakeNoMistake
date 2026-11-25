//! Puerto del almacén de secretos del sistema operativo y adaptador falso en
//! memoria para las pruebas (Entrega 8: Angel - Tarea 5; RF-INT-05, RNF-05).
//!
//! Guarda y lee la frase de paso de sincronización y las credenciales de Git
//! sin escribirlas nunca en texto plano en el libro de eventos, en el destino de
//! sincronización ni en los archivos exportados.

use std::fmt;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::respaldo::LARGO_MINIMO_FRASE;

/// Credenciales de autenticación para un repositorio Git remoto del usuario.
///
/// No deriva `Debug` automáticamente para evitar que el token o contraseña se
/// filtre en trazas o registros (RNF-05).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredencialesGit {
    /// Nombre de usuario para autenticar ante el servidor Git remoto.
    pub usuario: String,
    /// Token personal de acceso (PAT) o contraseña asociada al repositorio.
    pub token: String,
}

impl CredencialesGit {
    /// Crea un par de credenciales de Git con `usuario` y `token`.
    pub fn nuevas(usuario: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            usuario: usuario.into(),
            token: token.into(),
        }
    }

    /// Serializa las credenciales a JSON para su almacenamiento en el llavero del sistema.
    pub fn a_json(&self) -> Result<String, ErrorSecretos> {
        serde_json::to_string(self).map_err(|e| ErrorSecretos::AlmacenNoDisponible(e.to_string()))
    }

    /// Reconstruye las credenciales desde su representación JSON en el llavero del sistema.
    pub fn desde_json(texto: &str) -> Result<Self, ErrorSecretos> {
        serde_json::from_str(texto).map_err(|e| ErrorSecretos::AlmacenNoDisponible(e.to_string()))
    }
}

impl fmt::Debug for CredencialesGit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredencialesGit")
            .field("usuario", &self.usuario)
            .field("token", &"[OCULTO]")
            .finish()
    }
}

/// Errores tipados del almacén de secretos.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorSecretos {
    /// La frase de paso tiene menos de [`LARGO_MINIMO_FRASE`] caracteres.
    #[error("La frase de paso debe tener al menos 12 caracteres.")]
    FraseCorta,

    /// El usuario o el token de Git están vacíos.
    #[error("El usuario y el token o contraseña de Git son obligatorios.")]
    CredencialesIncompletas,

    /// El almacén de secretos del sistema operativo falló o no está disponible.
    #[error("No se pudo acceder al almacén de secretos del sistema operativo.")]
    AlmacenNoDisponible(String),
}

/// Valida que `frase` cumpla el largo mínimo de 12 caracteres.
pub fn validar_frase_de_paso(frase: &str) -> Result<(), ErrorSecretos> {
    if frase.chars().count() < LARGO_MINIMO_FRASE {
        return Err(ErrorSecretos::FraseCorta);
    }
    Ok(())
}

/// Valida que `credenciales` tenga tanto usuario como token no vacíos.
pub fn validar_credenciales_git(credenciales: &CredencialesGit) -> Result<(), ErrorSecretos> {
    if credenciales.usuario.trim().is_empty() || credenciales.token.trim().is_empty() {
        return Err(ErrorSecretos::CredencialesIncompletas);
    }
    Ok(())
}

/// Puerto para guardar, leer, actualizar y borrar los secretos de sincronización
/// (frase de paso y credenciales de Git) en el almacén del sistema operativo.
pub trait AlmacenSecretos: Send + Sync {
    /// Guarda (o reemplaza si ya existía) la frase de paso de sincronización.
    fn guardar_frase_de_paso(&self, frase: &str) -> Result<(), ErrorSecretos>;

    /// Actualiza la frase de paso en el almacén de secretos.
    fn actualizar_frase_de_paso(&self, frase: &str) -> Result<(), ErrorSecretos> {
        self.guardar_frase_de_paso(frase)
    }

    /// Lee la frase de paso guardada, o devuelve `Ok(None)` si no hay ninguna.
    fn leer_frase_de_paso(&self) -> Result<Option<String>, ErrorSecretos>;

    /// Elimina la frase de paso del almacén. Si no existía, devuelve `Ok(())`.
    fn borrar_frase_de_paso(&self) -> Result<(), ErrorSecretos>;

    /// Guarda (o reemplaza si ya existían) las credenciales de Git.
    fn guardar_credenciales_git(&self, credenciales: &CredencialesGit)
    -> Result<(), ErrorSecretos>;

    /// Actualiza las credenciales de Git en el almacén de secretos.
    fn actualizar_credenciales_git(
        &self,
        credenciales: &CredencialesGit,
    ) -> Result<(), ErrorSecretos> {
        self.guardar_credenciales_git(credenciales)
    }

    /// Lee las credenciales de Git guardadas, o devuelve `Ok(None)` si no hay.
    fn leer_credenciales_git(&self) -> Result<Option<CredencialesGit>, ErrorSecretos>;

    /// Elimina las credenciales de Git del almacén. Si no existían, devuelve `Ok(())`.
    fn borrar_credenciales_git(&self) -> Result<(), ErrorSecretos>;

    /// Borra tanto la frase de paso como las credenciales de Git.
    fn borrar_todo(&self) -> Result<(), ErrorSecretos> {
        self.borrar_frase_de_paso()?;
        self.borrar_credenciales_git()
    }
}

#[derive(Default)]
struct EstadoSecretosMemoria {
    frase_de_paso: Option<String>,
    credenciales_git: Option<CredencialesGit>,
    fallar: bool,
}

/// Adaptador falso en memoria del puerto [`AlmacenSecretos`] para pruebas.
#[derive(Default)]
pub struct AlmacenSecretosEnMemoria {
    estado: Mutex<EstadoSecretosMemoria>,
}

impl AlmacenSecretosEnMemoria {
    /// Crea un almacén de secretos en memoria inicialmente vacío.
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Configura si el almacén en memoria debe simular un fallo de disponibilidad.
    pub fn simular_fallo(&self, fallar: bool) {
        self.estado.lock().expect("mutex de secretos").fallar = fallar;
    }
}

impl AlmacenSecretos for AlmacenSecretosEnMemoria {
    fn guardar_frase_de_paso(&self, frase: &str) -> Result<(), ErrorSecretos> {
        validar_frase_de_paso(frase)?;
        let mut st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        st.frase_de_paso = Some(frase.to_string());
        Ok(())
    }

    fn leer_frase_de_paso(&self) -> Result<Option<String>, ErrorSecretos> {
        let st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        Ok(st.frase_de_paso.clone())
    }

    fn borrar_frase_de_paso(&self) -> Result<(), ErrorSecretos> {
        let mut st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        st.frase_de_paso = None;
        Ok(())
    }

    fn guardar_credenciales_git(
        &self,
        credenciales: &CredencialesGit,
    ) -> Result<(), ErrorSecretos> {
        validar_credenciales_git(credenciales)?;
        let mut st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        st.credenciales_git = Some(credenciales.clone());
        Ok(())
    }

    fn leer_credenciales_git(&self) -> Result<Option<CredencialesGit>, ErrorSecretos> {
        let st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        Ok(st.credenciales_git.clone())
    }

    fn borrar_credenciales_git(&self) -> Result<(), ErrorSecretos> {
        let mut st = self.estado.lock().expect("mutex de secretos");
        if st.fallar {
            return Err(ErrorSecretos::AlmacenNoDisponible(
                "fallo simulado en memoria".to_string(),
            ));
        }
        st.credenciales_git = None;
        Ok(())
    }
}

/// Batería de pruebas de contrato para cualquier adaptador de [`AlmacenSecretos`].
pub fn bateria_contrato_almacen_secretos(almacen: &dyn AlmacenSecretos) {
    // Limpieza inicial.
    almacen.borrar_todo().expect("borrar_todo inicial");
    assert_eq!(almacen.leer_frase_de_paso().unwrap(), None);
    assert_eq!(almacen.leer_credenciales_git().unwrap(), None);

    // Validación de frase corta (< 12 caracteres).
    assert_eq!(
        almacen.guardar_frase_de_paso("corta-11!!!"),
        Err(ErrorSecretos::FraseCorta)
    );
    assert_eq!(almacen.leer_frase_de_paso().unwrap(), None);

    // Guardar, leer, actualizar y borrar frase de paso.
    let frase_1 = "mi-frase-de-paso-inicial-segura";
    let frase_2 = "mi-frase-de-paso-actualizada-2026";
    almacen.guardar_frase_de_paso(frase_1).unwrap();
    assert_eq!(
        almacen.leer_frase_de_paso().unwrap().as_deref(),
        Some(frase_1)
    );

    almacen.actualizar_frase_de_paso(frase_2).unwrap();
    assert_eq!(
        almacen.leer_frase_de_paso().unwrap().as_deref(),
        Some(frase_2)
    );

    almacen.borrar_frase_de_paso().unwrap();
    assert_eq!(almacen.leer_frase_de_paso().unwrap(), None);
    // Borrar de nuevo es idempotente.
    almacen.borrar_frase_de_paso().unwrap();

    // Validación de credenciales incompletas.
    assert_eq!(
        almacen.guardar_credenciales_git(&CredencialesGit::nuevas("", "ghp_token")),
        Err(ErrorSecretos::CredencialesIncompletas)
    );
    assert_eq!(
        almacen.guardar_credenciales_git(&CredencialesGit::nuevas("beto", "   ")),
        Err(ErrorSecretos::CredencialesIncompletas)
    );
    assert_eq!(almacen.leer_credenciales_git().unwrap(), None);

    // Guardar, leer, actualizar y borrar credenciales de Git.
    let cred_1 = CredencialesGit::nuevas("beto", "ghp_token_inicial_12345");
    let cred_2 = CredencialesGit::nuevas("beto_nuevo", "ghp_token_rotado_67890");

    almacen.guardar_credenciales_git(&cred_1).unwrap();
    assert_eq!(almacen.leer_credenciales_git().unwrap(), Some(cred_1));

    almacen.actualizar_credenciales_git(&cred_2).unwrap();
    assert_eq!(
        almacen.leer_credenciales_git().unwrap(),
        Some(cred_2.clone())
    );

    // Las operaciones sobre la frase de paso y las credenciales de Git son independientes.
    almacen.guardar_frase_de_paso(frase_1).unwrap();
    almacen.borrar_credenciales_git().unwrap();
    assert_eq!(almacen.leer_credenciales_git().unwrap(), None);
    assert_eq!(
        almacen.leer_frase_de_paso().unwrap().as_deref(),
        Some(frase_1)
    );

    // borrar_todo elimina ambos secretos.
    almacen.guardar_credenciales_git(&cred_2).unwrap();
    almacen.borrar_todo().unwrap();
    assert_eq!(almacen.leer_frase_de_paso().unwrap(), None);
    assert_eq!(almacen.leer_credenciales_git().unwrap(), None);
}
