//! Pruebas del adaptador Git (Entrega 8: John - Tarea 5; RF-INT-03, RF-INT-07, RNF-10,
//! RNF-14).
//!
//! Usan repositorios Git reales: un repositorio «desnudo» en una carpeta temporal hace de
//! remoto (como GitHub) y cada dispositivo tiene su copia local. Necesitan el programa
//! `git` instalado. La primera prueba pasa al adaptador por la misma batería de contrato
//! que el falso en memoria y el de carpeta; el resto comprueba lo propio de Git.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aplicacion::contrato_transporte::{BancoDeDestino, bateria_contrato_transporte};
use aplicacion::secretos::CredencialesGit;
use aplicacion::{ErrorTransporte, TransporteSincronizacion, lotes_por_leer};
use infraestructura::transporte_git::{RAMA, TransporteGit, clasificar_error_git};

static CONTADOR: AtomicU32 = AtomicU32::new(0);

fn git(directorio: &Path, argumentos: &[&str]) -> String {
    let salida = Command::new("git")
        .arg("-C")
        .arg(directorio)
        .args(argumentos)
        .env("LC_ALL", "C")
        .output()
        .expect("estas pruebas necesitan el programa git");
    assert!(
        salida.status.success(),
        "git {argumentos:?} falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    String::from_utf8_lossy(&salida.stdout).into_owned()
}

/// Un remoto desnudo y las copias locales de cada dispositivo, en una carpeta temporal.
struct BancoGit {
    base: PathBuf,
}

impl BancoGit {
    fn nuevo() -> BancoGit {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("cc_git_{}_{nanos}_{n}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        git(&base, &["init", "-q", "--bare", "remoto.git"]);
        BancoGit { base }
    }

    fn remoto(&self) -> PathBuf {
        self.base.join("remoto.git")
    }

    fn oculto(&self) -> PathBuf {
        self.base.join("remoto-oculto.git")
    }

    fn copia_de(&self, dispositivo: &str) -> PathBuf {
        self.base.join("copias").join(dispositivo)
    }

    fn transporte(&self, dispositivo: &str) -> TransporteGit {
        TransporteGit::nuevo(
            self.remoto().to_str().unwrap(),
            self.copia_de(dispositivo),
            dispositivo,
            None,
        )
        .unwrap()
    }

    /// Archivos de la rama `main` del remoto.
    fn archivos_del_remoto(&self) -> Vec<String> {
        let mut archivos: Vec<String> =
            git(&self.remoto(), &["ls-tree", "-r", "--name-only", RAMA])
                .lines()
                .map(str::to_string)
                .collect();
        archivos.sort();
        archivos
    }
}

impl Drop for BancoGit {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl BancoDeDestino for BancoGit {
    fn conectar(&self, dispositivo: &str) -> Box<dyn TransporteSincronizacion> {
        Box::new(self.transporte(dispositivo))
    }

    /// Simula el remoto caído o sin red: el repositorio deja de estar donde se esperaba.
    fn dejar_inaccesible(&self) {
        fs::rename(self.remoto(), self.oculto()).unwrap();
    }

    fn restablecer(&self) {
        fs::rename(self.oculto(), self.remoto()).unwrap();
    }
}

fn nombres(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|n| n.to_string()).collect()
}

/// Todo lo que un dispositivo ve: (dispositivo, lote, contenido), en orden.
fn vista_completa(t: &dyn TransporteSincronizacion) -> Vec<(String, String, Vec<u8>)> {
    let mut todo = Vec::new();
    for dispositivo in t.listar_dispositivos().unwrap() {
        for lote in t.listar_lotes(&dispositivo).unwrap() {
            let contenido = t.leer_lote(&dispositivo, &lote).unwrap();
            todo.push((dispositivo.clone(), lote, contenido));
        }
    }
    todo
}

// ---------- Contrato común ----------

#[test]
fn el_adaptador_git_cumple_el_contrato() {
    bateria_contrato_transporte(&|| -> Box<dyn BancoDeDestino> { Box::new(BancoGit::nuevo()) });
}

// ---------- Lo propio de Git ----------

#[test]
fn cada_dispositivo_escribe_en_su_carpeta_del_repositorio() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    let mut telefono = banco.transporte("telefono");
    laptop
        .escribir_lote("20261124T120000Z-0000", b"uno")
        .unwrap();
    laptop.publicar().unwrap();
    telefono.traer().unwrap();
    telefono
        .escribir_lote("20261124T130000Z-0000", b"dos")
        .unwrap();
    telefono.publicar().unwrap();

    assert_eq!(
        banco.archivos_del_remoto(),
        nombres(&[
            "laptop/20261124T120000Z-0000.lote",
            "telefono/20261124T130000Z-0000.lote"
        ])
    );
}

#[test]
fn escribir_funciona_sin_conexion_y_se_publica_al_volver() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    let mut telefono = banco.transporte("telefono");

    banco.dejar_inaccesible();
    // La copia local se crea y el lote se confirma sin tocar la red.
    laptop.escribir_lote("lote-0001", b"sin red").unwrap();
    assert_eq!(
        laptop.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"])
    );
    assert_eq!(laptop.publicar(), Err(ErrorTransporte::DestinoInaccesible));
    assert_eq!(telefono.traer(), Err(ErrorTransporte::DestinoInaccesible));

    banco.restablecer();
    laptop.publicar().unwrap();
    telefono.traer().unwrap();
    assert_eq!(
        telefono.leer_lote("laptop", "lote-0001").unwrap(),
        b"sin red".to_vec()
    );
}

