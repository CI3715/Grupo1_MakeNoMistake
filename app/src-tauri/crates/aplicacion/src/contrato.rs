//! Batería de pruebas de contrato para el puerto [`RepositorioEventos`].
//!
//! Verifica que cualquier implementación del repositorio (en memoria o persistente)
//! respete las invariantes de orden de inserción, inmutabilidad e idempotencia, y
//! que mantenga la proyección `cuentas` coherente con el libro de eventos.

use crate::puertos::{CuentaProyectada, EventoRegistrado, RepositorioEventos, ResultadoGuardado};

/// Construye un [`EventoRegistrado`] de tipo `CuentaCreada` con el identificador y
/// carga JSON indicados.
pub fn evento_de_prueba(id: &str, datos: &str) -> EventoRegistrado {
    evento_de_tipo(id, "CuentaCreada", datos)
}

/// Construye un [`EventoRegistrado`] con el tipo y la carga JSON indicados.
pub fn evento_de_tipo(id: &str, tipo: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: tipo.to_string(),
        momento_registro: "2026-10-14T12:00:00Z".to_string(),
        dispositivo_origen: "dispositivo-de-prueba".to_string(),
        datos: datos.to_string(),
    }
}

/// Construye una carga JSON válida de un evento `CuentaCreada`.
pub fn datos_cuenta(id: &str, nombre: &str) -> String {
    format!(
        r#"{{"id":"{id}","nombre":"{nombre}","tipo":"efectivo","moneda":"VES","saldo_inicial":"0.00"}}"#
    )
}

/// Construye la fila esperada en la proyección `cuentas` para una cuenta de prueba.
fn cuenta_proyectada(id: &str, nombre: &str) -> CuentaProyectada {
    CuentaProyectada {
        id: id.to_string(),
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: "VES".to_string(),
        saldo_actual: "0.00".to_string(),
        estado: "activa".to_string(),
    }
}

/// Ejecuta las comprobaciones de contrato sobre un repositorio producido por `fabrica`.
///
/// La función `fabrica` debe devolver una instancia nueva y vacía en cada invocación.
pub fn bateria_contrato_repositorio<R, F>(fabrica: F)
where
    R: RepositorioEventos,
    F: Fn() -> R,
{
    // 1. Un repositorio recién inicializado no contiene eventos ni proyecciones.
    let repo = fabrica();
    assert!(
        repo.listar()
            .expect("listar un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo debe estar vacío"
    );
    assert!(
        repo.listar_cuentas()
            .expect("listar cuentas de un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo no debe tener cuentas proyectadas"
    );

    // 2. Guardar un evento nuevo lo lista igual y lo proyecta en `cuentas`.
    let mut repo = fabrica();
    let evento = evento_de_prueba("id-1", &datos_cuenta("c1", "Ñandú"));
    assert_eq!(
        repo.guardar(&evento).expect("guardar un evento nuevo"),
        ResultadoGuardado::Guardado
    );
    assert_eq!(repo.listar().expect("listar"), vec![evento.clone()]);
    assert_eq!(
        repo.listar_cuentas().expect("listar cuentas"),
        vec![cuenta_proyectada("c1", "Ñandú")]
    );

    // 3. Tanto los eventos como las cuentas conservan el orden de inserción,
    //    independientemente del orden alfabético del id.
    let mut repo = fabrica();
    let b = evento_de_prueba("b", &datos_cuenta("cb", "B"));
    let a = evento_de_prueba("a", &datos_cuenta("ca", "A"));
    let c = evento_de_prueba("c", &datos_cuenta("cc", "C"));
    for e in [&b, &a, &c] {
        repo.guardar(e).expect("guardar");
    }
    assert_eq!(repo.listar().expect("listar"), vec![b, a, c]);
    assert_eq!(
        repo.listar_cuentas().expect("listar cuentas"),
        vec![
            cuenta_proyectada("cb", "B"),
            cuenta_proyectada("ca", "A"),
            cuenta_proyectada("cc", "C"),
        ]
    );

    // 4. Guardar dos veces el mismo identificador es idempotente y no duplica registros.
    let mut repo = fabrica();
    let evento = evento_de_prueba("id-1", &datos_cuenta("c1", "Cartera"));
    assert_eq!(
        repo.guardar(&evento).expect("primera vez"),
        ResultadoGuardado::Guardado
    );
    assert_eq!(
        repo.guardar(&evento).expect("segunda vez"),
        ResultadoGuardado::YaExistia
    );
    assert_eq!(repo.listar().expect("listar").len(), 1);
    assert_eq!(repo.listar_cuentas().expect("listar cuentas").len(), 1);

    // 5. Los eventos son inmutables: si llega un id repetido con distinto contenido,
    //    prevalece el original, incluso en la proyección.
    let mut repo = fabrica();
    let original = evento_de_prueba("id-1", &datos_cuenta("c1", "Original"));
    let distinto = evento_de_prueba("id-1", &datos_cuenta("c1", "Distinto"));
    repo.guardar(&original).expect("guardar original");
    assert_eq!(
        repo.guardar(&distinto).expect("guardar distinto"),
        ResultadoGuardado::YaExistia
    );
    assert_eq!(repo.listar().expect("listar"), vec![original]);
    assert_eq!(
        repo.listar_cuentas().expect("listar cuentas"),
        vec![cuenta_proyectada("c1", "Original")]
    );

    // 6. Los eventos de tipos no reconocidos no alteran la proyección de cuentas.
    let mut repo = fabrica();
    repo.guardar(&evento_de_prueba("id-1", &datos_cuenta("c1", "Cartera")))
        .expect("guardar cuenta");
    repo.guardar(&evento_de_tipo("id-2", "MovimientoRegistrado", "{}"))
        .expect("guardar evento desconocido");
    assert_eq!(repo.listar().expect("listar").len(), 2);
    assert_eq!(repo.listar_cuentas().expect("listar cuentas").len(), 1);

    // 7. `reconstruir` recalcula las proyecciones desde los eventos y deja el mismo
    //    resultado que la proyección incremental.
    let mut repo = fabrica();
    for (id, cid, nombre) in [
        ("e1", "c1", "Uno"),
        ("e2", "c2", "Dos"),
        ("e3", "c3", "Tres"),
    ] {
        repo.guardar(&evento_de_prueba(id, &datos_cuenta(cid, nombre)))
            .expect("guardar");
    }
    repo.guardar(&evento_de_tipo("e4", "MovimientoRegistrado", "{}"))
        .expect("guardar evento desconocido");
    let antes = repo.listar_cuentas().expect("listar cuentas");
    repo.reconstruir().expect("reconstruir");
    assert_eq!(repo.listar_cuentas().expect("listar cuentas"), antes);
}
