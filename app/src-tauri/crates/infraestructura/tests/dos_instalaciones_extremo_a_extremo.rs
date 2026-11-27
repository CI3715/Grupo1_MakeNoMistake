//! Prueba de extremo a extremo con dos instalaciones de la app (Entrega 8: Ernesto -
//! Tarea 5; flujo de la sección 2.8; ESC-03 y ESC-04; RF-INT-01 a RF-INT-04 y RF-INT-07;
//! RNF-01, RNF-10).
//!
//! Cada «instalación» es lo que arma `lib.rs` al abrir la app: su propia carpeta de datos
//! con su propio archivo `cuentas_claras.db` (SQLite real), su identificador de
//! dispositivo, el cifrado real (Argon2id + AES-256-GCM) y el caso de uso `sincronizar`
//! que usan los comandos de Tauri. Los destinos son reales: una carpeta compartida en
//! disco y un repositorio Git «desnudo» que hace de remoto (necesita el programa `git`,
//! igual que `transporte_git.rs`).
//!
//! Qué reproduce, para cada destino (carpeta y Git) y en los dos órdenes de sincronización:
//!
//! 1. **Exportar:** la copia cifrada no muestra ningún dato legible del libro.
//! 2. **Importar:** con una frase incorrecta falla sin cambiar el libro; con la correcta,
//!    importar dos veces deja «0 nuevos, N ya existían».
//! 3. **Restaurar:** una instalación nueva queda con las mismas cuentas y saldos.
//! 4. **Sincronizar:** el ejemplo de convergencia de beto (sección 2.6): 151,67 → laptop
//!    126,67 y teléfono 191,67 → ambos 166,67, con los informes de la tabla.
//! 5. **Repetir** la sincronización no cambia nada ni escribe lotes nuevos (RNF-10).
//! 6. **Privacidad:** el destino contiene solo archivos cifrados.
//!
//! Con `cargo test -p infraestructura --test dos_instalaciones_extremo_a_extremo --
//! --nocapture` se imprime, para cada caso, la tabla de la sección 2.6.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use aplicacion::sincronizar_libro::{EstadoSincronizacion, InformeSincronizacion, sincronizar};
use aplicacion::{
    CasosDeUso, ErrorRespaldo, NuevaCuenta, NuevoMovimiento, TransporteSincronizacion,
};
use infraestructura::cifrado::{CifradorArgon2, MAGIA_CONTENEDOR, ParametrosArgon2};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;
use infraestructura::transporte_carpeta::TransporteCarpeta;
use infraestructura::transporte_git::{RAMA, TransporteGit};

const FRASE: &str = "frase-de-paso-de-beto-2026";

/// Textos del libro que nunca deben verse en el destino ni en una copia cifrada.
const DATOS_DEL_LIBRO: [&str; 7] = [
    "Efectivo",
    "Ahorros en euros",
    "Almuerzo",
    "151.67",
    "CuentaCreada",
    "MovimientoRegistrado",
    "cuentas-claras-respaldo",
];

static CONTADOR: AtomicU32 = AtomicU32::new(0);

// ---------- Utilidades ----------

/// Carpeta temporal única que se borra al terminar la prueba.
struct Escenario {
    base: PathBuf,
}

impl Escenario {
    fn nuevo(nombre: &str) -> Escenario {
        let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "cc_e2e_{nombre}_{}_{n}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&base).unwrap();
        Escenario { base }
    }
}

