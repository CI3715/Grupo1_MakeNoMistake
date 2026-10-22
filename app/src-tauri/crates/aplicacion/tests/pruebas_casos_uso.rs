use aplicacion::contrato::bateria_contrato_repositorio;
use aplicacion::puertos::{
    ErrorRepositorio, EventoRegistrado, RepositorioEventos, ResultadoGuardado,
};
use aplicacion::{
    CasosDeUso, DispositivoFijo, ErrorAplicacion, GeneradorSecuencial, NuevaCuenta, RelojFijo,
    RepositorioEnMemoria,
};

fn preparar_entorno() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-10-14T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-123")),
    )
}

#[test]
fn contrato_repositorio_en_memoria_pasa_bateria_completa() {
    bateria_contrato_repositorio(RepositorioEnMemoria::nuevo);
}

#[test]
fn inv01_catalogo_de_monedas_y_todo_monto_lleva_su_moneda() {
    let mut app = preparar_entorno();
    let monedas = app.listar_monedas();
    assert_eq!(monedas.len(), 4);
    for codigo in ["VES", "USD", "EUR", "USDT"] {
        assert!(
            monedas
                .iter()
                .any(|m| m.codigo == codigo && m.decimales == 2)
        );
    }

    let creada = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cartera".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "VES".to_string(),
            saldo_inicial: "100.00".to_string(),
        })
        .unwrap();
    assert_eq!(creada.moneda, "VES");

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas[0].moneda, "VES");
    assert_eq!(lista.subtotales[0].moneda, "VES");
}

#[test]
fn inv02_subtotales_separados_por_moneda_y_caso_vacio() {
    let mut app = preparar_entorno();

    let vacia = app.listar_cuentas().unwrap();
    assert!(vacia.cuentas.is_empty());
    assert!(vacia.subtotales.is_empty());

    app.crear_cuenta(NuevaCuenta {
        nombre: "Efectivo VES".to_string(),
        tipo: "efectivo".to_string(),
        moneda: "VES".to_string(),
        saldo_inicial: "1250.50".to_string(),
    })
    .unwrap();
    app.crear_cuenta(NuevaCuenta {
        nombre: "Cuenta USD".to_string(),
        tipo: "banco".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "300.00".to_string(),
    })
    .unwrap();
    app.crear_cuenta(NuevaCuenta {
        nombre: "Billetera USDT".to_string(),
        tipo: "billetera_digital".to_string(),
        moneda: "USDT".to_string(),
        saldo_inicial: "50.00".to_string(),
    })
    .unwrap();

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas.len(), 3);
    assert_eq!(lista.subtotales.len(), 3);
    assert_eq!(lista.subtotales[0].moneda, "VES");
    assert_eq!(lista.subtotales[0].total, "1250.50");
    assert_eq!(lista.subtotales[1].moneda, "USD");
    assert_eq!(lista.subtotales[1].total, "300.00");
    assert_eq!(lista.subtotales[2].moneda, "USDT");
    assert_eq!(lista.subtotales[2].total, "50.00");
}

#[test]
fn inv03_aritmetica_decimal_exacta_y_formateo_con_decimales_de_la_moneda() {
    let mut app = preparar_entorno();

    app.crear_cuenta(NuevaCuenta {
        nombre: "Cuenta 1".to_string(),
        tipo: "efectivo".to_string(),
        moneda: "VES".to_string(),
        saldo_inicial: "0.10".to_string(),
    })
    .unwrap();
    app.crear_cuenta(NuevaCuenta {
        nombre: "Cuenta 2".to_string(),
        tipo: "banco".to_string(),
        moneda: "VES".to_string(),
        saldo_inicial: "0.20".to_string(),
    })
    .unwrap();
    let cuenta_entera = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cuenta USD".to_string(),
            tipo: "banco".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "300".to_string(),
        })
        .unwrap();

    // "300" se formatea como "300.00"
    assert_eq!(cuenta_entera.saldo_actual, "300.00");

    // 0.10 + 0.20 da exactamente 0.30
    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.subtotales[0].total, "0.30");
    assert_eq!(lista.subtotales[1].total, "300.00");
}

