//! Destino de sincronización del libro y sus comandos (Entrega 8: RF-INT-03 a RF-INT-07).
//!
//! El destino es por libro (ADR-014): cada usuario tiene el suyo. Lo que no es secreto
//! (tipo, ubicación, usuario de Git) y lo que el dispositivo recuerda de la sincronización
//! (lotes leídos, eventos enviados, fecha de la última) viven en un archivo junto a su libro,
//! `<libro>.sync.json`. La frase de paso y el token de Git van solo al almacén de secretos del
//! sistema, con el id del usuario en el nombre del servicio: nunca al libro, al destino ni a
//! este archivo (RF-INT-05). Nada de esto habla con el servidor (RNF-01, INV-16).
//!
//! La lógica son funciones que reciben el archivo y el almacén de secretos, para probarlas sin
//! Tauri. Los comandos solo calculan esas dos cosas y delegan.

use std::fs;
use std::path::{Path, PathBuf};

use aplicacion::secretos::{
    AlmacenSecretos, CredencialesGit, ErrorSecretos, validar_credenciales_git,
    validar_frase_de_paso,
};
use aplicacion::sincronizar_libro::{EstadoSincronizacion, sincronizar};
use aplicacion::{CasosDeUso, Cifrador, Reloj};
use infraestructura::cifrado::CifradorArgon2;
use infraestructura::reloj::RelojSistema;
use infraestructura::secretos::{AlmacenSecretosSistema, SERVICIO_SECRETOS_POR_DEFECTO};
use infraestructura::transporte_carpeta::TransporteCarpeta;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::EstadoCasosDeUso;
use crate::dto::ErroresDto;
use crate::libros;
use crate::respaldo::EventoRechazadoDto;
use crate::sesion::{Almacen, AlmacenSistema, sin_campo};

const INTERNO: &str = "Ocurrió un problema interno. Intenta de nuevo.";
const CONFIG_ILEGIBLE: &str = "No se pudo leer la configuración de la sincronización.";
const GIT_NO_DISPONIBLE: &str = "El destino en un repositorio Git todavía no está disponible en esta versión. Usa una carpeta compartida.";
const TIPO_CARPETA: &str = "carpeta";
const TIPO_GIT: &str = "git";

// ---------- Datos que viajan a la pantalla ----------

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DestinoDto {
    pub tipo: String,
    pub ubicacion: String,
    pub usuario: Option<String>,
}

#[derive(Serialize)]
pub struct EstadoDto {
    pub destino: Option<DestinoDto>,
    pub ultima_sincronizacion: Option<String>,
}

/// Lo que la pantalla manda para configurar el destino. No deriva `Debug`: lleva la frase y
/// el token, que no deben acabar en un registro.
#[derive(Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
#[allow(dead_code)] // los campos de Git los usará el adaptador Git cuando exista
pub enum ConfigurarDestinoDto {
    Carpeta {
        ubicacion: String,
        frase: String,
    },
    Git {
        ubicacion: String,
        usuario: String,
        token: String,
        frase: String,
    },
}

#[derive(Deserialize)]
pub struct CredencialesDto {
    pub usuario: String,
    pub token: String,
}

#[derive(Serialize)]
pub struct InformeDto {
    pub enviados: usize,
    pub recibidos: usize,
    pub rechazados: Vec<EventoRechazadoDto>,
    pub ultima_sincronizacion: String,
}

// ---------- Archivo junto al libro ----------

#[derive(Serialize, Deserialize, Clone)]
struct DestinoGuardado {
    tipo: String,
    ubicacion: String,
    usuario: Option<String>,
}

