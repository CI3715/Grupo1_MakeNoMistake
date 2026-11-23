//! Interrupción a mitad de lote con SQLite real (Entrega 7: INV-13, INV-14, INV-15, ESC-05).
//!
//! Criterio de aceptación: si la aplicación se cierra a mitad de un lote, la siguiente
//! sincronización pide desde el último cursor guardado y termina el trabajo sin duplicar.
//!
//! `guardar_lote` abre su propia transacción, así que no se puede cortar desde fuera. El
//! fallo se provoca con un trigger TEMP de SQLite que aborta el insert de un movimiento
//! concreto: ocurre dentro de la transacción real, después de haber aplicado los
//! anteriores. Un trigger TEMP vive solo en esa conexión: al cerrar y reabrir el archivo
//! desaparece, igual que el problema que provocó el cierre de la aplicación.
//!
//! El archivo se llama `pruebas.rs` (como `sesion/pruebas.rs`) y no habla con el servidor
//! por su cuenta: la red es un cliente falso.

use super::*;
use crate::sesion::{Respuesta, SesionGuardada, SinConexion};
use crate::social_cliente::Peticion;
use aplicacion::{DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo};
use infraestructura::db::BaseDatos;
use infraestructura::repositorios::RepositorioEventosSqlite;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const AHORA: &str = "2026-11-20T12:00:00Z";
/// Eventos del lote del barrido. El evento `i` (desde 0) está en la posición `i + 1` y
/// es un gasto de `(i + 1) * 10,00`: con saldo inicial de 200,00 todos suman 150,00.
const N: usize = 5;

/// `ErroresDto` no implementa `Debug`: en las pruebas se abre con un mensaje fijo.
fn ok<T>(r: Result<T, ErroresDto>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("se esperaba Ok y llegó un error de la aplicación"),
    }
}

// ---------- Archivo temporal y libro ----------

/// Ruta del libro de la prueba; al terminar se borra con sus archivos auxiliares de WAL.
struct Archivo(PathBuf);

fn limpiar(ruta: &Path) {
    for sufijo in ["", "-wal", "-shm"] {
        let mut nombre = ruta.as_os_str().to_owned();
        nombre.push(sufijo);
        let _ = std::fs::remove_file(PathBuf::from(nombre));
    }
}

impl Archivo {
    fn nuevo(nombre: &str) -> Self {
        let ruta =
            std::env::temp_dir().join(format!("cc_interrup_{}_{nombre}.db", std::process::id()));
        limpiar(&ruta);
        Archivo(ruta)
    }
}

impl Drop for Archivo {
    fn drop(&mut self) {
        limpiar(&self.0);
    }
}

/// El libro abierto, cableado como en `lib.rs`. Al soltarlo se cierra la conexión.
struct Libro {
    casos: Mutex<CasosDeUso>,
    sync: SincronizacionSqlite,
    base: BaseDatos,
}

fn abrir(ruta: &Path) -> Libro {
    let base = BaseDatos::persistente(ruta).unwrap();
    let conn = base.conexion();
    let casos = CasosDeUso::nuevo(
        Box::new(RepositorioEventosSqlite::with_conn(Arc::clone(&conn))),
        Box::new(RelojFijo::nuevo(AHORA)),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );
    let sync = SincronizacionSqlite::nuevo(conn).unwrap();
    Libro {
        casos: Mutex::new(casos),
        sync,
        base,
    }
}

impl Libro {
    /// Crea «Efectivo USD» con 200,00 y la elige para el grupo `g1`.
    fn con_cuenta_para_g1(&self) -> String {
        let id = bloquear(&self.casos)
            .crear_cuenta(NuevaCuenta {
                nombre: "Efectivo USD".to_string(),
                tipo: "efectivo".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: "200.00".to_string(),
            })
            .unwrap()
            .id;
        self.sync.elegir_cuenta("g1", &id).unwrap();
        id
    }

    /// Aborta, solo en esta conexión, el insert del movimiento de ese evento del servidor.
    /// Los movimientos anteriores del mismo lote ya se habrán insertado en la transacción.
    fn provocar_fallo_en(&self, evento_id: &str) {
        let conn = self.base.conexion();
        conn.lock()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TEMP TRIGGER fallo_provocado BEFORE INSERT ON main.movimientos
                 WHEN NEW.id = '{evento_id}'
                 BEGIN SELECT RAISE(ABORT, 'fallo provocado'); END;"
            ))
            .unwrap();
    }

    /// (movimientos, saldo, cursor) de la cuenta.
    fn estado(&self, cuenta: &str) -> (usize, String, i64) {
        let d = bloquear(&self.casos).detalle_cuenta(cuenta).unwrap();
        (
            d.movimientos.len(),
            d.cuenta.saldo_actual,
            self.sync.cursor().unwrap(),
        )
    }
}

fn texto(centimos: i64) -> String {
    format!("{}.{:02}", centimos / 100, centimos % 100)
}

