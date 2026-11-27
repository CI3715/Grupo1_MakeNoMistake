//! Pruebas de exportar el libro (Entrega 8, RF-INT-01).

use aplicacion::{
    CasosDeUso, DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria,
};

fn entorno() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-24T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    )
}

fn cuenta(app: &mut CasosDeUso, nombre: &str) {
    app.crear_cuenta(NuevaCuenta {
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "10.00".to_string(),
    })
    .unwrap();
}

#[test]
fn exportar_devuelve_todos_los_eventos() {
    let mut app = entorno();
    cuenta(&mut app, "Efectivo");
    cuenta(&mut app, "Banco");
    let eventos = app.eventos_del_libro().unwrap();
    assert_eq!(eventos.len(), 2);
    assert!(eventos.iter().all(|e| e.tipo == "CuentaCreada"));
    assert_eq!(eventos[0].dispositivo_origen, "disp-1");
}

#[test]
fn exportar_no_modifica_el_libro() {
    let mut app = entorno();
    cuenta(&mut app, "Efectivo");
    let antes = app.eventos_del_libro().unwrap();
    let _ = app.eventos_del_libro().unwrap();
    let despues = app.eventos_del_libro().unwrap();
    assert_eq!(antes, despues);
    assert_eq!(app.listar_cuentas().unwrap().cuentas.len(), 1);
}
