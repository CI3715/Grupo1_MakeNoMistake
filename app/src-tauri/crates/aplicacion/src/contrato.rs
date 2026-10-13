//! Batería de pruebas de contrato para el puerto [`RepositorioEventos`].
//!
//! Verifica que cualquier implementación del repositorio (en memoria o persistente)
//! respete las invariantes de orden de inserción, inmutabilidad e idempotencia.

use crate::puertos::{EventoRegistrado, RepositorioEventos, ResultadoGuardado};

/// Construye un [`EventoRegistrado`] de prueba con el identificador y carga JSON indicados.
pub fn evento_de_prueba(id: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-10-14T12:00:00Z".to_string(),
        dispositivo_origen: "dispositivo-de-prueba".to_string(),
        datos: datos.to_string(),
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
    // 1. Un repositorio recién inicializado no contiene eventos.
    let repo = fabrica();
    assert!(
        repo.listar()
            .expect("listar un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo debe estar vacío"
    );

    // 2. Guardar un evento nuevo y listarlo devuelve exactamente el mismo registro.
    let mut repo = fabrica();
    let evento = evento_de_prueba("id-1", r#"{"cuenta_id":"c1","nombre":"Ñandú"}"#);
    assert_eq!(
        repo.guardar(&evento).expect("guardar un evento nuevo"),
        ResultadoGuardado::Guardado
    );
    assert_eq!(repo.listar().expect("listar"), vec![evento.clone()]);

    // 3. El listado conserva el orden de inserción, independientemente del orden alfabético del id.
    let mut repo = fabrica();
    let b = evento_de_prueba("b", "{}");
    let a = evento_de_prueba("a", "{}");
    let c = evento_de_prueba("c", "{}");
    for e in [&b, &a, &c] {
        repo.guardar(e).expect("guardar");
    }
    assert_eq!(repo.listar().expect("listar"), vec![b, a, c]);

    // 4. Guardar dos veces el mismo identificador es idempotente y no duplica registros.
    let mut repo = fabrica();
    let evento = evento_de_prueba("id-1", "{}");
    assert_eq!(
        repo.guardar(&evento).expect("primera vez"),
        ResultadoGuardado::Guardado
    );
    assert_eq!(
        repo.guardar(&evento).expect("segunda vez"),
        ResultadoGuardado::YaExistia
    );
    assert_eq!(repo.listar().expect("listar").len(), 1);

    // 5. Los eventos son inmutables: si llega un id repetido con distinto contenido, prevalece el original.
    let mut repo = fabrica();
    let original = evento_de_prueba("id-1", r#"{"version":1}"#);
    let distinto = evento_de_prueba("id-1", r#"{"version":2}"#);
    repo.guardar(&original).expect("guardar original");
    assert_eq!(
        repo.guardar(&distinto).expect("guardar distinto"),
        ResultadoGuardado::YaExistia
    );
    assert_eq!(repo.listar().expect("listar"), vec![original]);
}