/// Saldo de la cuenta después de aplicar los primeros `j` eventos del barrido.
fn saldo_tras(j: usize) -> String {
    texto(20_000 - (1_000 * j * (j + 1) / 2) as i64)
}

/// (movimientos, saldo, cursor) esperados con `j` eventos aplicados y ese cursor.
fn quedan(j: usize, cursor: i64) -> (usize, String, i64) {
    (j, saldo_tras(j), cursor)
}

// ---------- Cliente falso ----------

/// Responde en orden y anota las rutas pedidas; sin más respuestas simula el servidor apagado.
struct Guion {
    respuestas: Mutex<VecDeque<(u16, String)>>,
    pedidas: Mutex<Vec<String>>,
}

fn guion(r: Vec<(u16, String)>) -> Guion {
    Guion {
        respuestas: Mutex::new(r.into()),
        pedidas: Mutex::new(Vec::new()),
    }
}

impl Guion {
    fn pedidas(&self) -> Vec<String> {
        self.pedidas.lock().unwrap().clone()
    }
}

impl ClienteSocial for Guion {
    async fn enviar_peticion(
        &self,
        peticion: &Peticion,
        _token: &str,
    ) -> Result<Respuesta, SinConexion> {
        self.pedidas.lock().unwrap().push(peticion.ruta.clone());
        match self.respuestas.lock().unwrap().pop_front() {
            Some((codigo, cuerpo)) => Ok(Respuesta { codigo, cuerpo }),
            None => Err(SinConexion),
        }
    }
}

struct ConSesion;

impl Almacen for ConSesion {
    fn guardar(&self, _sesion: &SesionGuardada) -> Result<(), String> {
        Ok(())
    }
    fn leer(&self) -> Option<SesionGuardada> {
        Some(SesionGuardada {
            token: "t".to_string(),
            vence_en: "2099-01-01T00:00:00Z".to_string(),
            nombre_visible: "Beto".to_string(),
            usuario_id: "u-2".to_string(),
            nombre_usuario: "beto".to_string(),
            invalidada: false,
        })
    }
    fn borrar(&self) {}
}

fn evento(id: &str, posicion: i64, grupo: &str, monto: &str) -> String {
    format!(
        r#"{{"id":"{id}","posicion":{posicion},"tipo":"gasto_de_grupo","grupo":{{"id":"{grupo}","nombre":"Viaje a Mérida"}},"gasto":{{"id":"x-{id}","concepto":"Hotel"}},"fecha":"2026-11-20","moneda":"USD","monto":"{monto}","incluye_redondeo":false}}"#
    )
}

fn pagina(eventos: &[String], cursor: i64, hay_mas: bool) -> (u16, String) {
    (
        200,
        format!(
            r#"{{"eventos":[{}],"cursor":{cursor},"hay_mas":{hay_mas}}}"#,
            eventos.join(",")
        ),
    )
}

fn id_del(i: usize) -> String {
    format!("e-{}", 100 + i)
}

/// El evento `i` del barrido: posición `i + 1`, monto `(i + 1) * 10,00`, grupo `g1`.
fn evento_del(i: usize) -> String {
    let monto = texto(1_000 * (i as i64 + 1));
    evento(&id_del(i), i as i64 + 1, "g1", &monto)
}

fn eventos(desde: usize, hasta: usize) -> Vec<String> {
    (desde..hasta).map(evento_del).collect()
}

async fn sincronizar_con(l: &Libro, c: &Guion) -> Result<EstadoSincronizacionDto, ErroresDto> {
    sincronizar_libro(c, &ConSesion, AHORA, &l.casos, &l.sync).await
}

// ---------- Pruebas ----------

/// Un lote de N eventos se corta después de aplicar `k` movimientos, para cada `k`.
/// Nada queda a medias, al reabrir se vuelve a pedir desde el cursor guardado y el
/// resultado es un movimiento por evento; con el cursor en 0 no cambia nada.
#[tokio::test]
async fn interrupcion_a_mitad_de_lote_se_reanuda_desde_el_cursor_guardado_sin_duplicar() {
    for k in 0..N {
        let archivo = Archivo::nuevo(&format!("barrido_{k}"));

        // Primera ejecución: el evento `k` revienta después de aplicar los `k` anteriores.
        let cuenta = {
            let l = abrir(&archivo.0);
            let cuenta = l.con_cuenta_para_g1();
            l.provocar_fallo_en(&id_del(k));
            let cliente = guion(vec![pagina(&eventos(0, N), N as i64, false)]);
            assert!(sincronizar_con(&l, &cliente).await.is_err(), "k = {k}");
            assert_eq!(
                l.estado(&cuenta),
                quedan(0, 0),
                "k = {k}: quedó algo a medias"
            );
            assert_eq!(l.sync.ultima_sincronizacion().unwrap(), None, "k = {k}");
            assert!(cliente.pedidas()[0].ends_with("desde=0"), "k = {k}");
            cuenta
        }; // aquí se cierra la aplicación: se sueltan todas las conexiones

        // Segunda ejecución: el archivo está como estaba y el fallo ya no existe.
        let l = abrir(&archivo.0);
        assert_eq!(l.estado(&cuenta), quedan(0, 0), "k = {k}: tras reabrir");
        let cliente = guion(vec![pagina(&eventos(0, N), N as i64, false)]);
        ok(sincronizar_con(&l, &cliente).await);
        assert!(cliente.pedidas()[0].ends_with("desde=0"), "k = {k}");
        assert_eq!(l.estado(&cuenta), quedan(N, N as i64), "k = {k}");
        assert_eq!(
            l.sync.ultima_sincronizacion().unwrap().as_deref(),
            Some(AHORA)
        );

        // Con el cursor reiniciado a cero los N eventos vuelven a bajar y se ignoran.
        l.sync.reiniciar_cursor().unwrap();
        let cliente = guion(vec![pagina(&eventos(0, N), N as i64, false)]);
        ok(sincronizar_con(&l, &cliente).await);
        assert_eq!(
            l.estado(&cuenta),
            quedan(N, N as i64),
            "k = {k}: cursor a 0"
        );
    }
}