impl Drop for Escenario {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// Argon2id con parámetros ligeros: aquí se prueba el flujo, no la dureza de la derivación
/// (esa la cubren las pruebas de `cifrado.rs`). El contenedor guarda sus parámetros, así
/// que cualquier instalación descifra lo que cifró otra.
fn cifrador() -> CifradorArgon2 {
    CifradorArgon2::con_parametros(ParametrosArgon2 {
        memoria_kib: 1024,
        iteraciones: 1,
        paralelismo: 1,
    })
}

fn git(directorio: &Path, argumentos: &[&str]) -> String {
    let salida = Command::new("git")
        .arg("-C")
        .arg(directorio)
        .args(argumentos)
        .env("LC_ALL", "C")
        .output()
        .expect("esta prueba necesita el programa git");
    assert!(
        salida.status.success(),
        "git {argumentos:?} falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    String::from_utf8_lossy(&salida.stdout).into_owned()
}

/// Todos los archivos bajo `dir` (cualquier profundidad), sin entrar en `.git`.
fn archivos_de(dir: &Path) -> Vec<PathBuf> {
    let mut encontrados = Vec::new();
    let Ok(lectura) = fs::read_dir(dir) else {
        return encontrados;
    };
    for entrada in lectura {
        let ruta = entrada.unwrap().path();
        if ruta.is_dir() {
            if ruta.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            encontrados.extend(archivos_de(&ruta));
        } else {
            encontrados.push(ruta);
        }
    }
    encontrados.sort();
    encontrados
}

fn contiene(bytes: &[u8], aguja: &str) -> bool {
    let aguja = aguja.as_bytes();
    !aguja.is_empty() && bytes.windows(aguja.len()).any(|ventana| ventana == aguja)
}

// ---------- Una instalación de la app ----------

/// Una instalación: carpeta de datos propia, libro SQLite real y lo que recuerda de su
/// última sincronización (en la app real, el archivo `.sync.json`).
struct Instalacion {
    nombre: String,
    casos: CasosDeUso,
    estado: EstadoSincronizacion,
}

impl Instalacion {
    /// Abre (o crea) la instalación `nombre` con su carpeta de datos dentro de `raiz`,
    /// como hace `lib.rs`: base persistente, dispositivo persistente, reloj e ids reales.
    fn abrir(raiz: &Path, nombre: &str) -> Instalacion {
        let datos = raiz.join("datos").join(nombre);
        fs::create_dir_all(&datos).unwrap();
        let base = BaseDatos::persistente(datos.join("cuentas_claras.db")).unwrap();
        let conexion = base.conexion();
        let dispositivo = DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion)).unwrap();
        let casos = CasosDeUso::nuevo(
            Box::new(RepositorioEventosSqlite::with_conn(conexion)),
            Box::new(RelojSistema::nuevo()),
            Box::new(GeneradorUuid::nuevo()),
            Box::new(dispositivo),
        );
        Instalacion {
            nombre: nombre.to_string(),
            casos,
            estado: EstadoSincronizacion::default(),
        }
    }

    fn id_dispositivo(&self) -> String {
        self.casos.id_dispositivo()
    }

    fn crear_cuenta(&mut self, nombre: &str, moneda: &str, saldo: &str) -> String {
        self.casos
            .crear_cuenta(NuevaCuenta {
                nombre: nombre.to_string(),
                tipo: "efectivo".to_string(),
                moneda: moneda.to_string(),
                saldo_inicial: saldo.to_string(),
            })
            .unwrap()
            .id
    }