#[test]
fn inv13_idempotencia_guardar_y_proyectar_dos_veces_el_mismo_evento() {
    let mut repo = RepositorioEnMemoria::nuevo();
    let evento = EventoRegistrado {
        id: "evento-unico-1".to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-10-14T12:00:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"VES","saldo_inicial":"10.00"}"#.to_string(),
    };
    let desconocido = EventoRegistrado {
        id: "evento-futuro-2".to_string(),
        tipo: "MovimientoRegistrado".to_string(),
        momento_registro: "2026-10-14T12:01:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: "{}".to_string(),
    };

    assert_eq!(repo.guardar(&evento).unwrap(), ResultadoGuardado::Guardado);
    assert_eq!(repo.guardar(&evento).unwrap(), ResultadoGuardado::YaExistia);
    repo.guardar(&desconocido).unwrap();

    let app = CasosDeUso::nuevo(
        Box::new(repo),
        Box::new(RelojFijo::nuevo("2026-10-14T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas.len(), 1);
}

#[test]
fn inv18_moneda_de_cuenta_no_cambia_tras_crearse() {
    let mut app = preparar_entorno();
    app.crear_cuenta(NuevaCuenta {
        nombre: "Cartera".to_string(),
        tipo: "efectivo".to_string(),
        moneda: "VES".to_string(),
        saldo_inicial: "100.00".to_string(),
    })
    .unwrap();

    let intento = app.crear_cuenta(NuevaCuenta {
        nombre: "Cartera".to_string(),
        tipo: "efectivo".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "100.00".to_string(),
    });
    assert!(intento.is_err());

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas.len(), 1);
    assert_eq!(lista.cuentas[0].moneda, "VES");
}

#[test]
fn validacion_nombre_vacio_y_solo_espacios_se_rechaza() {
    let mut app = preparar_entorno();
    for invalido in ["", "   "] {
        let err = app
            .crear_cuenta(NuevaCuenta {
                nombre: invalido.to_string(),
                tipo: "efectivo".to_string(),
                moneda: "VES".to_string(),
                saldo_inicial: "10.00".to_string(),
            })
            .unwrap_err();
        match err {
            ErrorAplicacion::Validacion(lista) => {
                assert!(lista.iter().any(|e| e.campo.as_deref() == Some("nombre")));
            }
            _ => panic!("Se esperaba error en campo nombre"),
        }
    }
}

#[test]
fn validacion_nombre_repetido_sin_distinguir_mayusculas_se_rechaza() {
    let mut app = preparar_entorno();
    app.crear_cuenta(NuevaCuenta {
        nombre: "Cuenta Nómina".to_string(),
        tipo: "banco".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "100.00".to_string(),
    })
    .unwrap();

    let err = app
        .crear_cuenta(NuevaCuenta {
            nombre: "  cUeNtA nÓmInA  ".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "VES".to_string(),
            saldo_inicial: "50.00".to_string(),
        })
        .unwrap_err();
    match err {
        ErrorAplicacion::Validacion(lista) => {
            assert!(lista.iter().any(|e| e.campo.as_deref() == Some("nombre")));
        }
        _ => panic!("Se esperaba error en campo nombre"),
    }
}

#[test]
fn validacion_moneda_desconocida_se_rechaza() {
    let mut app = preparar_entorno();
    let err = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cripto".to_string(),
            tipo: "billetera_digital".to_string(),
            moneda: "BTC".to_string(),
            saldo_inicial: "1.00".to_string(),
        })
        .unwrap_err();
    match err {
        ErrorAplicacion::Validacion(lista) => {
            assert!(lista.iter().any(|e| e.campo.as_deref() == Some("moneda")));
        }
        _ => panic!("Se esperaba error en campo moneda"),
    }
}

#[test]
fn validacion_saldo_negativo_decimales_extra_no_numerico_y_cero_valido() {
    let mut app = preparar_entorno();

    for saldo_invalido in ["-10.00", "10.555", "no_es_numero"] {
        let err = app
            .crear_cuenta(NuevaCuenta {
                nombre: "Ahorros".to_string(),
                tipo: "banco".to_string(),
                moneda: "USD".to_string(),
                saldo_inicial: saldo_invalido.to_string(),
            })
            .unwrap_err();
        match err {
            ErrorAplicacion::Validacion(lista) => {
                assert!(
                    lista
                        .iter()
                        .any(|e| e.campo.as_deref() == Some("saldo_inicial"))
                );
            }
            _ => panic!("Se esperaba error en campo saldo_inicial"),
        }
    }

    let cuenta_cero = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cuenta en cero".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "0".to_string(),
        })
        .unwrap();
    assert_eq!(cuenta_cero.saldo_actual, "0.00");
}

#[test]
fn validacion_varios_errores_a_la_vez_sin_consumir_id_y_orden_de_creacion() {
    let mut app = preparar_entorno();

    let err = app
        .crear_cuenta(NuevaCuenta {
            nombre: "".to_string(),
            tipo: "tarjeta".to_string(),
            moneda: "XYZ".to_string(),
            saldo_inicial: "-5.00".to_string(),
        })
        .unwrap_err();
    match err {
        ErrorAplicacion::Validacion(lista) => {
            assert_eq!(lista.len(), 4);
            assert_eq!(lista[0].campo.as_deref(), Some("nombre"));
            assert_eq!(lista[1].campo.as_deref(), Some("tipo"));
            assert_eq!(lista[2].campo.as_deref(), Some("moneda"));
            assert_eq!(lista[3].campo.as_deref(), Some("saldo_inicial"));
        }
        _ => panic!("Se esperaba lista de errores acumulados"),
    }

    // La primera cuenta válida debe seguir recibiendo el ID 1
    let primera = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Primera".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "VES".to_string(),
            saldo_inicial: "10.00".to_string(),
        })
        .unwrap();
    assert_eq!(primera.id, "00000000-0000-4000-8000-000000000001");

    let segunda = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Segunda".to_string(),
            tipo: "banco".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "20.00".to_string(),
        })
        .unwrap();
    assert_eq!(segunda.id, "00000000-0000-4000-8000-000000000003");

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas[0].nombre, "Primera");
    assert_eq!(lista.cuentas[1].nombre, "Segunda");

    let err_repo = ErrorAplicacion::from(ErrorRepositorio("fallo".to_string()));
    assert!(matches!(err_repo, ErrorAplicacion::Almacenamiento(_)));
}
