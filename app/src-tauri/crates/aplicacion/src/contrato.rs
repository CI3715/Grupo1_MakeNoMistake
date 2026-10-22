//! Batería de pruebas de contrato para el puerto [`RepositorioEventos`].
//!
//! Verifica que cualquier implementación del repositorio (en memoria o persistente)
//! respete las invariantes de orden de inserción, inmutabilidad e idempotencia, y
//! que mantenga las proyecciones `cuentas`, `categorias` y `movimientos` coherentes
//! con el libro de eventos, incluido el saldo inicial separado del actual (D-11).

use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, EventoRegistrado, MovimientoProyectado,
    RepositorioEventos, ResultadoGuardado,
};

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

/// Construye una carga JSON de `CuentaCreada` con un saldo inicial arbitrario.
pub fn datos_cuenta_con_saldo(id: &str, nombre: &str, saldo_inicial: &str) -> String {
    format!(
        r#"{{"id":"{id}","nombre":"{nombre}","tipo":"efectivo","moneda":"USD","saldo_inicial":"{saldo_inicial}"}}"#
    )
}

/// Construye una carga JSON válida de un evento `CategoriaCreada`.
pub fn datos_categoria(id: &str, nombre: &str, tipo: &str, es_fabrica: bool) -> String {
    format!(r#"{{"id":"{id}","nombre":"{nombre}","tipo":"{tipo}","es_fabrica":{es_fabrica}}}"#)
}

/// Construye una carga JSON válida de un evento `MovimientoRegistrado`.
pub fn datos_movimiento(
    id: &str,
    cuenta_id: &str,
    tipo: &str,
    monto: &str,
    fecha: &str,
    categoria_id: &str,
) -> String {
    format!(
        r#"{{"id":"{id}","cuenta_id":"{cuenta_id}","tipo":"{tipo}","monto":"{monto}","fecha":"{fecha}","categoria_id":"{categoria_id}"}}"#
    )
}

/// Construye una carga JSON de `MovimientoCorregido` que reemplaza a `original`.
pub fn datos_corregido(
    id: &str,
    original: &str,
    cuenta_id: &str,
    tipo: &str,
    monto: &str,
    fecha: &str,
    categoria_id: &str,
) -> String {
    format!(
        r#"{{"id":"{id}","movimiento_original_id":"{original}","cuenta_id":"{cuenta_id}","tipo":"{tipo}","monto":"{monto}","fecha":"{fecha}","categoria_id":"{categoria_id}"}}"#
    )
}

/// Construye la fila esperada en la proyección `cuentas` para una cuenta de prueba.
fn cuenta_proyectada(id: &str, nombre: &str) -> CuentaProyectada {
    CuentaProyectada {
        id: id.to_string(),
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: "VES".to_string(),
        saldo_inicial: "0.00".to_string(),
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
    assert!(
        repo.listar_categorias()
            .expect("listar categorías de un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo no debe tener categorías proyectadas"
    );
    assert!(
        repo.listar_movimientos()
            .expect("listar movimientos de un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo no debe tener movimientos proyectados"
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

    // 6. Los eventos de tipos no reconocidos no alteran ninguna proyección.
    let mut repo = fabrica();
    repo.guardar(&evento_de_prueba("id-1", &datos_cuenta("c1", "Cartera")))
        .expect("guardar cuenta");
    repo.guardar(&evento_de_tipo("id-2", "EventoDesconocido", "{}"))
        .expect("guardar evento desconocido");
    assert_eq!(repo.listar().expect("listar").len(), 2);
    assert_eq!(repo.listar_cuentas().expect("listar cuentas").len(), 1);
    assert!(
        repo.listar_categorias()
            .expect("listar categorías")
            .is_empty()
    );
    assert!(
        repo.listar_movimientos()
            .expect("listar movimientos")
            .is_empty()
    );

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
    repo.guardar(&evento_de_tipo("e4", "EventoDesconocido", "{}"))
        .expect("guardar evento desconocido");
    let antes = repo.listar_cuentas().expect("listar cuentas");
    repo.reconstruir().expect("reconstruir");
    assert_eq!(repo.listar_cuentas().expect("listar cuentas"), antes);

    // 8. Categorías y movimientos se proyectan y el saldo actual se calcula desde
    //    el saldo inicial, que permanece intacto (D-11).
    let mut repo = fabrica();
    repo.guardar(&evento_de_prueba(
        "e-cuenta",
        &datos_cuenta_con_saldo("c1", "Cartera", "100.00"),
    ))
    .expect("guardar cuenta");
    repo.guardar(&evento_de_tipo(
        "e-cat",
        "CategoriaCreada",
        &datos_categoria("cat-1", "Salario", "ingreso", false),
    ))
    .expect("guardar categoría");
    repo.guardar(&evento_de_tipo(
        "e-m1",
        "MovimientoRegistrado",
        &datos_movimiento("m1", "c1", "ingreso", "30.00", "2026-10-14", "cat-1"),
    ))
    .expect("guardar movimiento");
    repo.guardar(&evento_de_tipo(
        "e-m2",
        "MovimientoRegistrado",
        &datos_movimiento("m2", "c1", "egreso", "10.00", "2026-10-15", "cat-1"),
    ))
    .expect("guardar movimiento");

    assert_eq!(
        repo.listar_categorias().expect("listar categorías"),
        vec![CategoriaProyectada {
            id: "cat-1".to_string(),
            nombre: "Salario".to_string(),
            tipo: "ingreso".to_string(),
            es_fabrica: false,
            estado: "activa".to_string(),
        }]
    );
    assert_eq!(
        repo.listar_movimientos().expect("listar movimientos"),
        vec![
            MovimientoProyectado {
                id: "m1".to_string(),
                cuenta_id: "c1".to_string(),
                tipo: "ingreso".to_string(),
                monto: "30.00".to_string(),
                fecha: "2026-10-14".to_string(),
                categoria_id: "cat-1".to_string(),
                nota: None,
                estado: "vigente".to_string(),
                reemplaza_a: None,
                corregido_por_evento: None,
            },
            MovimientoProyectado {
                id: "m2".to_string(),
                cuenta_id: "c1".to_string(),
                tipo: "egreso".to_string(),
                monto: "10.00".to_string(),
                fecha: "2026-10-15".to_string(),
                categoria_id: "cat-1".to_string(),
                nota: None,
                estado: "vigente".to_string(),
                reemplaza_a: None,
                corregido_por_evento: None,
            },
        ]
    );
    let cuenta = &repo.listar_cuentas().expect("listar cuentas")[0];
    assert_eq!(cuenta.saldo_inicial, "100.00", "el saldo inicial no cambia");
    assert_eq!(cuenta.saldo_actual, "120.00", "100 + 30 - 10");

    // 9. Corregir un movimiento marca el original y crea una fila nueva; el saldo
    //    solo cuenta la corrección.
    repo.guardar(&evento_de_tipo(
        "e-m3",
        "MovimientoCorregido",
        &datos_corregido("m3", "m1", "c1", "ingreso", "5.00", "2026-10-14", "cat-1"),
    ))
    .expect("guardar corrección");

    let movimientos = repo.listar_movimientos().expect("listar movimientos");
    assert_eq!(movimientos.len(), 3);
    assert_eq!(movimientos[0].id, "m1");
    assert_eq!(movimientos[0].estado, "corregido");
    assert_eq!(movimientos[0].corregido_por_evento.as_deref(), Some("e-m3"));
    assert_eq!(movimientos[2].id, "m3");
    assert_eq!(movimientos[2].estado, "vigente");
    assert_eq!(movimientos[2].reemplaza_a.as_deref(), Some("m1"));
    let cuenta = &repo.listar_cuentas().expect("listar cuentas")[0];
    assert_eq!(cuenta.saldo_inicial, "100.00");
    assert_eq!(cuenta.saldo_actual, "95.00", "100 + 5 - 10 (m1 corregido)");

    // 10. `reconstruir` rearma también categorías y movimientos.
    let cuentas_antes = repo.listar_cuentas().expect("listar cuentas");
    let categorias_antes = repo.listar_categorias().expect("listar categorías");
    let movimientos_antes = repo.listar_movimientos().expect("listar movimientos");
    repo.reconstruir().expect("reconstruir");
    assert_eq!(
        repo.listar_cuentas().expect("listar cuentas"),
        cuentas_antes
    );
    assert_eq!(
        repo.listar_categorias().expect("listar categorías"),
        categorias_antes
    );
    assert_eq!(
        repo.listar_movimientos().expect("listar movimientos"),
        movimientos_antes
    );
}