#[test]
fn si_otro_publico_a_la_vez_se_vuelve_a_traer_y_se_reintenta_sin_conflictos() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    let mut telefono = banco.transporte("telefono");
    let mut tableta = banco.transporte("tableta");
    // Los tres escriben sin haber visto lo de los demás.
    laptop.escribir_lote("lote-0001", b"laptop").unwrap();
    telefono.escribir_lote("lote-0001", b"telefono").unwrap();
    tableta.escribir_lote("lote-0001", b"tableta").unwrap();

    laptop.publicar().unwrap();
    // El remoto ya cambió: el empuje se rechaza y el adaptador integra y reintenta solo.
    telefono.publicar().unwrap();
    tableta.publicar().unwrap();

    for t in [&mut laptop, &mut telefono, &mut tableta] {
        t.traer().unwrap();
    }
    let vista = vista_completa(&laptop);
    assert_eq!(vista.len(), 3);
    assert_eq!(vista_completa(&telefono), vista);
    assert_eq!(vista_completa(&tableta), vista);
    // La historia es lineal: ningún commit de fusión.
    let fusiones = git(&banco.remoto(), &["rev-list", "--merges", RAMA]);
    assert!(
        fusiones.trim().is_empty(),
        "no debía haber fusiones: {fusiones}"
    );
}

#[test]
fn publicar_sin_nada_pendiente_no_usa_la_red() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    laptop.escribir_lote("lote-0001", b"uno").unwrap();
    laptop.publicar().unwrap();

    banco.dejar_inaccesible();
    assert_eq!(laptop.publicar(), Ok(()), "no había nada que publicar");
    banco.restablecer();
}

#[test]
fn traer_solo_transfiere_lo_nuevo_y_solo_se_leen_los_lotes_que_faltan() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    let mut telefono = banco.transporte("telefono");
    let mut leidos = BTreeSet::new();

    laptop.escribir_lote("lote-0001", b"primero").unwrap();
    laptop.publicar().unwrap();
    telefono.traer().unwrap();
    let primera = lotes_por_leer(&telefono, &leidos).unwrap();
    assert_eq!(
        primera,
        vec![("laptop".to_string(), "lote-0001".to_string())]
    );
    leidos.extend(primera);

    // Sin novedades, traer no descarga nada y no queda nada por leer.
    let antes = git(&banco.copia_de("telefono"), &["count-objects", "-v"]);
    telefono.traer().unwrap();
    assert_eq!(
        git(&banco.copia_de("telefono"), &["count-objects", "-v"]),
        antes
    );
    assert!(lotes_por_leer(&telefono, &leidos).unwrap().is_empty());

    laptop.escribir_lote("lote-0002", b"segundo").unwrap();
    laptop.publicar().unwrap();
    telefono.traer().unwrap();
    assert_eq!(
        lotes_por_leer(&telefono, &leidos).unwrap(),
        vec![("laptop".to_string(), "lote-0002".to_string())]
    );
}