impl From<DestinoGuardado> for DestinoDto {
    fn from(d: DestinoGuardado) -> Self {
        DestinoDto {
            tipo: d.tipo,
            ubicacion: d.ubicacion,
            usuario: d.usuario,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Archivo {
    destino: Option<DestinoGuardado>,
    estado: EstadoSincronizacion,
    ultima_sincronizacion: Option<String>,
}

fn leer(ruta: &Path) -> Result<Archivo, ErroresDto> {
    match fs::read_to_string(ruta) {
        Ok(texto) => serde_json::from_str(&texto).map_err(|_| sin_campo(CONFIG_ILEGIBLE)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Archivo::default()),
        Err(_) => Err(sin_campo(CONFIG_ILEGIBLE)),
    }
}

/// Escribe primero a un temporal y luego renombra, para no dejar el archivo a medias.
fn guardar(ruta: &Path, archivo: &Archivo) -> Result<(), ErroresDto> {
    let texto = serde_json::to_string_pretty(archivo).map_err(|_| sin_campo(INTERNO))?;
    let mut nombre = ruta.as_os_str().to_owned();
    nombre.push(".tmp");
    let temporal = PathBuf::from(nombre);
    let fallo = || sin_campo("No se pudo guardar la configuración de la sincronización.");
    if fs::write(&temporal, texto).is_err() {
        let _ = fs::remove_file(&temporal);
        return Err(fallo());
    }
    if fs::rename(&temporal, ruta).is_err() {
        let _ = fs::remove_file(&temporal);
        return Err(fallo());
    }
    Ok(())
}

fn secreto(e: ErrorSecretos) -> ErroresDto {
    sin_campo(&e.to_string())
}

// ---------- Lógica (sin Tauri) ----------

/// Destino configurado y fecha de la última sincronización exitosa. Sin red.
pub fn estado_del_destino(ruta: &Path) -> Result<EstadoDto, ErroresDto> {
    let archivo = leer(ruta)?;
    Ok(EstadoDto {
        destino: archivo.destino.map(Into::into),
        ultima_sincronizacion: archivo.ultima_sincronizacion,
    })
}

/// Guarda el destino. Un destino nuevo empieza de cero: sin lotes leídos ni eventos enviados.
pub fn configurar_destino(
    ruta: &Path,
    secretos: &dyn AlmacenSecretos,
    datos: ConfigurarDestinoDto,
) -> Result<DestinoDto, ErroresDto> {
    let (ubicacion, frase) = match datos {
        ConfigurarDestinoDto::Git { .. } => return Err(sin_campo(GIT_NO_DISPONIBLE)),
        ConfigurarDestinoDto::Carpeta { ubicacion, frase } => (ubicacion, frase),
    };
    validar_frase_de_paso(&frase).map_err(secreto)?;
    let carpeta = Path::new(&ubicacion);
    if !carpeta.is_absolute() || !carpeta.is_dir() {
        return Err(sin_campo("La carpeta elegida no existe."));
    }
    secretos.guardar_frase_de_paso(&frase).map_err(secreto)?;
    secretos.borrar_credenciales_git().map_err(secreto)?;
    let destino = DestinoGuardado {
        tipo: TIPO_CARPETA.to_string(),
        ubicacion,
        usuario: None,
    };
    let archivo = Archivo {
        destino: Some(destino.clone()),
        ..Archivo::default()
    };
    if let Err(e) = guardar(ruta, &archivo) {
        let _ = secretos.borrar_todo();
        return Err(e);
    }
    Ok(destino.into())
}

/// Reemplaza usuario y token del destino Git.
pub fn actualizar_credenciales(
    ruta: &Path,
    secretos: &dyn AlmacenSecretos,
    datos: CredencialesDto,
) -> Result<(), ErroresDto> {
    let mut archivo = leer(ruta)?;
    let Some(destino) = archivo.destino.as_mut().filter(|d| d.tipo == TIPO_GIT) else {
        return Err(sin_campo(
            "El destino configurado no es un repositorio Git.",
        ));
    };
    let credenciales = CredencialesGit::nuevas(datos.usuario.trim(), datos.token);
    validar_credenciales_git(&credenciales).map_err(secreto)?;
    secretos
        .actualizar_credenciales_git(&credenciales)
        .map_err(secreto)?;
    destino.usuario = Some(credenciales.usuario.clone());
    guardar(ruta, &archivo)
}

/// Quita el destino de este dispositivo y borra su frase y credenciales. No borra nada del
/// destino ni cambia el libro.
pub fn quitar_destino(ruta: &Path, secretos: &dyn AlmacenSecretos) -> Result<(), ErroresDto> {
    secretos.borrar_todo().map_err(secreto)?;
    match fs::remove_file(ruta) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(sin_campo("No se pudo quitar la configuración del destino.")),
    }
}

/// Trae, fusiona, envía y publica. El estado nuevo solo se guarda si todo salió bien: si
/// falla, lo pendiente sale en la siguiente sincronización y no se pierde nada.
pub fn sincronizar_con_destino(
    casos: &mut CasosDeUso,
    ruta: &Path,
    secretos: &dyn AlmacenSecretos,
    cifrador: &dyn Cifrador,
    ahora: &str,
) -> Result<InformeDto, ErroresDto> {
    let mut archivo = leer(ruta)?;
    let Some(destino) = archivo.destino.clone() else {
        return Err(sin_campo(
            "No hay un destino de sincronización configurado.",
        ));
    };
    if destino.tipo != TIPO_CARPETA {
        return Err(sin_campo(GIT_NO_DISPONIBLE));
    }
    let frase = secretos
        .leer_frase_de_paso()
        .map_err(secreto)?
        .ok_or_else(|| {
            sin_campo("No se encontró la frase de paso de este destino. Configúralo de nuevo.")
        })?;
    let mut transporte = TransporteCarpeta::nuevo(&destino.ubicacion, &casos.id_dispositivo())
        .map_err(|e| sin_campo(&e.to_string()))?;
    let (informe, estado) = sincronizar(
        casos,
        &mut transporte,
        cifrador,
        &frase,
        ahora,
        &archivo.estado,
    )
    .map_err(|e| sin_campo(&e.to_string()))?;

    archivo.estado = estado;
    archivo.ultima_sincronizacion = Some(ahora.to_string());
    guardar(ruta, &archivo)?;
    Ok(InformeDto {
        enviados: informe.enviados,
        recibidos: informe.recibidos,
        rechazados: informe
            .rechazados
            .into_iter()
            .map(|r| EventoRechazadoDto {
                id: r.id,
                motivo: r.motivo,
            })
            .collect(),
        ultima_sincronizacion: ahora.to_string(),
    })
}

// ---------- Comandos ----------

struct Contexto {
    ruta_estado: PathBuf,
    secretos: AlmacenSecretosSistema,
}

/// El archivo y el almacén de secretos del libro que corresponde a la sesión guardada.
fn contexto(app: &AppHandle) -> Result<Contexto, ErroresDto> {
    let carpeta = app.path().app_data_dir().map_err(|_| sin_campo(INTERNO))?;
    let usuario = AlmacenSistema::nuevo(&carpeta)
        .leer()
        .map(|s| s.usuario_id)
        .filter(|id| !id.is_empty());
    let libro = libros::ruta_objetivo(&carpeta, usuario.as_deref()).map_err(|m| sin_campo(&m))?;
    let nombre = usuario.as_deref().unwrap_or("sin-sesion");
    Ok(Contexto {
        ruta_estado: libro.with_extension("sync.json"),
        secretos: AlmacenSecretosSistema::con_servicio(format!(
            "{SERVICIO_SECRETOS_POR_DEFECTO}-{nombre}"
        )),
    })
}

/// Destino configurado (si lo hay) y fecha de la última sincronización exitosa.
#[tauri::command]
pub fn estado_sincronizacion_libro(app: AppHandle) -> Result<EstadoDto, ErroresDto> {
    estado_del_destino(&contexto(&app)?.ruta_estado)
}

/// Guarda el destino. Se invoca con `invoke('configurar_destino_sincronizacion', { datos })`.
#[tauri::command(rename_all = "snake_case")]
pub fn configurar_destino_sincronizacion(
    app: AppHandle,
    datos: ConfigurarDestinoDto,
) -> Result<DestinoDto, ErroresDto> {
    let c = contexto(&app)?;
    configurar_destino(&c.ruta_estado, &c.secretos, datos)
}

/// Reemplaza usuario y token del repositorio Git configurado.
#[tauri::command(rename_all = "snake_case")]
pub fn actualizar_credenciales_destino(
    app: AppHandle,
    datos: CredencialesDto,
) -> Result<(), ErroresDto> {
    let c = contexto(&app)?;
    actualizar_credenciales(&c.ruta_estado, &c.secretos, datos)
}

/// Quita el destino y borra su frase y credenciales del almacén de secretos.
#[tauri::command]
pub fn quitar_destino_sincronizacion(app: AppHandle) -> Result<(), ErroresDto> {
    let c = contexto(&app)?;
    quitar_destino(&c.ruta_estado, &c.secretos)
}

/// Trae, fusiona, envía y publica (RF-INT-03). Informa enviados, recibidos y rechazados.
#[tauri::command]
pub fn sincronizar_libro(
    app: AppHandle,
    estado: State<'_, EstadoCasosDeUso>,
) -> Result<InformeDto, ErroresDto> {
    let c = contexto(&app)?;
    let mut casos = estado.lock().map_err(|_| sin_campo(INTERNO))?;
    sincronizar_con_destino(
        &mut casos,
        &c.ruta_estado,
        &c.secretos,
        &CifradorArgon2::nuevo(),
        &RelojSistema::nuevo().ahora_utc(),
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use aplicacion::secretos::AlmacenSecretosEnMemoria;
    use aplicacion::{
        DispositivoFijo, ErrorRespaldo, GeneradorIdentificadores, NuevaCuenta, RelojFijo,
        RepositorioEnMemoria,
    };

    const FRASE: &str = "una frase de paso larga";
    const AHORA: &str = "2026-11-26T12:00:00Z";

    fn ok<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => panic!("se esperaba Ok y llegó: {}", primer_mensaje(&e)),
        }
    }

    fn error<T>(r: Result<T, ErroresDto>) -> String {
        match r {
            Ok(_) => panic!("se esperaba un error"),
            Err(e) => primer_mensaje(&e),
        }
    }

    fn primer_mensaje(e: &ErroresDto) -> String {
        e.errores
            .first()
            .map(|c| c.mensaje.clone())
            .unwrap_or_default()
    }

    /// Cifrador de mentira que sí distingue la frase y no deja el texto legible.
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

    struct Instalacion {
        casos: CasosDeUso,
        ruta_estado: PathBuf,
        secretos: AlmacenSecretosEnMemoria,
    }

    fn carpeta(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("cc-destino-{}-{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    fn instalacion(base: &Path, id: &str, prefijo: &str) -> Instalacion {
        Instalacion {
            casos: CasosDeUso::nuevo(
                Box::new(RepositorioEnMemoria::nuevo()),
                Box::new(RelojFijo::nuevo(AHORA)),
                Box::new(Prefijo {
                    prefijo: prefijo.to_string(),
                    siguiente: 0,
                }),
                Box::new(DispositivoFijo::nuevo(id)),
            ),
            ruta_estado: base.join(format!("{id}.sync.json")),
            secretos: AlmacenSecretosEnMemoria::nuevo(),
        }
    }

    fn a_carpeta(i: &Instalacion, destino: &Path) {
        ok(configurar_destino(
            &i.ruta_estado,
            &i.secretos,
            ConfigurarDestinoDto::Carpeta {
                ubicacion: destino.to_string_lossy().into_owned(),
                frase: FRASE.to_string(),
            },
        ));
    }

    fn cuenta(i: &mut Instalacion, nombre: &str) {
        i.casos
            .crear_cuenta(NuevaCuenta {
                nombre: nombre.to_string(),
                tipo: "efectivo".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "10.00".to_string(),
            })
            .unwrap();
    }

    fn sincronizar_i(i: &mut Instalacion) -> Result<InformeDto, ErroresDto> {
        sincronizar_con_destino(
            &mut i.casos,
            &i.ruta_estado,
            &i.secretos,
            &CifradorConFrase,
            AHORA,
        )
    }

    fn ids(i: &Instalacion) -> Vec<String> {
        let mut v: Vec<String> = i
            .casos
            .eventos_del_libro()
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        v.sort();
        v
    }

    fn archivos_de(dir: &Path, acumulado: &mut Vec<PathBuf>) {
        for entrada in fs::read_dir(dir).unwrap().flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                archivos_de(&ruta, acumulado);
            } else {
                acumulado.push(ruta);
            }
        }
    }

