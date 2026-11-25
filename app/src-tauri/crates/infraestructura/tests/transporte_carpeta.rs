//! Pruebas del adaptador de carpeta compartida (Entrega 8: Ernesto - Tarea 3 y Tarea 4;
//! RF-INT-07, RNF-14).
//!
//! Usa carpetas temporales reales. La primera prueba pasa al adaptador por la misma
//! batería de contrato que el falso en memoria y el de Git; el resto comprueba lo
//! propio de una carpeta: escritura con nombre temporal y renombrado, archivos
//! ajenos que se ignoran, una carpeta que desaparece.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aplicacion::contrato_transporte::{BancoDeDestino, bateria_contrato_transporte};
use aplicacion::{ErrorTransporte, TransporteSincronizacion};
use infraestructura::transporte_carpeta::TransporteCarpeta;

static CONTADOR: AtomicU32 = AtomicU32::new(0);

/// Carpeta temporal única que se borra al terminar la prueba.
struct BancoCarpeta {
    base: PathBuf,
}

impl BancoCarpeta {
    fn nuevo() -> BancoCarpeta {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let base =
            std::env::temp_dir().join(format!("cc_transporte_{}_{nanos}_{n}", std::process::id()));
        fs::create_dir_all(base.join("destino")).unwrap();
        BancoCarpeta { base }
    }

    /// La carpeta compartida que el usuario eligió.
    fn destino(&self) -> PathBuf {
        self.base.join("destino")
    }

    fn oculto(&self) -> PathBuf {
        self.base.join("destino_oculto")
    }

    fn transporte(&self, dispositivo: &str) -> TransporteCarpeta {
        TransporteCarpeta::nuevo(self.destino(), dispositivo).unwrap()
    }
}

impl Drop for BancoCarpeta {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl BancoDeDestino for BancoCarpeta {
    fn conectar(&self, dispositivo: &str) -> Box<dyn TransporteSincronizacion> {
        Box::new(self.transporte(dispositivo))
    }

    /// Simula un disco desmontado o una carpeta borrada: la carpeta deja de estar.
    fn dejar_inaccesible(&self) {
        fs::rename(self.destino(), self.oculto()).unwrap();
    }

    fn restablecer(&self) {
        fs::rename(self.oculto(), self.destino()).unwrap();
    }
}

fn nombres(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|n| n.to_string()).collect()
}

/// Nombres de los archivos de una carpeta, en orden.
fn archivos_en(ruta: &Path) -> Vec<String> {
    let mut archivos: Vec<String> = fs::read_dir(ruta)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    archivos.sort();
    archivos
}

// ---------- Contrato común ----------

#[test]
fn el_adaptador_de_carpeta_cumple_el_contrato() {
    bateria_contrato_transporte(&|| -> Box<dyn BancoDeDestino> { Box::new(BancoCarpeta::nuevo()) });
}

// ---------- Lo propio de una carpeta ----------

#[test]
fn cada_dispositivo_escribe_solo_en_su_propia_carpeta() {
    let banco = BancoCarpeta::nuevo();
    let mut laptop = banco.transporte("laptop");
    laptop.escribir_lote("lote-0001", b"uno").unwrap();

    assert_eq!(archivos_en(&banco.destino()), nombres(&["laptop"]));
    assert_eq!(
        archivos_en(&banco.destino().join("laptop")),
        nombres(&["lote-0001.lote"])
    );
    assert!(!banco.destino().join("telefono").exists());
}

#[test]
fn el_lote_se_escribe_con_nombre_temporal_y_no_deja_restos() {
    let banco = BancoCarpeta::nuevo();
    let mut laptop = banco.transporte("laptop");
    laptop.escribir_lote("lote-0001", b"contenido").unwrap();
    laptop.escribir_lote("lote-0002", b"otro").unwrap();

    assert_eq!(
        archivos_en(&banco.destino().join("laptop")),
        nombres(&["lote-0001.lote", "lote-0002.lote"]),
        "no debe quedar ningún .tmp"
    );
}

