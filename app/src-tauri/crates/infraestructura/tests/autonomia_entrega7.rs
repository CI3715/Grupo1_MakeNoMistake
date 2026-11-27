//! Autonomía y libros distintos de la Entrega 7 contra SQLite real
//! (RNF-01, INV-13, INV-15, INV-16, ADR-014, ADR-015).
//!
//! Este crate no puede hacer HTTP (lo prohíbe `tests/arquitectura.rs`), así que
//! todo lo de aquí funciona con el servidor apagado: las partes de grupo que ya
//! bajaron siguen en el libro, sus saldos y lo personal funcionan al cerrar y
//! reabrir, y volver a aplicar el mismo evento (por ejemplo, al sincronizar otra
//! vez desde el cursor 0) no duplica nada.
//!
//! Además, cada usuario del dispositivo tiene su propio archivo
//! `cuentas_claras_<id>.db` (ADR-014): la parte de ana cae solo en el libro de
//! ana y la de beto solo en el de beto. Qué archivo se abre según la sesión lo
//! prueban `src/libros.rs` y `src/cambio_libro.rs` del crate `app`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use aplicacion::{CasosDeUso, DatosParteDeGrupo, NuevaCuenta, NuevoMovimiento, ResultadoAplicar};
use infraestructura::db::BaseDatos;
use infraestructura::dispositivo::DispositivoOrigenSqlite;
use infraestructura::id_gen::GeneradorUuid;
use infraestructura::reloj::RelojSistema;
use infraestructura::repositorios::RepositorioEventosSqlite;

const GRUPO: &str = "600f76c8-e198-5d6f-82b8-ffb04cc3fc13";
const HOTEL: &str = "ff6edddc-3a5d-55a2-b39c-c50a58649d1f";
// Ids de evento de `docs/vectores-sincronizacion.json`: uno por participante.
const EVENTO_ANA: &str = "f16b54ef-4fe0-55cc-80f6-2da2cad68c97";
const EVENTO_BETO: &str = "6f798a51-6a23-524c-9671-10c6d799e5d1";
const ID_ANA: &str = "0b9d6c1e-52f7-4c3a-9d10-7a4e8f2b6c33";
const ID_BETO: &str = "5c2a9e47-1d3b-4f60-8a7e-3b9c0d1e2f48";

/// Carpeta de datos temporal; se borra con todo lo que tenga al terminar.
struct CarpetaTemporal(PathBuf);

