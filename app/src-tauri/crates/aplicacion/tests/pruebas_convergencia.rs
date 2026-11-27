//! Verifica el ejemplo de convergencia de la especificación (sección 2.6).
//!
//! Dos dispositivos parten del mismo libro de 40 eventos con saldo 151,67,
//! registran movimientos sin conexión y se sincronizan en cualquiera de los dos
//! órdenes. Al final deben tener los mismos 44 eventos y el mismo saldo 166,67.

use aplicacion::{
    CasosDeUso, DispositivoFijo, EventoRegistrado, GeneradorSecuencial, RelojFijo,
    RepositorioEnMemoria,
};

const MOMENTO_CUENTA: &str = "2026-11-24T07:00:00Z";
const MOMENTO_BASE: &str = "2026-11-24T08:00:00Z";
const MOMENTO_LAPTOP: &str = "2026-11-24T10:00:00Z";
const MOMENTO_TELEFONO: &str = "2026-11-24T11:00:00Z";

fn evento(id: &str, tipo: &str, momento: &str, dispositivo: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: tipo.to_string(),
        momento_registro: momento.to_string(),
        dispositivo_origen: dispositivo.to_string(),
        datos: datos.to_string(),
    }
}

fn cuenta_inicial() -> EventoRegistrado {
    evento(
        "cuenta-1",
        "CuentaCreada",
        MOMENTO_CUENTA,
        "base",
        r#"{"id":"cuenta-1","nombre":"Efectivo USD","tipo":"efectivo","moneda":"USD","saldo_inicial":"200.00"}"#,
    )
}

fn movimiento(
    id: &str,
    tipo: &str,
    monto: &str,
    momento: &str,
    dispositivo: &str,
) -> EventoRegistrado {
    evento(
        id,
        "MovimientoRegistrado",
        momento,
        dispositivo,
        &format!(
            r#"{{"id":"{id}","cuenta_id":"cuenta-1","tipo":"{tipo}","monto":"{monto}","moneda":"USD","fecha":"2026-11-24","categoria_id":"gasto:otros","nota":""}}"#
        ),
    )
}

/// 40 eventos: la cuenta y 39 movimientos que dejan el saldo en 151,67.
fn libro_inicial() -> Vec<EventoRegistrado> {
    let mut eventos = vec![cuenta_inicial()];
    // 38 × 1,00 y 1 × 10,33 = 48,33 de egresos sobre 200,00.
    for n in 1..=38 {
        eventos.push(movimiento(
            &format!("base-{n:02}"),
            "gasto",
            "1.00",
            MOMENTO_BASE,
            "base",
        ));
    }
    eventos.push(movimiento(
        "base-39",
        "gasto",
        "10.33",
        MOMENTO_BASE,
        "base",
    ));
    assert_eq!(eventos.len(), 40);
    eventos
}

/// Dos movimientos sin conexión en la laptop: 20,00 y 5,00 de gastos.
fn movimientos_laptop() -> Vec<EventoRegistrado> {
    vec![
        movimiento("lap-1", "gasto", "20.00", MOMENTO_LAPTOP, "laptop"),
        movimiento("lap-2", "gasto", "5.00", MOMENTO_LAPTOP, "laptop"),
    ]
}

/// Dos movimientos sin conexión en el teléfono: 50,00 de ingreso y 10,00 de gasto.
fn movimientos_telefono() -> Vec<EventoRegistrado> {
    vec![
        movimiento("tel-1", "ingreso", "50.00", MOMENTO_TELEFONO, "teléfono"),
        movimiento("tel-2", "gasto", "10.00", MOMENTO_TELEFONO, "teléfono"),
    ]
}

fn libro(dispositivo: &str, eventos: Vec<EventoRegistrado>) -> CasosDeUso {
    let mut casos = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-24T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo(dispositivo)),
    );
    casos.fusionar_eventos(&eventos).expect("libro inicial");
    casos
}

fn saldo(casos: &CasosDeUso) -> String {
    casos
        .listar_cuentas()
        .expect("cuentas")
        .cuentas
        .into_iter()
        .find(|c| c.id == "cuenta-1")
        .expect("la cuenta existe")
        .saldo_actual
}

fn comprobar_convergencia(laptop_primero: bool) {
    let base = libro_inicial();
    let de_laptop = movimientos_laptop();
    let de_telefono = movimientos_telefono();

    let mut laptop = libro("laptop", [base.clone(), de_laptop.clone()].concat());
    let mut telefono = libro("teléfono", [base, de_telefono.clone()].concat());

    assert_eq!(laptop.eventos_del_libro().unwrap().len(), 42);
    assert_eq!(telefono.eventos_del_libro().unwrap().len(), 42);
    assert_eq!(saldo(&laptop), "126.67");
    assert_eq!(saldo(&telefono), "191.67");

    // Quien sincroniza primero envía sus dos eventos; el otro los recibe y
    // también envía los suyos. El resultado no depende de ese orden.
    if laptop_primero {
        let informe = laptop.fusionar_eventos(&de_telefono).unwrap();
        assert_eq!(informe.nuevos.len(), 2);
        let informe = telefono.fusionar_eventos(&de_laptop).unwrap();
        assert_eq!(informe.nuevos.len(), 2);
    } else {
        let informe = telefono.fusionar_eventos(&de_laptop).unwrap();
        assert_eq!(informe.nuevos.len(), 2);
        let informe = laptop.fusionar_eventos(&de_telefono).unwrap();
        assert_eq!(informe.nuevos.len(), 2);
    }

    assert_eq!(laptop.eventos_del_libro().unwrap().len(), 44);
    assert_eq!(telefono.eventos_del_libro().unwrap().len(), 44);
    assert_eq!(saldo(&laptop), "166.67", "saldo de la laptop");
    assert_eq!(saldo(&telefono), "166.67", "saldo del teléfono");
    let mut eventos_laptop = laptop.eventos_del_libro().unwrap();
    let mut eventos_telefono = telefono.eventos_del_libro().unwrap();
    aplicacion::ordenar_eventos(&mut eventos_laptop);
    aplicacion::ordenar_eventos(&mut eventos_telefono);
    assert_eq!(
        eventos_laptop, eventos_telefono,
        "mismo conjunto de eventos"
    );
    assert_eq!(
        laptop
            .detalle_cuenta("cuenta-1")
            .unwrap()
            .movimientos
            .iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>(),
        telefono
            .detalle_cuenta("cuenta-1")
            .unwrap()
            .movimientos
            .iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>(),
        "mismo orden de movimientos al derivar el saldo"
    );

    // Una sincronización posterior no agrega nada ni mueve el saldo.
    let informe = laptop.fusionar_eventos(&de_telefono).unwrap();
    assert!(informe.nuevos.is_empty());
    assert_eq!(informe.ya_existian, 2);
    assert_eq!(saldo(&laptop), "166.67");
}

#[test]
fn convergencia_laptop_primero() {
    comprobar_convergencia(true);
}

#[test]
fn convergencia_telefono_primero() {
    comprobar_convergencia(false);
}