#[test]
fn los_bytes_llegan_identicos_sin_conversion_de_fines_de_linea() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    let mut telefono = banco.transporte("telefono");
    let contenido: Vec<u8> = (0..=255u8).chain(b"\r\n\n\r".iter().copied()).collect();
    laptop.escribir_lote("binario", &contenido).unwrap();
    laptop.publicar().unwrap();
    telefono.traer().unwrap();
    assert_eq!(telefono.leer_lote("laptop", "binario").unwrap(), contenido);
}

#[test]
fn las_credenciales_no_quedan_en_la_copia_local() {
    let banco = BancoGit::nuevo();
    let mut laptop = TransporteGit::nuevo(
        banco.remoto().to_str().unwrap(),
        banco.copia_de("laptop"),
        "laptop",
        Some(CredencialesGit::nuevas("ana", "tok-super-secreto")),
    )
    .unwrap();
    laptop.escribir_lote("lote-0001", b"uno").unwrap();
    laptop.publicar().unwrap();

    let config = fs::read_to_string(banco.copia_de("laptop").join(".git/config")).unwrap();
    assert!(!config.contains("tok-super-secreto"), "{config}");
    assert!(!config.contains("ana"), "{config}");
}

#[test]
fn un_repositorio_que_no_existe_es_un_destino_inaccesible() {
    let banco = BancoGit::nuevo();
    let falta = banco.base.join("no-existe.git");
    let mut laptop = TransporteGit::nuevo(
        falta.to_str().unwrap(),
        banco.copia_de("laptop"),
        "laptop",
        None,
    )
    .unwrap();
    assert_eq!(laptop.traer(), Err(ErrorTransporte::DestinoInaccesible));
    assert!(ErrorTransporte::DestinoInaccesible.es_reintentable());
}

#[test]
fn un_servidor_que_no_se_resuelve_es_sin_conexion() {
    // `.invalid` está reservado: nunca se resuelve (RFC 2606). No sale ningún dato.
    let banco = BancoGit::nuevo();
    let mut laptop = TransporteGit::nuevo(
        "https://cuentas-claras.invalid/ana/libro.git",
        banco.copia_de("laptop"),
        "laptop",
        Some(CredencialesGit::nuevas("ana", "tok")),
    )
    .unwrap();
    assert_eq!(laptop.traer(), Err(ErrorTransporte::SinConexion));
}

#[test]
fn una_ubicacion_vacia_o_que_parece_una_opcion_se_rechaza() {
    for ubicacion in ["", "   ", "--upload-pack=algo", "repo\nmalo"] {
        assert!(
            matches!(
                TransporteGit::nuevo(ubicacion, "/tmp/x", "laptop", None),
                Err(ErrorTransporte::DestinoInaccesible)
            ),
            "{ubicacion:?}"
        );
    }
    assert!(matches!(
        TransporteGit::nuevo("/tmp/repo.git", "/tmp/x", "../fuera", None),
        Err(ErrorTransporte::NombreInvalido(_))
    ));
}

#[test]
fn las_otras_ramas_del_repositorio_no_se_traen() {
    let banco = BancoGit::nuevo();
    // Alguien tiene otra rama en el mismo repositorio.
    let ajeno = banco.base.join("ajeno");
    git(
        &banco.base,
        &["clone", "-q", banco.remoto().to_str().unwrap(), "ajeno"],
    );
    fs::write(ajeno.join("LEEME.md"), "hola").unwrap();
    git(&ajeno, &["checkout", "-q", "-b", "otra"]);
    git(&ajeno, &["add", "LEEME.md"]);
    git(
        &ajeno,
        &[
            "-c",
            "user.name=x",
            "-c",
            "user.email=x@x",
            "commit",
            "-q",
            "-m",
            "otra",
        ],
    );
    git(&ajeno, &["push", "-q", "origin", "otra"]);

    let mut laptop = banco.transporte("laptop");
    laptop.traer().unwrap();
    let ramas = git(&banco.copia_de("laptop"), &["branch", "-r"]);
    assert!(!ramas.contains("otra"), "{ramas}");
    assert!(laptop.listar_dispositivos().unwrap().is_empty());
}