    #[test]
    fn la_frase_va_al_almacen_de_secretos_y_no_al_archivo() {
        let base = carpeta("secretos");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let i = instalacion(&base, "laptop", "lap");
        a_carpeta(&i, &destino);

        let estado = ok(estado_del_destino(&i.ruta_estado));
        assert_eq!(estado.destino.as_ref().unwrap().tipo, "carpeta");
        assert_eq!(estado.ultima_sincronizacion, None);
        assert_eq!(
            i.secretos.leer_frase_de_paso().unwrap().as_deref(),
            Some(FRASE)
        );
        assert!(!fs::read_to_string(&i.ruta_estado).unwrap().contains(FRASE));
    }

    #[test]
    fn frase_corta_carpeta_inexistente_y_git_se_rechazan_sin_dejar_destino() {
        let base = carpeta("rechazos");
        let i = instalacion(&base, "laptop", "lap");
        let existente = base.join("existe");
        fs::create_dir_all(&existente).unwrap();
        let carpeta_dto = |ubicacion: &Path, frase: &str| ConfigurarDestinoDto::Carpeta {
            ubicacion: ubicacion.to_string_lossy().into_owned(),
            frase: frase.to_string(),
        };

        assert!(
            error(configurar_destino(
                &i.ruta_estado,
                &i.secretos,
                carpeta_dto(&existente, "corta")
            ))
            .contains("al menos 12")
        );
        assert!(
            error(configurar_destino(
                &i.ruta_estado,
                &i.secretos,
                carpeta_dto(&base.join("no-existe"), FRASE)
            ))
            .contains("no existe")
        );
        let git = ConfigurarDestinoDto::Git {
            ubicacion: "repo".to_string(),
            usuario: "ana".to_string(),
            token: "tok".to_string(),
            frase: FRASE.to_string(),
        };
        assert!(
            error(configurar_destino(&i.ruta_estado, &i.secretos, git))
                .contains("todavía no está disponible")
        );
        assert!(ok(estado_del_destino(&i.ruta_estado)).destino.is_none());
        assert_eq!(i.secretos.leer_frase_de_paso().unwrap(), None);
    }

