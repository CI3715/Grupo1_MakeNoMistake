//! Pruebas de aplicar la parte de un gasto de grupo al libro (Entrega 7).
//! Cubren INV-13, INV-15 e INV-18 y el rechazo de corregir desde el núcleo.

use aplicacion::{
    CasosDeUso, DatosCorregirMovimiento, DatosParteDeGrupo, DispositivoFijo, ErrorAplicacion,
    GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria, ResultadoAplicar,
};

fn entorno() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-20T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    )
}

fn cuenta(app: &mut CasosDeUso, nombre: &str, moneda: &str, saldo: &str) -> String {
    app.crear_cuenta(NuevaCuenta {
        nombre: nombre.to_string(),
        tipo: "efectivo".to_string(),
        moneda: moneda.to_string(),
        saldo_inicial: saldo.to_string(),
    })
    .unwrap()
    .id
}

fn saldo(app: &CasosDeUso, id: &str) -> String {
    app.listar_cuentas()
        .unwrap()
        .cuentas
        .into_iter()
        .find(|c| c.id == id)
        .expect("la cuenta debía existir")
        .saldo_actual
}

fn parte(evento: &str, cuenta_id: &str, monto: &str, moneda: &str) -> DatosParteDeGrupo {
    DatosParteDeGrupo {
        evento_servidor_id: evento.to_string(),
        grupo_id: "g1".to_string(),
        gasto_id: format!("gasto-{evento}"),
        cuenta_id: cuenta_id.to_string(),
        monto: monto.to_string(),
        moneda: moneda.to_string(),
        fecha: "2026-11-20".to_string(),
        concepto: "Viaje a Mérida · Hotel".to_string(),
        incluye_redondeo: false,
    }
}

fn mensaje_de_validacion(e: ErrorAplicacion) -> (Option<String>, String) {
    match e {
        ErrorAplicacion::Validacion(v) => (v[0].campo.clone(), v[0].mensaje.clone()),
        otro => panic!("esperaba un error de validación, llegó {otro:?}"),
    }
}

#[test]
fn idempotencia_el_mismo_evento_dos_veces_deja_el_mismo_saldo_que_una() {
    let mut app = entorno();
    let c = cuenta(&mut app, "Efectivo USD", "USD", "200.00");
    assert_eq!(
        app.aplicar_parte_de_grupo(parte("e-100", &c, "33.33", "USD"))
            .unwrap(),
        ResultadoAplicar::Aplicada
    );
    assert_eq!(saldo(&app, &c), "166.67");
    assert_eq!(
        app.aplicar_parte_de_grupo(parte("e-100", &c, "33.33", "USD"))
            .unwrap(),
        ResultadoAplicar::YaAplicada
    );
    assert_eq!(saldo(&app, &c), "166.67");
}

#[test]
fn idempotencia_un_lote_repetido_reproduce_el_ejemplo_2_6() {
    // Hotel 33,33 y cena 15,00; luego el cursor vuelve a cero y baja todo otra vez.
    let mut app = entorno();
    let c = cuenta(&mut app, "Efectivo USD", "USD", "200.00");
    let lote = [("e-100", "33.33"), ("e-101", "15.00")];
    for _ in 0..3 {
        for (evento, monto) in lote {
            app.aplicar_parte_de_grupo(parte(evento, &c, monto, "USD"))
                .unwrap();
        }
    }
    assert_eq!(saldo(&app, &c), "151.67");
}

#[test]
fn una_cuenta_de_otra_moneda_se_rechaza_y_no_cambia_el_saldo() {
    let mut app = entorno();
    let euros = cuenta(&mut app, "Efectivo EUR", "EUR", "200.00");
    let error = app
        .aplicar_parte_de_grupo(parte("e-100", &euros, "33.33", "USD"))
        .unwrap_err();
    let (campo, _) = mensaje_de_validacion(error);
    assert_eq!(campo.as_deref(), Some("cuenta_id"));
    assert_eq!(saldo(&app, &euros), "200.00");
}

#[test]
fn una_cuenta_inexistente_se_rechaza() {
    let mut app = entorno();
    let error = app
        .aplicar_parte_de_grupo(parte("e-100", "no-existe", "33.33", "USD"))
        .unwrap_err();
    let (campo, _) = mensaje_de_validacion(error);
    assert_eq!(campo.as_deref(), Some("cuenta_id"));
}

#[test]
fn un_monto_invalido_se_rechaza() {
    let mut app = entorno();
    let c = cuenta(&mut app, "Efectivo USD", "USD", "200.00");
    for malo in ["0.00", "-5.00", "33.333", "abc", ""] {
        let error = app
            .aplicar_parte_de_grupo(parte("e-100", &c, malo, "USD"))
            .unwrap_err();
        let (campo, _) = mensaje_de_validacion(error);
        assert_eq!(campo.as_deref(), Some("monto"), "monto {malo:?}");
    }
    assert_eq!(saldo(&app, &c), "200.00");
}

#[test]
fn un_movimiento_de_grupo_no_se_puede_corregir_desde_el_nucleo() {
    let mut app = entorno();
    let c = cuenta(&mut app, "Efectivo USD", "USD", "200.00");
    app.aplicar_parte_de_grupo(parte("e-100", &c, "33.33", "USD"))
        .unwrap();
    let error = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: "e-100".to_string(),
            monto_correcto: "10.00".to_string(),
        })
        .unwrap_err();
    let (campo, mensaje) = mensaje_de_validacion(error);
    assert_eq!(campo, None);
    assert_eq!(mensaje, "Se corrige en el grupo.");
    assert_eq!(saldo(&app, &c), "166.67");
}