    fn movimiento(&mut self, cuenta: &str, tipo: &str, monto: &str, categoria: &str, nota: &str) {
        self.casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.to_string(),
                tipo: tipo.to_string(),
                monto: monto.to_string(),
                fecha: "2026-11-26".to_string(),
                categoria_id: categoria.to_string(),
                nota: Some(nota.to_string()),
            })
            .unwrap();
    }

    fn ids(&self) -> BTreeSet<String> {
        self.casos
            .eventos_del_libro()
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect()
    }

    fn cantidad_de_eventos(&self) -> usize {
        self.casos.eventos_del_libro().unwrap().len()
    }

    /// Eventos que nacen en cada instalación al abrir la base (categorías de fábrica,
    /// idénticas en todas). Sirven para separar «lo que se recibe» de «lo que ya se tenía».
    fn eventos_de_fabrica(&self) -> usize {
        self.casos
            .eventos_del_libro()
            .unwrap()
            .iter()
            .filter(|e| e.dispositivo_origen == "fabrica")
            .count()
    }

    /// (nombre, saldo) de cada cuenta, ordenado: lo que muestra la pantalla de cuentas.
    fn saldos(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .casos
            .listar_cuentas()
            .unwrap()
            .cuentas
            .into_iter()
            .map(|c| (c.nombre, c.saldo_actual))
            .collect();
        v.sort();
        v
    }

    fn saldo_de(&self, cuenta: &str) -> String {
        self.saldos()
            .into_iter()
            .find(|(nombre, _)| nombre == cuenta)
            .unwrap_or_else(|| panic!("no existe la cuenta {cuenta}"))
            .1
    }

    /// «Sincronizar ahora»: lo mismo que hace `sincronizar_con_destino` en la app. El estado
    /// nuevo solo se recuerda si todo salió bien.
    fn sincronizar(&mut self, destino: &Destino, ahora: &str) -> InformeSincronizacion {
        let mut transporte = destino.transporte(&self.nombre, &self.id_dispositivo());
        let (informe, estado) = sincronizar(
            &mut self.casos,
            transporte.as_mut(),
            &cifrador(),
            FRASE,
            ahora,
            &self.estado,
        )
        .unwrap_or_else(|e| panic!("{}: la sincronización falló: {e}", self.nombre));
        self.estado = estado;
        informe
    }
}

// ---------- Los dos destinos reales ----------

/// A dónde sincronizan las dos instalaciones.
enum Destino {
    /// Una carpeta que otro servicio mantiene sincronizada.
    Carpeta { carpeta: PathBuf },
    /// Un repositorio Git remoto (desnudo) y, por dispositivo, su copia de trabajo.
    Git { remoto: PathBuf, copias: PathBuf },
}

impl Destino {
    fn carpeta(raiz: &Path) -> Destino {
        let carpeta = raiz.join("carpeta-compartida");
        fs::create_dir_all(&carpeta).unwrap();
        Destino::Carpeta { carpeta }
    }

    fn git(raiz: &Path) -> Destino {
        let remoto = raiz.join("remoto.git");
        fs::create_dir_all(raiz).unwrap();
        git(raiz, &["init", "-q", "--bare", "remoto.git"]);
        Destino::Git {
            remoto,
            copias: raiz.join("copias-git"),
        }
    }