    #[test]
    fn sin_destino_o_sin_frase_da_un_mensaje_claro() {
        let base = carpeta("sin-destino");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let mut i = instalacion(&base, "laptop", "lap");
        assert!(error(sincronizar_i(&mut i)).contains("No hay un destino"));

        a_carpeta(&i, &destino);
        i.secretos.borrar_frase_de_paso().unwrap();
        assert!(error(sincronizar_i(&mut i)).contains("frase de paso"));
    }

    #[test]
    fn dos_instalaciones_convergen_con_una_carpeta_compartida() {
        let base = carpeta("convergen");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let mut laptop = instalacion(&base, "laptop", "lap");
        let mut telefono = instalacion(&base, "telefono", "tel");
        a_carpeta(&laptop, &destino);
        a_carpeta(&telefono, &destino);

        cuenta(&mut laptop, "Efectivo");
        cuenta(&mut laptop, "Banco");
        let primera = ok(sincronizar_i(&mut laptop));
        assert_eq!((primera.enviados, primera.recibidos), (2, 0));
        assert_eq!(primera.ultima_sincronizacion, AHORA);
        let bajada = ok(sincronizar_i(&mut telefono));
        assert_eq!((bajada.enviados, bajada.recibidos), (0, 2));

        cuenta(&mut telefono, "Cafe");
        cuenta(&mut laptop, "Almuerzo");
        let a = ok(sincronizar_i(&mut telefono));
        assert_eq!((a.enviados, a.recibidos), (1, 0));
        let b = ok(sincronizar_i(&mut laptop));
        assert_eq!((b.enviados, b.recibidos), (1, 1));
        let c = ok(sincronizar_i(&mut telefono));
        assert_eq!((c.enviados, c.recibidos), (0, 1));

        assert_eq!(ids(&laptop).len(), 4);
        assert_eq!(ids(&laptop), ids(&telefono));
        for i in [&mut laptop, &mut telefono] {
            let repetida = ok(sincronizar_i(i));
            assert_eq!((repetida.enviados, repetida.recibidos), (0, 0));
        }
        assert_eq!(
            ok(estado_del_destino(&laptop.ruta_estado)).ultima_sincronizacion,
            Some(AHORA.to_string())
        );
    }