#[test]
fn una_copia_local_borrada_se_reconstruye_desde_el_remoto() {
    let banco = BancoGit::nuevo();
    let mut laptop = banco.transporte("laptop");
    laptop.escribir_lote("lote-0001", b"uno").unwrap();
    laptop.publicar().unwrap();

    fs::remove_dir_all(banco.copia_de("laptop")).unwrap();
    let mut laptop = banco.transporte("laptop");
    assert!(laptop.listar_lotes("laptop").unwrap().is_empty());
    laptop.traer().unwrap();
    assert_eq!(
        laptop.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"])
    );
    // Un lote ya publicado sigue siendo inmutable.
    assert!(matches!(
        laptop.escribir_lote("lote-0001", b"otro"),
        Err(ErrorTransporte::LoteYaExiste { .. })
    ));
}

// ---------- Clasificación de errores de git ----------

#[test]
fn los_errores_de_git_se_clasifican() {
    let credenciales = [
        "remote: Invalid username or password.\nfatal: Authentication failed for 'https://github.com/ana/libro.git/'",
        "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        "remote: HTTP Basic: Access denied\nfatal: Authentication failed for 'https://gitlab.com/ana/libro.git/'",
        "remote: Permission to ana/libro.git denied to beto.\nfatal: unable to access 'https://github.com/ana/libro.git/': The requested URL returned error: 403",
        "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.",
        "Host key verification failed.\nfatal: Could not read from remote repository.",
    ];
    for texto in credenciales {
        assert_eq!(
            clasificar_error_git(texto),
            ErrorTransporte::CredencialesRechazadas,
            "{texto}"
        );
    }
    let sin_conexion = [
        "fatal: unable to access 'https://github.com/ana/libro.git/': Could not resolve host: github.com",
        "ssh: Could not resolve hostname github.com: Temporary failure in name resolution\nfatal: Could not read from remote repository.",
        "fatal: unable to access 'https://github.com/ana/libro.git/': Failed to connect to github.com port 443 after 21 ms: Network is unreachable",
        "ssh: connect to host github.com port 22: Connection timed out",
    ];
    for texto in sin_conexion {
        assert_eq!(
            clasificar_error_git(texto),
            ErrorTransporte::SinConexion,
            "{texto}"
        );
    }
    let cambio = [
        " ! [rejected]        HEAD -> main (fetch first)\nerror: failed to push some refs to 'https://github.com/ana/libro.git'",
        " ! [rejected]        HEAD -> main (non-fast-forward)",
    ];
    for texto in cambio {
        assert_eq!(
            clasificar_error_git(texto),
            ErrorTransporte::DestinoCambio,
            "{texto}"
        );
    }
    let inaccesible = [
        "fatal: '/media/usb/libro.git' does not appear to be a git repository\nfatal: Could not read from remote repository.",
        "remote: Repository not found.\nfatal: repository 'https://github.com/ana/libro.git/' not found",
        "fatal: unable to access 'https://git.ejemplo.org/libro.git/': The requested URL returned error: 503",
        "fatal: unable to access 'https://git.ejemplo.org/libro.git/': Failed to connect: Connection refused",
    ];
    for texto in inaccesible {
        assert_eq!(
            clasificar_error_git(texto),
            ErrorTransporte::DestinoInaccesible,
            "{texto}"
        );
    }
    assert!(matches!(
        clasificar_error_git("fatal: algo inesperado"),
        ErrorTransporte::Interno(_)
    ));
}

#[test]
fn las_credenciales_rechazadas_no_son_reintentables_y_el_resto_si() {
    assert!(!ErrorTransporte::CredencialesRechazadas.es_reintentable());
    for e in [
        ErrorTransporte::SinConexion,
        ErrorTransporte::DestinoInaccesible,
        ErrorTransporte::DestinoCambio,
    ] {
        assert!(e.es_reintentable(), "{e:?}");
    }
}