/// La página 1 se confirma (su cursor se guarda) y la 2 se interrumpe a la mitad: lo de la
/// página 1 se conserva, lo de la 2 no deja nada, y el reintento pide desde el cursor de la 1.
#[tokio::test]
async fn interrupcion_en_la_segunda_pagina_conserva_la_primera_y_reanuda_desde_su_cursor() {
    let archivo = Archivo::nuevo("dos_paginas");
    let pagina_1 = || pagina(&eventos(0, 2), 2, true);
    let pagina_2 = || pagina(&eventos(2, N), N as i64, false);

    let cuenta = {
        let l = abrir(&archivo.0);
        let cuenta = l.con_cuenta_para_g1();
        // La página 2 aplica `e-102` y revienta en `e-103`.
        l.provocar_fallo_en(&id_del(3));
        let cliente = guion(vec![pagina_1(), pagina_2()]);
        assert!(sincronizar_con(&l, &cliente).await.is_err());
        let pedidas = cliente.pedidas();
        assert!(pedidas[0].ends_with("desde=0") && pedidas[1].ends_with("desde=2"));
        assert_eq!(l.estado(&cuenta), quedan(2, 2), "solo la página 1");
        assert_eq!(l.sync.ultima_sincronizacion().unwrap(), None);
        cuenta
    };

    let l = abrir(&archivo.0);
    assert_eq!(l.estado(&cuenta), quedan(2, 2), "tras reabrir");
    let cliente = guion(vec![pagina_2()]);
    ok(sincronizar_con(&l, &cliente).await);
    assert_eq!(cliente.pedidas().len(), 1);
    assert!(
        cliente.pedidas()[0].ends_with("desde=2"),
        "pide desde el cursor guardado"
    );
    assert_eq!(l.estado(&cuenta), quedan(N, N as i64));
    assert_eq!(
        l.sync.ultima_sincronizacion().unwrap().as_deref(),
        Some(AHORA)
    );

    // Volver a bajar todo (cursor en 0) no duplica nada.
    l.sync.reiniciar_cursor().unwrap();
    let cliente = guion(vec![pagina_1(), pagina_2()]);
    ok(sincronizar_con(&l, &cliente).await);
    assert_eq!(l.estado(&cuenta), quedan(N, N as i64));
}

/// Las partes que quedan pendientes (grupo sin cuenta elegida) se guardan en el mismo lote
/// que los movimientos: si el lote se interrumpe tampoco quedan a medias, y al reintentar
/// quedan una sola vez, también con el cursor en 0.
#[tokio::test]
async fn interrupcion_no_deja_partes_pendientes_a_medias() {
    let archivo = Archivo::nuevo("pendientes");
    let lote = || {
        pagina(
            &[
                evento("e-100", 1, "g1", "10.00"),
                evento("e-101", 2, "g2", "5.00"),
                evento("e-102", 3, "g1", "20.00"),
            ],
            3,
            false,
        )
    };

    let cuenta = {
        let l = abrir(&archivo.0);
        let cuenta = l.con_cuenta_para_g1();
        l.provocar_fallo_en("e-102");
        assert!(sincronizar_con(&l, &guion(vec![lote()])).await.is_err());
        assert_eq!(l.estado(&cuenta), (0, "200.00".to_string(), 0));
        assert!(l.sync.partes_pendientes().unwrap().is_empty());
        cuenta
    };

    let l = abrir(&archivo.0);
    ok(sincronizar_con(&l, &guion(vec![lote()])).await);
    assert_eq!(l.estado(&cuenta), (2, "170.00".to_string(), 3));
    assert_eq!(l.sync.partes_pendientes().unwrap().len(), 1);

    l.sync.reiniciar_cursor().unwrap();
    ok(sincronizar_con(&l, &guion(vec![lote()])).await);
    assert_eq!(l.estado(&cuenta), (2, "170.00".to_string(), 3));
    assert_eq!(l.sync.partes_pendientes().unwrap().len(), 1);
}