    #[test]
    fn la_carpeta_compartida_solo_tiene_archivos_cifrados() {
        let base = carpeta("privacidad");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let mut laptop = instalacion(&base, "laptop", "lap");
        a_carpeta(&laptop, &destino);
        cuenta(&mut laptop, "Efectivo");
        ok(sincronizar_i(&mut laptop));

        let mut archivos = Vec::new();
        archivos_de(&destino, &mut archivos);
        assert!(!archivos.is_empty());
        for ruta in archivos {
            let bytes = fs::read(&ruta).unwrap();
            let texto = String::from_utf8_lossy(&bytes);
            assert!(!texto.contains("Efectivo"), "se lee el libro en {ruta:?}");
        }
    }

    #[test]
    fn destino_inaccesible_no_pierde_nada_ni_cambia_el_estado() {
        let base = carpeta("inaccesible");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let mut laptop = instalacion(&base, "laptop", "lap");
        a_carpeta(&laptop, &destino);
        cuenta(&mut laptop, "Efectivo");
        fs::remove_dir_all(&destino).unwrap();

        assert!(error(sincronizar_i(&mut laptop)).contains("No se pudo llegar"));
        assert_eq!(
            ok(estado_del_destino(&laptop.ruta_estado)).ultima_sincronizacion,
            None
        );
        assert_eq!(ids(&laptop).len(), 1);

        fs::create_dir_all(&destino).unwrap();
        let informe = ok(sincronizar_i(&mut laptop));
        assert_eq!(informe.enviados, 1);
    }