#[test]
fn los_temporales_y_los_archivos_ajenos_se_ignoran_al_leer() {
    let banco = BancoCarpeta::nuevo();
    let telefono = banco.destino().join("telefono");
    fs::create_dir_all(&telefono).unwrap();
    // Un lote terminado y varias cosas que no lo son.
    fs::write(telefono.join("lote-0002.lote"), b"bueno").unwrap();
    fs::write(telefono.join("lote-0001.lote.tmp"), b"a medio escribir").unwrap();
    fs::write(telefono.join("notas.txt"), b"nada").unwrap();
    fs::write(
        telefono.join("lote-0003 (copia en conflicto).lote"),
        b"copia",
    )
    .unwrap();
    fs::write(telefono.join(".lote-0004.lote"), b"oculto").unwrap();
    fs::create_dir_all(telefono.join("lote-0005.lote")).unwrap();

    let laptop = banco.transporte("laptop");
    assert_eq!(
        laptop.listar_lotes("telefono").unwrap(),
        nombres(&["lote-0002"])
    );
    assert_eq!(
        laptop.leer_lote("telefono", "lote-0002").unwrap(),
        b"bueno".to_vec()
    );
    // El temporal no cuenta como lote: leerlo por su nombre no lo encuentra.
    assert!(matches!(
        laptop.leer_lote("telefono", "lote-0001"),
        Err(ErrorTransporte::LoteNoExiste { .. })
    ));
}

#[test]
fn un_dispositivo_con_solo_temporales_no_aparece() {
    let banco = BancoCarpeta::nuevo();
    let tableta = banco.destino().join("tableta");
    fs::create_dir_all(&tableta).unwrap();
    fs::write(tableta.join("lote-0001.lote.tmp"), b"a medio escribir").unwrap();
    fs::create_dir_all(banco.destino().join("vacio")).unwrap();

    let laptop = banco.transporte("laptop");
    assert!(laptop.listar_dispositivos().unwrap().is_empty());
}

#[test]
fn las_carpetas_de_otros_programas_no_son_dispositivos() {
    let banco = BancoCarpeta::nuevo();
    for oculta in [".git", ".stfolder", ".dropbox.cache"] {
        let carpeta = banco.destino().join(oculta);
        fs::create_dir_all(&carpeta).unwrap();
        fs::write(carpeta.join("lote-0001.lote"), b"no es mio").unwrap();
    }
    fs::write(banco.destino().join("LEEME.txt"), b"hola").unwrap();

    let mut telefono = banco.transporte("telefono");
    telefono.escribir_lote("lote-0001", b"uno").unwrap();
    let laptop = banco.transporte("laptop");
    assert_eq!(
        laptop.listar_dispositivos().unwrap(),
        nombres(&["telefono"])
    );
}

#[test]
fn un_lote_ya_escrito_no_cambia_en_disco() {
    let banco = BancoCarpeta::nuevo();
    let mut laptop = banco.transporte("laptop");
    laptop.escribir_lote("lote-0001", b"original").unwrap();
    let ruta = banco.destino().join("laptop").join("lote-0001.lote");

    let intento = laptop.escribir_lote("lote-0001", b"distinto");
    assert!(matches!(intento, Err(ErrorTransporte::LoteYaExiste { .. })));
    assert_eq!(fs::read(&ruta).unwrap(), b"original".to_vec());
    assert_eq!(
        archivos_en(&banco.destino().join("laptop")),
        nombres(&["lote-0001.lote"])
    );
}

#[test]
fn un_temporal_abandonado_por_un_cierre_brusco_no_impide_reintentar() {
    let banco = BancoCarpeta::nuevo();
    let propia = banco.destino().join("laptop");
    fs::create_dir_all(&propia).unwrap();
    fs::write(
        propia.join("lote-0001.lote.tmp"),
        b"restos de un cierre brusco",
    )
    .unwrap();

    let mut laptop = banco.transporte("laptop");
    assert!(laptop.listar_lotes("laptop").unwrap().is_empty());
    laptop.escribir_lote("lote-0001", b"completo").unwrap();

    assert_eq!(
        fs::read(propia.join("lote-0001.lote")).unwrap(),
        b"completo".to_vec()
    );
    assert_eq!(archivos_en(&propia), nombres(&["lote-0001.lote"]));
}

