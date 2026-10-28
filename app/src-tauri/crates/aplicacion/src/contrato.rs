//! Batería de pruebas de contrato para el puerto [`RepositorioEventos`].
//!
//! Verifica que cualquier implementación del repositorio (en memoria o persistente)
//! respete las invariantes de orden de inserción, inmutabilidad e idempotencia, y
//! que mantenga las proyecciones `cuentas`, `categorias` y `movimientos` coherentes
//! con el libro de eventos, incluido el saldo inicial separado del actual (D-11).

use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, EventoRegistrado, MonedaProyectada,
    MovimientoProyectado, RepositorioEventos, ResultadoGuardado, TasaProyectada,
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

/// Construye una carga JSON válida de un evento `MonedaCreada`.
pub fn datos_moneda(codigo: &str, decimales: u32) -> String {
    format!(
        r#"{{"codigo":"{codigo}","nombre":"Moneda {codigo}","simbolo":"{codigo}","decimales":{decimales}}}"#
    )
}

/// Construye una carga JSON válida de un evento `TasaRegistrada`.
pub fn datos_tasa(
    id: &str,
    origen: &str,
    destino: &str,
    valor: &str,
    fecha: &str,
    fuente: &str,
) -> String {
    format!(
        r#"{{"id":"{id}","moneda_origen":"{origen}","moneda_destino":"{destino}","valor":"{valor}","fecha":"{fecha}","fuente":"{fuente}"}}"#
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
    assert!(
        repo.listar_monedas_propias()
            .expect("listar monedas de un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo no debe tener monedas propias"
    );
    assert!(
        repo.listar_tasas()
            .expect("listar tasas de un repositorio vacío")
            .is_empty(),
        "un repositorio nuevo no debe tener tasas"
    );
    assert_eq!(
        repo.moneda_base().expect("leer la moneda base"),
        None,
        "sin MonedaBaseCambiada no hay moneda base elegida"
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

    bateria_monedas_y_tasas(&fabrica);
}

/// Comprobaciones de contrato de la Entrega 4: monedas propias, historial de
/// tasas y moneda base.
fn bateria_monedas_y_tasas<R, F>(fabrica: &F)
where
    R: RepositorioEventos,
    F: Fn() -> R,
{
    // 11. Una moneda propia se proyecta y una cuenta puede usarla con sus decimales.
    let mut repo = fabrica();
    repo.guardar(&evento_de_tipo(
        "e-btc",
        "MonedaCreada",
        &datos_moneda("BTC", 8),
    ))
    .expect("crear BTC");
    repo.guardar(&evento_de_tipo(
        "e-jpy",
        "MonedaCreada",
        &datos_moneda("YEN", 0),
    ))
    .expect("crear YEN");
    assert_eq!(
        repo.listar_monedas_propias().expect("listar monedas"),
        vec![
            MonedaProyectada {
                codigo: "BTC".into(),
                nombre: "Moneda BTC".into(),
                simbolo: "BTC".into(),
                decimales: 8,
            },
            MonedaProyectada {
                codigo: "YEN".into(),
                nombre: "Moneda YEN".into(),
                simbolo: "YEN".into(),
                decimales: 0,
            },
        ]
    );

    repo.guardar(&evento_de_tipo(
        "e-c-btc",
        "CuentaCreada",
        r#"{"id":"c-btc","nombre":"Fría","tipo":"billetera_digital","moneda":"BTC","saldo_inicial":"0.00012345"}"#,
    ))
    .expect("cuenta en BTC");
    repo.guardar(&evento_de_tipo(
        "e-c-yen",
        "CuentaCreada",
        r#"{"id":"c-yen","nombre":"Yenes","tipo":"efectivo","moneda":"YEN","saldo_inicial":"1500"}"#,
    ))
    .expect("cuenta en YEN");
    repo.guardar(&evento_de_tipo(
        "e-cat",
        "CategoriaCreada",
        &datos_categoria("cat-i", "Regalos", "ingreso", false),
    ))
    .expect("categoría");
    repo.guardar(&evento_de_tipo(
        "e-m-btc",
        "MovimientoRegistrado",
        &datos_movimiento(
            "m-btc",
            "c-btc",
            "ingreso",
            "0.00000005",
            "2026-10-14",
            "cat-i",
        ),
    ))
    .expect("movimiento en BTC");
    repo.guardar(&evento_de_tipo(
        "e-m-yen",
        "MovimientoRegistrado",
        &datos_movimiento("m-yen", "c-yen", "ingreso", "250", "2026-10-14", "cat-i"),
    ))
    .expect("movimiento en YEN");

    let cuentas = repo.listar_cuentas().expect("listar cuentas");
    assert_eq!(cuentas[0].saldo_inicial, "0.00012345");
    assert_eq!(cuentas[0].saldo_actual, "0.00012350", "8 decimales exactos");
    assert_eq!(cuentas[1].saldo_inicial, "1500");
    assert_eq!(cuentas[1].saldo_actual, "1750", "moneda sin decimales");
    let movimientos = repo.listar_movimientos().expect("listar movimientos");
    assert_eq!(movimientos[0].monto, "0.00000005");
    assert_eq!(movimientos[1].monto, "250");

    // 12. Una moneda repetida o de fábrica se rechaza sin tocar la proyección.
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-btc-2",
            "MonedaCreada",
            &datos_moneda("BTC", 2)
        ))
        .is_err(),
        "BTC ya existe"
    );
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-usd",
            "MonedaCreada",
            &datos_moneda("USD", 2)
        ))
        .is_err(),
        "USD es de fábrica"
    );
    assert_eq!(repo.listar_monedas_propias().unwrap().len(), 2);

    // 13. Las tasas se acumulan en orden de registro (varias por par y fecha) y
    //     su valor se proyecta en texto canónico.
    let tasas = [
        ("t1", "USD", "VES", "887.00", "2026-10-20", "BCV"),
        ("t2", "USDT", "VES", "1000", "2026-10-21", "casa de cambio"),
        (
            "t3",
            "USD",
            "VES",
            "890.50",
            "2026-10-20",
            "estimación propia",
        ),
        (
            "t4",
            "BTC",
            "USD",
            "65000.12345678",
            "2026-10-21",
            "exchange",
        ),
    ];
    for (id, origen, destino, valor, fecha, fuente) in tasas {
        repo.guardar(&evento_de_tipo(
            &format!("e-{id}"),
            "TasaRegistrada",
            &datos_tasa(id, origen, destino, valor, fecha, fuente),
        ))
        .expect("registrar tasa");
    }
    let historial = repo.listar_tasas().expect("listar tasas");
    let resumen: Vec<(&str, &str, &str)> = historial
        .iter()
        .map(|t| (t.id.as_str(), t.valor.as_str(), t.estado.as_str()))
        .collect();
    assert_eq!(
        resumen,
        vec![
            ("t1", "887", "vigente"),
            ("t2", "1000", "vigente"),
            ("t3", "890.5", "vigente"),
            ("t4", "65000.12345678", "vigente"),
        ]
    );
    assert_eq!(
        historial[0],
        TasaProyectada {
            id: "t1".into(),
            moneda_origen: "USD".into(),
            moneda_destino: "VES".into(),
            valor: "887".into(),
            fecha: "2026-10-20".into(),
            fuente: "BCV".into(),
            estado: "vigente".into(),
        }
    );

    // 14. Una tasa con una moneda desconocida o un id repetido se rechaza.
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-t-xyz",
            "TasaRegistrada",
            &datos_tasa("t-xyz", "XYZ", "USD", "2", "2026-10-21", "BCV"),
        ))
        .is_err(),
        "XYZ no está en el catálogo"
    );
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-t1-bis",
            "TasaRegistrada",
            &datos_tasa("t1", "USD", "VES", "1", "2026-10-21", "BCV"),
        ))
        .is_err(),
        "el id t1 ya existe"
    );
    assert_eq!(repo.listar_tasas().unwrap().len(), 4);

    // 15. Eliminar una tasa es un evento nuevo: la tasa queda marcada, no se borra.
    repo.guardar(&evento_de_tipo(
        "e-del-t3",
        "TasaEliminada",
        r#"{"tasa_id":"t3"}"#,
    ))
    .expect("eliminar t3");
    let historial = repo.listar_tasas().unwrap();
    assert_eq!(
        historial.len(),
        4,
        "la tasa eliminada sigue en el historial"
    );
    assert_eq!(historial[2].estado, "eliminada");
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-del-t3-bis",
            "TasaEliminada",
            r#"{"tasa_id":"t3"}"#,
        ))
        .is_err(),
        "no se elimina dos veces"
    );
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-del-nada",
            "TasaEliminada",
            r#"{"tasa_id":"no-existe"}"#,
        ))
        .is_err(),
        "no se elimina una tasa inexistente"
    );

    // 16. La moneda base es la del último MonedaBaseCambiada y debe existir.
    repo.guardar(&evento_de_tipo(
        "e-base-ves",
        "MonedaBaseCambiada",
        r#"{"moneda":"VES"}"#,
    ))
    .expect("base VES");
    repo.guardar(&evento_de_tipo(
        "e-base-btc",
        "MonedaBaseCambiada",
        r#"{"moneda":"BTC"}"#,
    ))
    .expect("base BTC (moneda propia)");
    assert_eq!(repo.moneda_base().unwrap().as_deref(), Some("BTC"));
    assert!(
        repo.guardar(&evento_de_tipo(
            "e-base-xyz",
            "MonedaBaseCambiada",
            r#"{"moneda":"XYZ"}"#,
        ))
        .is_err()
    );
    assert_eq!(repo.moneda_base().unwrap().as_deref(), Some("BTC"));

    // 17. Cambiar la base no toca saldos ni movimientos, y `reconstruir` deja
    //     todo idéntico.
    let antes = (
        repo.listar_cuentas().unwrap(),
        repo.listar_movimientos().unwrap(),
        repo.listar_monedas_propias().unwrap(),
        repo.listar_tasas().unwrap(),
        repo.moneda_base().unwrap(),
    );
    assert_eq!(antes.0, cuentas, "la moneda base no cambia ningún saldo");
    repo.reconstruir().expect("reconstruir");
    assert_eq!(
        (
            repo.listar_cuentas().unwrap(),
            repo.listar_movimientos().unwrap(),
            repo.listar_monedas_propias().unwrap(),
            repo.listar_tasas().unwrap(),
            repo.moneda_base().unwrap(),
        ),
        antes
    );
}