    #[test]
    fn quitar_el_destino_borra_la_configuracion_y_los_secretos_pero_no_el_libro() {
        let base = carpeta("quitar");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let mut laptop = instalacion(&base, "laptop", "lap");
        a_carpeta(&laptop, &destino);
        cuenta(&mut laptop, "Efectivo");
        ok(sincronizar_i(&mut laptop));
        let mut antes = Vec::new();
        archivos_de(&destino, &mut antes);

        ok(quitar_destino(&laptop.ruta_estado, &laptop.secretos));
        assert!(
            ok(estado_del_destino(&laptop.ruta_estado))
                .destino
                .is_none()
        );
        assert_eq!(laptop.secretos.leer_frase_de_paso().unwrap(), None);
        assert_eq!(ids(&laptop).len(), 1);
        let mut despues = Vec::new();
        archivos_de(&destino, &mut despues);
        assert_eq!(antes, despues);
        ok(quitar_destino(&laptop.ruta_estado, &laptop.secretos));
    }

    #[test]
    fn actualizar_credenciales_solo_aplica_a_un_destino_git() {
        let base = carpeta("credenciales");
        let destino = base.join("compartida");
        fs::create_dir_all(&destino).unwrap();
        let i = instalacion(&base, "laptop", "lap");
        a_carpeta(&i, &destino);
        let datos = || CredencialesDto {
            usuario: "ana".to_string(),
            token: "tok-secreto".to_string(),
        };
        assert!(
            error(actualizar_credenciales(
                &i.ruta_estado,
                &i.secretos,
                datos()
            ))
            .contains("no es un repositorio Git")
        );

        // Simula un destino Git ya guardado: el adaptador llega después.
        let mut archivo = ok(leer(&i.ruta_estado));
        archivo.destino = Some(DestinoGuardado {
            tipo: TIPO_GIT.to_string(),
            ubicacion: "repo".to_string(),
            usuario: Some("viejo".to_string()),
        });
        ok(guardar(&i.ruta_estado, &archivo));
        ok(actualizar_credenciales(
            &i.ruta_estado,
            &i.secretos,
            datos(),
        ));

        let guardadas = i.secretos.leer_credenciales_git().unwrap().unwrap();
        assert_eq!(guardadas.usuario, "ana");
        let estado = ok(estado_del_destino(&i.ruta_estado));
        assert_eq!(estado.destino.unwrap().usuario.as_deref(), Some("ana"));
        assert!(
            !fs::read_to_string(&i.ruta_estado)
                .unwrap()
                .contains("tok-secreto")
        );
    }
}