impl CarpetaTemporal {
    fn nueva() -> Self {
        let ruta = std::env::temp_dir().join(format!("cc_e7_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&ruta).expect("crear la carpeta temporal");
        CarpetaTemporal(ruta)
    }

    /// El libro de un usuario, con el nombre que fija ADR-014.
    fn libro_de(&self, id_usuario: &str) -> PathBuf {
        self.0.join(format!("cuentas_claras_{id_usuario}.db"))
    }
}

impl Drop for CarpetaTemporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Abre el libro con el mismo cableado que `src/lib.rs` de la app.
fn abrir(ruta: &Path) -> CasosDeUso {
    let base = BaseDatos::persistente(ruta).expect("abrir el libro local");
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

fn cuenta_usd(casos: &mut CasosDeUso, saldo: &str) -> String {
    casos
        .crear_cuenta(NuevaCuenta {
            nombre: "Efectivo USD".into(),
            tipo: "efectivo".into(),
            moneda: "USD".into(),
            saldo_inicial: saldo.into(),
        })
        .expect("crear la cuenta")
        .id
}

/// La parte del hotel tal como la entrega `GET /cambios` a cada persona.
fn parte_del_hotel(
    evento: &str,
    cuenta_id: &str,
    monto: &str,
    redondeo: bool,
) -> DatosParteDeGrupo {
    DatosParteDeGrupo {
        evento_servidor_id: evento.into(),
        grupo_id: GRUPO.into(),
        gasto_id: HOTEL.into(),
        cuenta_id: cuenta_id.into(),
        monto: monto.into(),
        moneda: "USD".into(),
        fecha: "2026-03-14".into(),
        concepto: "Viaje a Mérida · Hotel".into(),
        incluye_redondeo: redondeo,
    }
}

fn saldo(casos: &CasosDeUso, cuenta_id: &str) -> String {
    casos
        .detalle_cuenta(cuenta_id)
        .expect("detalle de la cuenta")
        .cuenta
        .saldo_actual
}

fn movimientos_de_grupo(casos: &CasosDeUso, cuenta_id: &str) -> usize {
    casos
        .detalle_cuenta(cuenta_id)
        .expect("detalle de la cuenta")
        .movimientos
        .iter()
        .filter(|m| m.gasto_de_grupo)
        .count()
}

#[test]
fn sin_servidor_lo_ya_sincronizado_y_lo_personal_siguen_al_reabrir() {
    let carpeta = CarpetaTemporal::nueva();
    let ruta = carpeta.libro_de(ID_ANA);
    let cuenta = {
        let mut casos = abrir(&ruta);
        let cuenta = cuenta_usd(&mut casos, "200.00");
        // La parte del hotel ya había bajado antes de perder la conexión.
        let aplicada = casos
            .aplicar_parte_de_grupo(parte_del_hotel(EVENTO_ANA, &cuenta, "33.34", true))
            .expect("aplicar la parte del hotel");
        assert_eq!(aplicada, ResultadoAplicar::Aplicada);
        // Lo personal funciona igual, sin red.
        casos
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.clone(),
                tipo: "gasto".into(),
                monto: "16.66".into(),
                fecha: String::new(),
                categoria_id: "gasto:alimentacion".into(),
                nota: None,
            })
            .expect("registrar un gasto personal");
        cuenta
    };

    // Cerrar y reabrir: el mismo saldo. Reaplicar el evento no crea otro
    // movimiento ni cambia el saldo (INV-13, INV-15).
    for _ in 0..2 {
        let mut casos = abrir(&ruta);
        assert_eq!(saldo(&casos, &cuenta), "150.00");
        let otra_vez = casos
            .aplicar_parte_de_grupo(parte_del_hotel(EVENTO_ANA, &cuenta, "33.34", true))
            .expect("reaplicar la parte");
        assert_eq!(otra_vez, ResultadoAplicar::YaAplicada);
        assert_eq!(movimientos_de_grupo(&casos, &cuenta), 1);
        assert_eq!(saldo(&casos, &cuenta), "150.00");
    }
}

#[test]
fn cada_usuario_tiene_su_libro_con_solo_sus_partes() {
    let carpeta = CarpetaTemporal::nueva();
    let libro_ana = carpeta.libro_de(ID_ANA);
    let libro_beto = carpeta.libro_de(ID_BETO);

    let cuenta_ana = {
        let mut casos = abrir(&libro_ana);
        let cuenta = cuenta_usd(&mut casos, "200.00");
        casos
            .aplicar_parte_de_grupo(parte_del_hotel(EVENTO_ANA, &cuenta, "33.34", true))
            .expect("parte de ana");
        cuenta
    };

    // beto inicia sesión en el mismo dispositivo: otro archivo, vacío.
    let cuenta_beto = {
        let mut casos = abrir(&libro_beto);
        assert!(
            casos
                .listar_cuentas()
                .expect("cuentas de beto")
                .cuentas
                .is_empty(),
            "beto no ve las cuentas de ana"
        );
        let cuenta = cuenta_usd(&mut casos, "100.00");
        casos
            .aplicar_parte_de_grupo(parte_del_hotel(EVENTO_BETO, &cuenta, "33.33", false))
            .expect("parte de beto");
        cuenta
    };

    // Cada libro tiene solo la parte de su dueño, también al reabrirlos.
    for _ in 0..2 {
        let ana = abrir(&libro_ana);
        assert_eq!(
            ana.listar_cuentas().expect("cuentas de ana").cuentas.len(),
            1
        );
        assert_eq!(saldo(&ana, &cuenta_ana), "166.66");
        assert_eq!(movimientos_de_grupo(&ana, &cuenta_ana), 1);
        assert!(
            ana.detalle_cuenta(&cuenta_beto).is_err(),
            "la cuenta de beto no está"
        );

        let beto = abrir(&libro_beto);
        assert_eq!(
            beto.listar_cuentas()
                .expect("cuentas de beto")
                .cuentas
                .len(),
            1
        );
        assert_eq!(saldo(&beto, &cuenta_beto), "66.67");
        assert_eq!(movimientos_de_grupo(&beto, &cuenta_beto), 1);
        assert!(
            beto.detalle_cuenta(&cuenta_ana).is_err(),
            "la cuenta de ana no está"
        );
    }
}