    fn etiqueta(&self) -> &'static str {
        match self {
            Destino::Carpeta { .. } => "carpeta compartida",
            Destino::Git { .. } => "repositorio Git",
        }
    }

    fn transporte(
        &self,
        nombre_instalacion: &str,
        dispositivo: &str,
    ) -> Box<dyn TransporteSincronizacion> {
        match self {
            Destino::Carpeta { carpeta } => {
                Box::new(TransporteCarpeta::nuevo(carpeta, dispositivo).unwrap())
            }
            Destino::Git { remoto, copias } => Box::new(
                TransporteGit::nuevo(
                    remoto.to_str().unwrap(),
                    copias.join(nombre_instalacion),
                    dispositivo,
                    None,
                )
                .unwrap(),
            ),
        }
    }

    /// Lo que hay publicado en el destino (rutas relativas), en orden.
    fn publicado(&self) -> Vec<String> {
        match self {
            Destino::Carpeta { carpeta } => archivos_de(carpeta)
                .iter()
                .map(|r| {
                    r.strip_prefix(carpeta)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect(),
            Destino::Git { remoto, .. } => {
                let mut archivos: Vec<String> =
                    git(remoto, &["ls-tree", "-r", "--name-only", RAMA])
                        .lines()
                        .map(str::to_string)
                        .collect();
                archivos.sort();
                archivos
            }
        }
    }

    /// Archivos reales que alguien con acceso al destino podría abrir: en una carpeta, los
    /// archivos de la carpeta; en Git, los de una copia de trabajo (sin `.git`).
    fn archivos_visibles(&self) -> Vec<PathBuf> {
        match self {
            Destino::Carpeta { carpeta } => archivos_de(carpeta),
            Destino::Git { copias, .. } => archivos_de(copias),
        }
    }
}

/// El destino solo guarda archivos cifrados: ninguno muestra datos del libro ni la frase, y
/// todos empiezan con la marca del contenedor cifrado (RNF-01, RF-INT-06).
fn comprobar_que_el_destino_solo_tiene_archivos_cifrados(destino: &Destino) {
    let archivos = destino.archivos_visibles();
    assert!(
        !archivos.is_empty(),
        "{}: no hay archivos que revisar",
        destino.etiqueta()
    );
    for ruta in archivos {
        let bytes = fs::read(&ruta).unwrap();
        let nombre = ruta.to_string_lossy().into_owned();
        assert!(
            bytes.starts_with(MAGIA_CONTENEDOR),
            "{}: {nombre} no es un contenedor cifrado",
            destino.etiqueta()
        );
        for texto in DATOS_DEL_LIBRO.into_iter().chain([FRASE]) {
            assert!(
                !contiene(&bytes, texto) && !nombre.contains(texto),
                "{}: {nombre} deja ver «{texto}»",
                destino.etiqueta()
            );
        }
    }
}

// ---------- Pasos 1 a 3: exportar, importar y restaurar (ESC-04) ----------

/// Exporta cifrado, comprueba que no se lee nada, y restaura en una instalación nueva.
/// Devuelve la copia exportada y la instalación restaurada.
fn exportar_importar_y_restaurar(raiz: &Path, original: &Instalacion) -> (Vec<u8>, Instalacion) {
    // 1. Exportar: no modifica el libro y, cifrada, no revela ningún dato.
    let eventos_antes = original.cantidad_de_eventos();
    let copia = original
        .casos
        .exportar_copia(Some(FRASE), &cifrador())
        .unwrap();
    assert_eq!(original.cantidad_de_eventos(), eventos_antes);
    assert!(copia.starts_with(MAGIA_CONTENEDOR));
    for texto in DATOS_DEL_LIBRO {
        assert!(
            !contiene(&copia, texto),
            "la copia cifrada deja ver «{texto}»"
        );
    }

    // 3. Restaurar: una instalación nueva, con su propia carpeta de datos, queda igual.
    let mut restaurada = Instalacion::abrir(raiz, "instalacion-nueva");

    // 2a. Con una frase incorrecta: error claro y el libro no cambia.
    let antes = restaurada.cantidad_de_eventos();
    let error = restaurada
        .casos
        .importar_copia(&copia, Some("otra frase distinta"), &cifrador())
        .unwrap_err();
    assert_eq!(error, ErrorRespaldo::FraseIncorrectaOArchivoDanado);
    assert_eq!(restaurada.cantidad_de_eventos(), antes);

    // 2b. Con la frase correcta: lo que no tenía llega, y lo de fábrica ya existía.
    let primera = restaurada
        .casos
        .importar_copia(&copia, Some(FRASE), &cifrador())
        .unwrap();
    assert!(primera.rechazados.is_empty());
    assert_eq!(primera.ya_existian, restaurada.eventos_de_fabrica());
    assert_eq!(
        primera.nuevos.len() + primera.ya_existian,
        original.cantidad_de_eventos()
    );

    // 2c. Importar el mismo archivo otra vez: «0 nuevos, N ya existían» (ESC-04).
    let despues_de_una = restaurada.cantidad_de_eventos();
    let segunda = restaurada
        .casos
        .importar_copia(&copia, Some(FRASE), &cifrador())
        .unwrap();
    assert_eq!(segunda.nuevos.len(), 0);
    assert_eq!(segunda.ya_existian, original.cantidad_de_eventos());
    assert!(segunda.rechazados.is_empty());
    assert_eq!(restaurada.cantidad_de_eventos(), despues_de_una);

    // Mismos eventos, mismas cuentas y mismos saldos que el original.
    assert_eq!(restaurada.ids(), original.ids());
    assert_eq!(restaurada.saldos(), original.saldos());
    (copia, restaurada)
}

// ---------- Pasos 4 a 6: el ejemplo de convergencia de beto (ESC-03) ----------

/// (enviados, recibidos) de un informe, como en la tabla de la sección 2.6.
fn resumen(informe: &InformeSincronizacion) -> (usize, usize) {
    (informe.enviados, informe.recibidos)
}

fn imprimir(fila: &str, laptop: &Instalacion, telefono: &Instalacion, informe: &str) {
    println!(
        "  {fila:<26} laptop {:>3} ev · {:>7}   teléfono {:>3} ev · {:>7}   {informe}",
        laptop.cantidad_de_eventos(),
        laptop.saldo_de("Efectivo"),
        telefono.cantidad_de_eventos(),
        telefono.saldo_de("Efectivo"),
    );
}

/// El ejemplo de la sección 2.6 con dos instalaciones y un destino real.
fn escenario_de_beto(destino: &Destino, raiz: &Path, telefono_primero: bool) {
    println!(
        "\n== {} · sincroniza primero {} ==",
        destino.etiqueta(),
        if telefono_primero {
            "el teléfono"
        } else {
            "la laptop"
        }
    );

    // Libro de partida: Efectivo USD con saldo 151,67 y una cuenta más.
    let mut laptop = Instalacion::abrir(raiz, "laptop");
    let efectivo = laptop.crear_cuenta("Efectivo", "USD", "100.00");
    laptop.crear_cuenta("Ahorros en euros", "EUR", "500.00");
    laptop.movimiento(&efectivo, "ingreso", "80.00", "ingreso:salario", "Quincena");
    laptop.movimiento(
        &efectivo,
        "egreso",
        "28.33",
        "gasto:alimentacion",
        "Mercado",
    );
    assert_eq!(laptop.saldo_de("Efectivo"), "151.67");

    // Pasos 1 a 3 del flujo: exportar, importar (frase mala, buena, dos veces) y restaurar.
    let (copia, restaurada) = exportar_importar_y_restaurar(raiz, &laptop);
    assert_eq!(restaurada.saldo_de("Efectivo"), "151.67");
    drop(restaurada);

    // El teléfono se une al destino con la misma frase: la primera sincronización trae
    // todo el libro. El teléfono parte del mismo libro que la laptop (sección 2.6).
    let mut telefono = Instalacion::abrir(raiz, "telefono");
    let enviados_por_la_laptop = laptop.sincronizar(destino, "2026-11-26T09:00:00Z");
    assert_eq!(
        enviados_por_la_laptop.enviados,
        laptop.cantidad_de_eventos()
    );
    let union = telefono.sincronizar(destino, "2026-11-26T09:01:00Z");
    assert_eq!(
        union.recibidos,
        laptop.cantidad_de_eventos() - telefono.eventos_de_fabrica()
    );
    assert!(union.rechazados.is_empty());
    assert_eq!(laptop.ids(), telefono.ids());
    assert_eq!(laptop.saldos(), telefono.saldos());
    let eventos_de_partida = laptop.cantidad_de_eventos();
    imprimir("Inicio", &laptop, &telefono, "—");

    // Sin conexión, cada uno registra lo suyo: laptop −20,00 y −5,00; teléfono +50,00 y −10,00.
    let efectivo_laptop = efectivo;
    let efectivo_telefono = telefono
        .casos
        .listar_cuentas()
        .unwrap()
        .cuentas
        .into_iter()
        .find(|c| c.nombre == "Efectivo")
        .unwrap()
        .id;
    assert_eq!(efectivo_laptop, efectivo_telefono);
    laptop.movimiento(
        &efectivo_laptop,
        "egreso",
        "20.00",
        "gasto:alimentacion",
        "Almuerzo",
    );
    laptop.movimiento(
        &efectivo_laptop,
        "egreso",
        "5.00",
        "gasto:alimentacion",
        "Café",
    );
    telefono.movimiento(
        &efectivo_telefono,
        "ingreso",
        "50.00",
        "ingreso:salario",
        "Ingreso",
    );
    telefono.movimiento(
        &efectivo_telefono,
        "egreso",
        "10.00",
        "gasto:alimentacion",
        "Gasto",
    );
    assert_eq!(laptop.cantidad_de_eventos(), eventos_de_partida + 2);
    assert_eq!(telefono.cantidad_de_eventos(), eventos_de_partida + 2);
    assert_eq!(laptop.saldo_de("Efectivo"), "126.67");
    assert_eq!(telefono.saldo_de("Efectivo"), "191.67");
    imprimir("Movimientos sin conexión", &laptop, &telefono, "—");

    // Sincronizan en el orden pedido: primero, segundo y de nuevo el primero.
    {
        let (primero, segundo, nombres) = if telefono_primero {
            (&mut telefono, &mut laptop, ("Teléfono", "Laptop"))
        } else {
            (&mut laptop, &mut telefono, ("Laptop", "Teléfono"))
        };

        let uno = primero.sincronizar(destino, "2026-11-26T10:00:00Z");
        assert_eq!(resumen(&uno), (2, 0), "{} sincroniza primero", nombres.0);
        assert!(uno.rechazados.is_empty());
        println!(
            "  {} sincroniza: {} enviados, {} recibidos",
            nombres.0, uno.enviados, uno.recibidos
        );

        let dos = segundo.sincronizar(destino, "2026-11-26T10:01:00Z");
        assert_eq!(resumen(&dos), (2, 2), "{} sincroniza después", nombres.1);
        assert!(dos.rechazados.is_empty());
        println!(
            "  {} sincroniza: {} enviados, {} recibidos",
            nombres.1, dos.enviados, dos.recibidos
        );

        let tres = primero.sincronizar(destino, "2026-11-26T10:02:00Z");
        assert_eq!(resumen(&tres), (0, 2), "{} sincroniza otra vez", nombres.0);
        assert!(tres.rechazados.is_empty());
        println!(
            "  {} sincroniza: {} enviados, {} recibidos",
            nombres.0, tres.enviados, tres.recibidos
        );
    }

    // Mismos eventos y mismos saldos, sin perder ni duplicar nada: 44 en el ejemplo.
    assert_eq!(laptop.cantidad_de_eventos(), eventos_de_partida + 4);
    assert_eq!(laptop.ids(), telefono.ids());
    assert_eq!(laptop.saldos(), telefono.saldos());
    assert_eq!(laptop.saldo_de("Efectivo"), "166.67");
    assert_eq!(telefono.saldo_de("Efectivo"), "166.67");
    imprimir(
        "Convergencia",
        &laptop,
        &telefono,
        "mismos eventos y saldos",
    );

    // Repetir la sincronización no cambia nada y no escribe lotes nuevos (RNF-10).
    let publicado = destino.publicado();
    let ids = laptop.ids();
    let saldos = laptop.saldos();
    for (i, ahora) in ["2026-11-26T11:00:00Z", "2026-11-26T11:01:00Z"]
        .iter()
        .enumerate()
    {
        let a = laptop.sincronizar(destino, ahora);
        let b = telefono.sincronizar(destino, ahora);
        assert_eq!(resumen(&a), (0, 0), "laptop, repetición {i}");
        assert_eq!(resumen(&b), (0, 0), "teléfono, repetición {i}");
        assert!(a.rechazados.is_empty() && b.rechazados.is_empty());
    }
    assert_eq!(destino.publicado(), publicado, "no debe haber lotes nuevos");
    assert_eq!(laptop.ids(), ids);
    assert_eq!(telefono.ids(), ids);
    assert_eq!(laptop.saldos(), saldos);
    imprimir(
        "Ambos sincronizan otra vez",
        &laptop,
        &telefono,
        "0 enviados, 0 recibidos",
    );

    // Cada dispositivo escribió solo en su propia carpeta del destino.
    let propios = [laptop.id_dispositivo(), telefono.id_dispositivo()];
    for ruta in destino.publicado() {
        assert!(
            propios.iter().any(|d| ruta.starts_with(&format!("{d}/"))),
            "archivo fuera de las carpetas de los dispositivos: {ruta}"
        );
    }

    // Una instalación nueva que se une al mismo destino con la misma frase queda igual.
    let mut tableta = Instalacion::abrir(raiz, "tableta");
    let union = tableta.sincronizar(destino, "2026-11-26T12:00:00Z");
    assert!(union.rechazados.is_empty());
    assert_eq!(tableta.ids(), laptop.ids());
    assert_eq!(tableta.saldos(), laptop.saldos());
    assert_eq!(tableta.saldo_de("Efectivo"), "166.67");

    // La copia exportada al inicio sigue siendo importable y nunca borra lo que ya hay.
    let antes = laptop.cantidad_de_eventos();
    let informe = laptop
        .casos
        .importar_copia(&copia, Some(FRASE), &cifrador())
        .unwrap();
    assert_eq!(
        informe.nuevos.len(),
        0,
        "importar una copia antigua no agrega nada"
    );
    assert_eq!(
        laptop.cantidad_de_eventos(),
        antes,
        "ni elimina eventos locales"
    );
    assert_eq!(laptop.saldo_de("Efectivo"), "166.67");

    // Privacidad: lo que quedó en el destino es solo texto cifrado.
    comprobar_que_el_destino_solo_tiene_archivos_cifrados(destino);
}

// ---------- Las cuatro combinaciones: destino × orden ----------

#[test]
fn carpeta_compartida_sincroniza_primero_la_laptop() {
    let e = Escenario::nuevo("carpeta_laptop");
    escenario_de_beto(&Destino::carpeta(&e.base), &e.base, false);
}

#[test]
fn carpeta_compartida_sincroniza_primero_el_telefono() {
    let e = Escenario::nuevo("carpeta_telefono");
    escenario_de_beto(&Destino::carpeta(&e.base), &e.base, true);
}

#[test]
fn repositorio_git_sincroniza_primero_la_laptop() {
    let e = Escenario::nuevo("git_laptop");
    escenario_de_beto(&Destino::git(&e.base), &e.base, false);
}

#[test]
fn repositorio_git_sincroniza_primero_el_telefono() {
    let e = Escenario::nuevo("git_telefono");
    escenario_de_beto(&Destino::git(&e.base), &e.base, true);
}

// ---------- Casos que completan el flujo ----------

/// Las dos instalaciones tienen carpetas de datos distintas y libros distintos en disco:
/// lo que una escribe no aparece en la otra hasta que sincronizan.
#[test]
fn cada_instalacion_tiene_su_propia_carpeta_de_datos() {
    let e = Escenario::nuevo("carpetas_distintas");
    let mut laptop = Instalacion::abrir(&e.base, "laptop");
    let telefono = Instalacion::abrir(&e.base, "telefono");
    laptop.crear_cuenta("Efectivo", "USD", "10.00");

    assert!(e.base.join("datos/laptop/cuentas_claras.db").is_file());
    assert!(e.base.join("datos/telefono/cuentas_claras.db").is_file());
    assert_ne!(laptop.id_dispositivo(), telefono.id_dispositivo());
    assert!(
        telefono.saldos().is_empty(),
        "el teléfono no ve la cuenta de la laptop"
    );
}

/// Un dispositivo con otra frase no puede leer lo que otro dejó en el destino y su libro
/// no cambia (la sincronización entera falla antes de tocarlo).
#[test]
fn con_otra_frase_no_se_importa_nada_del_destino() {
    let e = Escenario::nuevo("otra_frase");
    let destino = Destino::carpeta(&e.base);
    let mut laptop = Instalacion::abrir(&e.base, "laptop");
    laptop.crear_cuenta("Efectivo", "USD", "10.00");
    laptop.sincronizar(&destino, "2026-11-26T09:00:00Z");

    let mut intruso = Instalacion::abrir(&e.base, "intruso");
    let antes = intruso.ids();
    let mut transporte = destino.transporte("intruso", &intruso.id_dispositivo());
    let resultado = sincronizar(
        &mut intruso.casos,
        transporte.as_mut(),
        &cifrador(),
        "una frase que no es la de beto",
        "2026-11-26T09:05:00Z",
        &intruso.estado,
    );
    assert!(resultado.is_err());
    assert_eq!(intruso.ids(), antes);
    assert!(intruso.saldos().is_empty());
}

/// Sincroniza `instalacion` con una carpeta usando `nombre_carpeta` como su carpeta en el
/// destino. Sirve para fijar el orden en que el destino lista las carpetas de los
/// dispositivos (siempre por nombre), que con los ids reales sería aleatorio.
fn sincronizar_en_carpeta(
    instalacion: &mut Instalacion,
    carpeta: &Path,
    nombre_carpeta: &str,
    ahora: &str,
) -> InformeSincronizacion {
    let mut transporte = TransporteCarpeta::nuevo(carpeta, nombre_carpeta).unwrap();
    let (informe, estado) = sincronizar(
        &mut instalacion.casos,
        &mut transporte,
        &cifrador(),
        FRASE,
        ahora,
        &instalacion.estado,
    )
    .unwrap_or_else(|e| panic!("{}: la sincronización falló: {e}", instalacion.nombre));
    instalacion.estado = estado;
    informe
}

/// Un dispositivo nuevo recibe bien el libro aunque la carpeta del dispositivo que solo
/// agregó movimientos se liste antes que la del que creó la cuenta: el orden de las
/// carpetas no es el orden en que ocurrieron los eventos. Antes de arreglarlo, esto
/// fallaba la mitad de las veces (los ids de dispositivo son aleatorios).
#[test]
fn un_dispositivo_nuevo_recibe_bien_sin_importar_el_orden_de_las_carpetas() {
    let e = Escenario::nuevo("orden_de_carpetas");
    let carpeta = e.base.join("carpeta-compartida");
    fs::create_dir_all(&carpeta).unwrap();

    // La laptop crea la cuenta y la publica en la carpeta «b-laptop».
    let mut laptop = Instalacion::abrir(&e.base, "laptop");
    let efectivo = laptop.crear_cuenta("Efectivo", "USD", "100.00");
    sincronizar_en_carpeta(&mut laptop, &carpeta, "b-laptop", "2026-11-26T09:00:00Z");

    // El teléfono la recibe, registra un gasto y lo publica en «a-telefono»: su lote solo
    // tiene un movimiento y depende de la cuenta que está en el lote de la laptop.
    let mut telefono = Instalacion::abrir(&e.base, "telefono");
    sincronizar_en_carpeta(
        &mut telefono,
        &carpeta,
        "a-telefono",
        "2026-11-26T09:01:00Z",
    );
    telefono.movimiento(&efectivo, "egreso", "10.00", "gasto:alimentacion", "Gasto");
    let enviado = sincronizar_en_carpeta(
        &mut telefono,
        &carpeta,
        "a-telefono",
        "2026-11-26T09:02:00Z",
    );
    assert_eq!((enviado.enviados, enviado.recibidos), (1, 0));

    // La tableta lista primero «a-telefono» y después «b-laptop».
    let mut tableta = Instalacion::abrir(&e.base, "tableta");
    let union = sincronizar_en_carpeta(&mut tableta, &carpeta, "c-tableta", "2026-11-26T09:03:00Z");
    assert!(union.rechazados.is_empty());
    let esperados: BTreeSet<String> = laptop.ids().union(&telefono.ids()).cloned().collect();
    assert_eq!(tableta.ids(), esperados);
    assert_eq!(tableta.saldo_de("Efectivo"), "90.00");

    // Y la laptop, al sincronizar, queda igual que la tableta.
    sincronizar_en_carpeta(&mut laptop, &carpeta, "b-laptop", "2026-11-26T09:04:00Z");
    assert_eq!(laptop.ids(), tableta.ids());
    assert_eq!(laptop.saldos(), tableta.saldos());
}