#[test]
fn los_bytes_en_disco_son_exactamente_los_escritos() {
    let banco = BancoCarpeta::nuevo();
    let mut laptop = banco.transporte("laptop");
    let contenido: Vec<u8> = (0..=255).collect();
    laptop.escribir_lote("binario", &contenido).unwrap();
    let ruta = banco.destino().join("laptop").join("binario.lote");
    assert_eq!(fs::read(ruta).unwrap(), contenido);
}

#[test]
fn una_carpeta_inexistente_es_un_destino_inaccesible_y_no_se_crea() {
    let banco = BancoCarpeta::nuevo();
    let falta = banco.base.join("no-existe");
    let mut laptop = TransporteCarpeta::nuevo(&falta, "laptop").unwrap();

    assert_eq!(laptop.traer(), Err(ErrorTransporte::DestinoInaccesible));
    assert_eq!(laptop.publicar(), Err(ErrorTransporte::DestinoInaccesible));
    assert_eq!(
        laptop.listar_dispositivos(),
        Err(ErrorTransporte::DestinoInaccesible)
    );
    assert_eq!(
        laptop.listar_lotes("laptop"),
        Err(ErrorTransporte::DestinoInaccesible)
    );
    assert_eq!(
        laptop.escribir_lote("lote-0001", b"uno"),
        Err(ErrorTransporte::DestinoInaccesible)
    );
    assert!(!falta.exists(), "no se debe crear la carpeta en silencio");
}

#[test]
fn el_transporte_rechaza_un_dispositivo_con_nombre_peligroso() {
    let banco = BancoCarpeta::nuevo();
    for nombre in ["", "..", "a/b", "../fuera", "con espacio"] {
        assert!(
            matches!(
                TransporteCarpeta::nuevo(banco.destino(), nombre),
                Err(ErrorTransporte::NombreInvalido(_))
            ),
            "debía rechazar {nombre:?}"
        );
    }
}

#[test]
fn dos_instancias_sobre_la_misma_carpeta_convergen_en_cualquier_orden() {
    // El ejemplo de la sección 2.6 a nivel de transporte: cada dispositivo escribe sin
    // conexión y luego sincronizan; el resultado no depende de quién va primero.
    let mut finales = Vec::new();
    for laptop_primero in [true, false] {
        let banco = BancoCarpeta::nuevo();
        let mut laptop = banco.transporte("laptop");
        let mut telefono = banco.transporte("telefono");
        laptop
            .escribir_lote("lote-0001", b"almuerzo y cafe")
            .unwrap();
        telefono
            .escribir_lote("lote-0001", b"ingreso y gasto")
            .unwrap();

        if laptop_primero {
            sincronizar(&mut laptop);
            sincronizar(&mut telefono);
            sincronizar(&mut laptop);
        } else {
            sincronizar(&mut telefono);
            sincronizar(&mut laptop);
            sincronizar(&mut telefono);
        }
        let vista_laptop = vista_completa(&laptop);
        assert_eq!(vista_laptop, vista_completa(&telefono));
        assert_eq!(vista_laptop.len(), 2);

        // Repetir la sincronización no cambia nada.
        sincronizar(&mut laptop);
        sincronizar(&mut telefono);
        assert_eq!(vista_completa(&laptop), vista_laptop);
        assert_eq!(vista_completa(&telefono), vista_laptop);
        finales.push(vista_laptop);
    }
    assert_eq!(finales[0], finales[1]);
}

/// Una sincronización completa desde el punto de vista del transporte: traer y publicar.
fn sincronizar(t: &mut TransporteCarpeta) {
    t.traer().unwrap();
    t.publicar().unwrap();
}

/// Todo lo que un dispositivo ve: (dispositivo, lote, contenido), en orden.
fn vista_completa(t: &TransporteCarpeta) -> Vec<(String, String, Vec<u8>)> {
    let mut todo = Vec::new();
    for dispositivo in t.listar_dispositivos().unwrap() {
        for lote in t.listar_lotes(&dispositivo).unwrap() {
            let contenido = t.leer_lote(&dispositivo, &lote).unwrap();
            todo.push((dispositivo.clone(), lote, contenido));
        }
    }
    todo
}
