//! Prueba de autonomía de la Entrega 6 (RNF-01, INV-16).
//!
//! Con el servidor apagado, lo social no puede funcionar, pero todo lo personal
//! sí: la sesión falla con un mensaje claro y sin cerrar nada, y en la misma
//! ejecución se crean cuentas y movimientos en un SQLite real, se previsualiza
//! una división (que es local) y, al cerrar y reabrir la base, los saldos son
//! idénticos. Es la versión automática de la prueba manual del README
//! («apaga el servidor y la red, y vuelve a abrir la aplicación»).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aplicacion::{CasosDeUso, NuevaCuenta, NuevoMovimiento};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;

use crate::dto::{IniciarSesionDto, RegistroDto};
use crate::dto_social::{ParticipanteDto, PrevisualizarDivisionDto};
use crate::previsualizacion::previsualizar;
use crate::sesion::{self, Almacen, ClienteHttp, SesionGuardada};

const MENSAJE_SIN_CONEXION: &str =
    "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión.";

/// Archivo SQLite temporal que se borra con sus `-wal` y `-shm` al terminar.
struct BaseTemporal(PathBuf);

impl BaseTemporal {
    fn nueva() -> Self {
        let nombre = format!("cuentas_claras_autonomia_{}.db", uuid::Uuid::new_v4());
        BaseTemporal(std::env::temp_dir().join(nombre))
    }
}

impl Drop for BaseTemporal {
    fn drop(&mut self) {
        for sufijo in ["", "-wal", "-shm"] {
            let mut archivo = self.0.clone().into_os_string();
            archivo.push(sufijo);
            let _ = std::fs::remove_file(PathBuf::from(archivo));
        }
    }
}

/// Abre la base con el mismo cableado que `lib.rs`.
fn abrir_app(ruta: &Path) -> CasosDeUso {
    let base = BaseDatos::persistente(ruta).expect("abrir la base local");
    let conexion = base.conexion();
    let dispositivo =
        DispositivoOrigenSqlite::nuevo(Arc::clone(&conexion)).expect("identificar el dispositivo");
    CasosDeUso::nuevo(
        Box::new(RepositorioEventosSqlite::with_conn(conexion)),
        Box::new(RelojSistema::nuevo()),
        Box::new(GeneradorUuid::nuevo()),
        Box::new(dispositivo),
    )
}

#[derive(Default)]
struct AlmacenVacio(RefCell<Option<SesionGuardada>>);

impl Almacen for AlmacenVacio {
    fn guardar(&self, sesion: &SesionGuardada) -> Result<(), String> {
        *self.0.borrow_mut() = Some(sesion.clone());
        Ok(())
    }
    fn leer(&self) -> Option<SesionGuardada> {
        self.0.borrow().clone()
    }
    fn borrar(&self) {
        *self.0.borrow_mut() = None;
    }
}

fn mensaje(e: &crate::dto::ErroresDto) -> &str {
    e.errores.first().map(|x| x.mensaje.as_str()).unwrap_or("")
}

#[tokio::test]
async fn con_el_servidor_apagado_el_libro_personal_sigue_completo() {
    let cliente = ClienteHttp::hacia_servidor_apagado();
    let almacen = AlmacenVacio::default();

    // 1. Lo que necesita el servidor falla con un mensaje claro, sin pánico.
    let registro = sesion::registrar(
        &cliente,
        &RegistroDto {
            nombre_usuario: "ana".into(),
            nombre_visible: "Ana".into(),
            correo: "ana@ejemplo.com".into(),
            contrasena: "una-clave-larga".into(),
        },
    )
    .await;
    let Err(e) = registro else {
        panic!("sin servidor el registro no puede funcionar");
    };
    assert_eq!(mensaje(&e), MENSAJE_SIN_CONEXION);

    let inicio = sesion::iniciar(
        &cliente,
        &almacen,
        &IniciarSesionDto {
            correo: "ana@ejemplo.com".into(),
            contrasena: "una-clave-larga".into(),
        },
    )
    .await;
    let Err(e) = inicio else {
        panic!("sin servidor no se puede iniciar sesión");
    };
    assert_eq!(mensaje(&e), MENSAJE_SIN_CONEXION);
    assert!(almacen.leer().is_none(), "no se guarda una sesión a medias");

    // 2. Todo lo personal funciona igual, en un SQLite real.
    let temporal = BaseTemporal::nueva();
    let cuenta_id = {
        let mut casos = abrir_app(&temporal.0);
        let cuenta = casos
            .crear_cuenta(NuevaCuenta {
                nombre: "Billetera".into(),
                tipo: "efectivo".into(),
                moneda: "USD".into(),
                saldo_inicial: "1000.00".into(),
            })
            .expect("crear una cuenta sin servidor");
        for (tipo, monto, categoria) in [
            ("ingreso", "800.00", "ingreso:salario"),
            ("gasto", "500.00", "gasto:alimentacion"),
        ] {
            casos
                .registrar_movimiento(NuevoMovimiento {
                    cuenta_id: cuenta.id.clone(),
                    tipo: tipo.into(),
                    monto: monto.into(),
                    fecha: String::new(),
                    categoria_id: categoria.into(),
                    nota: None,
                })
                .expect("registrar un movimiento sin servidor");
        }
        cuenta.id
    };

    // 3. La vista previa de la división es local: da los números del enunciado.
    let previa = match previsualizar(&PrevisualizarDivisionDto {
        monto: "100.00".into(),
        regla: "partes_iguales".into(),
        participantes: ["ana", "beto", "carla"]
            .iter()
            .map(|u| ParticipanteDto {
                usuario: (*u).into(),
                valor: String::new(),
            })
            .collect(),
    }) {
        Ok(previa) => previa,
        Err(e) => panic!("previsualizar sin servidor: {}", mensaje(&e)),
    };
    let montos: Vec<&str> = previa.partes.iter().map(|p| p.monto.as_str()).collect();
    assert_eq!(montos, ["33.34", "33.33", "33.33"]);

    // 4. Cerrar y reabrir: los mismos datos, sin haber tocado la red.
    for _ in 0..2 {
        let casos = abrir_app(&temporal.0);
        let detalle = casos
            .detalle_cuenta(&cuenta_id)
            .expect("detalle al reabrir");
        assert_eq!(detalle.cuenta.saldo_actual, "1300.00");
        assert_eq!(detalle.movimientos.len(), 2);
    }
}
