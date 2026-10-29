//! Pruebas de integración de los casos de uso y del repositorio en memoria.
//!
//! Verifica las invariantes del sistema (INV-01, INV-02, INV-03, INV-04, INV-05,
//! INV-13, INV-18), las validaciones de cuentas, categorías, movimientos y
//! correcciones, y reproduce el escenario del ejemplo 2.2 del enunciado.

use aplicacion::contrato::bateria_contrato_repositorio;
use aplicacion::puertos::{
    ErrorRepositorio, EventoRegistrado, RepositorioEventos, ResultadoGuardado,
};
use aplicacion::{
    CasosDeUso, DatosCorregirMovimiento, DatosPrevisualizarMovimiento, DispositivoFijo,
    ErrorAplicacion, GeneradorSecuencial, NuevaCategoria, NuevaCuenta, NuevaMoneda,
    NuevaMonedaPropia, NuevaTasa, NuevoMovimiento, RelojFijo, RelojModificable,
    RepositorioEnMemoria,
};

/// Crea una instancia de [`CasosDeUso`] configurada con adaptadores deterministas en memoria.
fn preparar_entorno() -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-10-19T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-123")),
    )
}

/// Comprueba que la implementación en memoria cumpla el contrato del puerto [`RepositorioEventos`].
#[test]
fn contrato_repositorio_en_memoria_pasa_bateria_completa() {
    bateria_contrato_repositorio(RepositorioEnMemoria::nuevo);
}

/// Comprueba que el catálogo contenga las cuatro monedas base con 2 decimales
/// y que cada cuenta y subtotal conserve su moneda asociada (INV-01).
#[test]
fn inv01_catalogo_de_monedas_y_todo_monto_lleva_su_moneda() {
    let mut app = preparar_entorno();
    let monedas = app.listar_monedas().unwrap();
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

/// Comprueba que los subtotales se calculen por separado para cada moneda sin mezclarse (INV-02).
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

/// Comprueba que la suma de saldos utilice aritmética decimal exacta (`0.10 + 0.20 == 0.30`)
/// y que los montos enteros se formateen con los decimales de la moneda (INV-03).
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

    assert_eq!(cuenta_entera.saldo_actual, "300.00");

    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.subtotales[0].total, "0.30");
    assert_eq!(lista.subtotales[1].total, "300.00");
}

/// Comprueba que guardar y proyectar dos veces el mismo evento e ignorar eventos desconocidos
/// produzca exactamente el mismo estado (INV-13).
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
        tipo: "EventoDesconocido".to_string(),
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

/// Reproduce paso a paso el ejemplo 2.2 del enunciado (M-01) y comprueba INV-04 e INV-05:
/// Cuenta «Billetera dólares» en USD con saldo inicial 1000.00 -> Ingreso Salario +800.00 (1800.00)
/// -> Gasto Alimentación -500.00 (1300.00) -> Ajuste a 50.00 (+450.00, saldo final 1750.00).
#[test]
fn ejemplo_2_2_billetera_dolares_e_invariantes_inv04_inv05() {
    let mut app = preparar_entorno();

    // Paso 1: Crear cuenta "Billetera dólares" en USD con saldo inicial 1000.00
    let cuenta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Billetera dólares".to_string(),
            tipo: "billetera_digital".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "1000.00".to_string(),
        })
        .unwrap();
    assert_eq!(cuenta.saldo_actual, "1000.00");

    // Paso 2: Ingreso Salario por 800.00 -> saldo 1800.00
    let ingreso = app
        .registrar_movimiento(NuevoMovimiento {
            cuenta_id: cuenta.id.clone(),
            tipo: "ingreso".to_string(),
            monto: "800.00".to_string(),
            fecha: "2026-10-18".to_string(),
            categoria_id: "Salario".to_string(),
            nota: Some("Quincena".to_string()),
        })
        .unwrap();
    assert_eq!(ingreso.monto_con_signo, "+800.00");
    assert_eq!(
        app.detalle_cuenta(&cuenta.id).unwrap().cuenta.saldo_actual,
        "1800.00"
    );

    // Paso 3: Gasto Alimentación por 500.00 (era 50, se anotó 500) -> saldo 1300.00
    let gasto = app
        .registrar_movimiento(NuevoMovimiento {
            cuenta_id: cuenta.id.clone(),
            tipo: "gasto".to_string(),
            monto: "500.00".to_string(),
            fecha: "2026-10-19".to_string(),
            categoria_id: "gasto:alimentacion".to_string(),
            nota: Some("Mercado".to_string()),
        })
        .unwrap();
    assert_eq!(gasto.monto_con_signo, "-500.00");
    assert_eq!(
        app.detalle_cuenta(&cuenta.id).unwrap().cuenta.saldo_actual,
        "1300.00"
    );

    // Paso 4: Ajuste del paso 3 indicando monto correcto 50.00 -> efecto +450.00, saldo 1750.00
    let ajuste = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: gasto.id.clone(),
            monto_correcto: "50.00".to_string(),
        })
        .unwrap();
    assert_eq!(ajuste.estado, "ajuste");
    assert_eq!(ajuste.monto, "450.00");
    assert_eq!(ajuste.monto_con_signo, "+450.00");
    assert_eq!(ajuste.corrige_a.as_deref(), Some(gasto.id.as_str()));

    // Verificación de INV-04 (original sigue visible y marcado como corregido) e INV-05 (composición y saldo)
    let detalle = app.detalle_cuenta(&cuenta.id).unwrap();
    assert_eq!(detalle.cuenta.saldo_actual, "1750.00");
    assert_eq!(detalle.composicion.saldo_inicial, "1000.00");
    assert_eq!(detalle.composicion.total_ingresos, "800.00");
    assert_eq!(detalle.composicion.total_gastos, "500.00");
    assert_eq!(detalle.composicion.total_ajustes, "450.00");

    // En la lista están los 3 registros: el ajuste más reciente, el gasto marcado como corregido y el ingreso
    assert_eq!(detalle.movimientos.len(), 3);
    assert_eq!(detalle.movimientos[0].id, ajuste.id);
    assert_eq!(detalle.movimientos[0].estado, "ajuste");
    assert_eq!(detalle.movimientos[1].id, gasto.id);
    assert_eq!(detalle.movimientos[1].estado, "corregido");
    assert_eq!(detalle.movimientos[1].ajustes, vec![ajuste.id.clone()]);
    assert_eq!(detalle.movimientos[2].id, ingreso.id);

    // Mis cuentas también refleja el saldo actualizado en la cuenta y en el subtotal por moneda
    let resumen = app.listar_cuentas().unwrap();
    assert_eq!(resumen.cuentas[0].saldo_actual, "1750.00");
    assert_eq!(resumen.subtotales[0].total, "1750.00");
}

/// Comprueba todas las validaciones de `RegistrarMovimiento` (monto cero, negativo,
/// exceso de decimales, categoría vacía, inexistente o de otro tipo, y fecha futura).
#[test]
fn validaciones_de_registrar_movimiento() {
    let mut app = preparar_entorno();
    let cuenta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Banco USD".to_string(),
            tipo: "banco".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "500.00".to_string(),
        })
        .unwrap();

    // 1. Monto cero, negativo y con más decimales de los que admite USD
    for monto_malo in ["0", "0.00", "-20.00", "10.555", "no_numerico"] {
        let err = app
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.id.clone(),
                tipo: "gasto".to_string(),
                monto: monto_malo.to_string(),
                fecha: "2026-10-19".to_string(),
                categoria_id: "gasto:alimentacion".to_string(),
                nota: None,
            })
            .unwrap_err();
        match err {
            ErrorAplicacion::Validacion(lista) => {
                assert!(lista.iter().any(|e| e.campo.as_deref() == Some("monto")));
            }
            _ => panic!("Se esperaba error en campo monto para {monto_malo}"),
        }
    }

    // 2. Categoría vacía, inexistente o del tipo equivocado (categoría de ingreso en un gasto)
    for cat_mala in ["   ", "gasto:inexistente", "ingreso:salario"] {
        let err = app
            .registrar_movimiento(NuevoMovimiento {
                cuenta_id: cuenta.id.clone(),
                tipo: "gasto".to_string(),
                monto: "25.00".to_string(),
                fecha: "2026-10-19".to_string(),
                categoria_id: cat_mala.to_string(),
                nota: None,
            })
            .unwrap_err();
        match err {
            ErrorAplicacion::Validacion(lista) => {
                assert!(
                    lista
                        .iter()
                        .any(|e| e.campo.as_deref() == Some("categoria"))
                );
            }
            _ => panic!("Se esperaba error en campo categoria para {cat_mala}"),
        }
    }

    // 3. Fecha futura (reloj fijo está en 2026-10-19)
    let err_fecha = app
        .registrar_movimiento(NuevoMovimiento {
            cuenta_id: cuenta.id.clone(),
            tipo: "gasto".to_string(),
            monto: "25.00".to_string(),
            fecha: "2026-10-20".to_string(),
            categoria_id: "gasto:alimentacion".to_string(),
            nota: None,
        })
        .unwrap_err();
    match err_fecha {
        ErrorAplicacion::Validacion(lista) => {
            assert!(lista.iter().any(|e| e.campo.as_deref() == Some("fecha")));
        }
        _ => panic!("Se esperaba error en campo fecha"),
    }
}

/// Comprueba las reglas de `CorregirMovimiento`: monto correcto 0 (anulación),
/// múltiples correcciones sucesivas sobre el valor vigente, rechazo de corregir un ajuste,
/// rechazo de movimiento inexistente y rechazo de monto igual al vigente.
#[test]
fn validaciones_y_casos_de_corregir_movimiento() {
    let mut app = preparar_entorno();
    let cuenta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Efectivo VES".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "VES".to_string(),
            saldo_inicial: "1000.00".to_string(),
        })
        .unwrap();

    let gasto = app
        .registrar_movimiento(NuevoMovimiento {
            cuenta_id: cuenta.id.clone(),
            tipo: "gasto".to_string(),
            monto: "200.00".to_string(),
            fecha: "2026-10-19".to_string(),
            categoria_id: "gasto:transporte".to_string(),
            nota: None,
        })
        .unwrap();

    // 1. Primera corrección a 150.00 -> ajuste +50.00, saldo 850.00
    let aj1 = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: gasto.id.clone(),
            monto_correcto: "150.00".to_string(),
        })
        .unwrap();
    assert_eq!(aj1.monto_con_signo, "+50.00");
    assert_eq!(
        app.detalle_cuenta(&cuenta.id).unwrap().cuenta.saldo_actual,
        "850.00"
    );

    // 2. Rechazo si se intenta corregir con el mismo monto vigente (150.00)
    let igual_vigente = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: gasto.id.clone(),
            monto_correcto: "150.00".to_string(),
        })
        .unwrap_err();
    assert!(matches!(igual_vigente, ErrorAplicacion::Validacion(_)));

    // 3. Segunda corrección a 0.00 (anula el gasto por completo) -> ajuste +150.00, saldo 1000.00
    let aj2 = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: gasto.id.clone(),
            monto_correcto: "0".to_string(),
        })
        .unwrap();
    assert_eq!(aj2.monto_con_signo, "+150.00");
    assert_eq!(
        app.detalle_cuenta(&cuenta.id).unwrap().cuenta.saldo_actual,
        "1000.00"
    );

    // 4. Rechazo al intentar corregir un ajuste en lugar del original
    let corr_ajuste = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: aj1.id.clone(),
            monto_correcto: "10.00".to_string(),
        })
        .unwrap_err();
    assert!(matches!(corr_ajuste, ErrorAplicacion::Validacion(_)));

    // 5. Rechazo al intentar corregir un movimiento inexistente
    let inex = app
        .corregir_movimiento(DatosCorregirMovimiento {
            movimiento_id: "id-inexistente".to_string(),
            monto_correcto: "10.00".to_string(),
        })
        .unwrap_err();
    assert!(matches!(inex, ErrorAplicacion::Validacion(_)));
}

/// Comprueba la creación y listado de categorías y la previsualización de saldo negativo.
#[test]
fn categorias_y_previsualizacion_de_saldo_negativo() {
    let mut app = preparar_entorno();
    let inicial = app.listar_categorias().unwrap();
    assert_eq!(inicial.len(), 11);

    let nueva = app
        .crear_categoria(NuevaCategoria {
            nombre: "Mascotas".to_string(),
            tipo: "gasto".to_string(),
        })
        .unwrap();
    assert!(!nueva.de_fabrica);
    assert_eq!(app.listar_categorias().unwrap().len(), 12);

    // Repetir nombre en el mismo tipo sin distinguir mayúsculas se rechaza
    let repetida = app.crear_categoria(NuevaCategoria {
        nombre: "  mAsCoTaS ".to_string(),
        tipo: "gasto".to_string(),
    });
    assert!(matches!(repetida, Err(ErrorAplicacion::Validacion(_))));

    // Previsualización de gasto que deja saldo negativo sin guardar nada
    let cuenta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Caja".to_string(),
            tipo: "efectivo".to_string(),
            moneda: "USD".to_string(),
            saldo_inicial: "100.00".to_string(),
        })
        .unwrap();

    let prev = app
        .previsualizar_movimiento(DatosPrevisualizarMovimiento {
            cuenta_id: cuenta.id.clone(),
            tipo: "gasto".to_string(),
            monto: "150.00".to_string(),
        })
        .unwrap();
    assert_eq!(prev.saldo_resultante, "-50.00");
    assert!(prev.deja_saldo_negativo);

    // El saldo de la cuenta sigue intacto en 100.00 porque previsualizar no guarda eventos
    assert_eq!(
        app.detalle_cuenta(&cuenta.id).unwrap().cuenta.saldo_actual,
        "100.00"
    );
}

/// Comprueba que aplicar o almacenar el mismo evento más de una vez produzca el mismo estado
/// para todos los tipos de evento de las Entregas 2 y 3 (INV-13).
#[test]
fn inv13_idempotencia_con_todos_los_eventos_nuevos() {
    let mut repo = RepositorioEnMemoria::nuevo();
    let ev_cuenta = EventoRegistrado {
        id: "ev-cta-1".to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-10-19T10:00:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: r#"{"id":"c1","nombre":"Cartera","tipo":"efectivo","moneda":"USD","saldo_inicial":"100.00"}"#.to_string(),
    };
    let ev_cat = EventoRegistrado {
        id: "ev-cat-1".to_string(),
        tipo: "CategoriaCreada".to_string(),
        momento_registro: "2026-10-19T10:01:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: r#"{"id":"cat-1","id_categoria":"cat-1","nombre":"Extra","tipo":"egreso","es_fabrica":false}"#.to_string(),
    };
    let ev_mov = EventoRegistrado {
        id: "ev-mov-1".to_string(),
        tipo: "MovimientoRegistrado".to_string(),
        momento_registro: "2026-10-19T10:02:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: r#"{"id":"m1","id_movimiento":"m1","cuenta_id":"c1","id_cuenta":"c1","tipo":"egreso","monto":"40.00","moneda":"USD","fecha":"2026-10-19","categoria_id":"cat-1","id_categoria":"cat-1","nota":" Café "}"#.to_string(),
    };
    let ev_corr = EventoRegistrado {
        id: "ev-corr-1".to_string(),
        tipo: "MovimientoCorregido".to_string(),
        momento_registro: "2026-10-19T10:03:00Z".to_string(),
        dispositivo_origen: "disp-1".to_string(),
        datos: r#"{"id_ajuste":"a1","id_movimiento_original":"m1","monto_correcto":"15.00","efecto":"25.00","moneda":"USD"}"#.to_string(),
    };

    for ev in [&ev_cuenta, &ev_cat, &ev_mov, &ev_corr] {
        assert_eq!(repo.guardar(ev).unwrap(), ResultadoGuardado::Guardado);
        assert_eq!(repo.guardar(ev).unwrap(), ResultadoGuardado::YaExistia);
    }

    let app = CasosDeUso::nuevo(
        Box::new(repo),
        Box::new(RelojFijo::nuevo("2026-10-19T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    let detalle = app.detalle_cuenta("c1").unwrap();
    assert_eq!(detalle.cuenta.saldo_actual, "85.00");
    assert_eq!(detalle.movimientos.len(), 2);
}

/// Comprueba que la moneda de una cuenta permanezca inalterada tras su creación (INV-18).
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

/// Comprueba las validaciones de creación de cuenta y conversión de errores.
#[test]
fn validaciones_de_crear_cuenta_y_errores_de_repositorio() {
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

    let err_repo = ErrorAplicacion::from(ErrorRepositorio("fallo".to_string()));
    assert!(matches!(err_repo, ErrorAplicacion::Almacenamiento(_)));
}

/// RF-CAM-01: una moneda propia entra al catálogo y se puede usar en una
/// cuenta nueva con sus propios decimales; los saldos se calculan en su escala.
#[test]
fn una_moneda_propia_se_crea_y_se_usa_en_una_cuenta_nueva() {
    let mut app = preparar_entorno();

    let btc = app
        .crear_moneda(NuevaMoneda {
            codigo: "BTC".to_string(),
            nombre: "Bitcoin".to_string(),
            simbolo: "â‚¿".to_string(),
            decimales: "8".to_string(),
        })
        .unwrap();
    assert_eq!(btc.decimales, 8);

    let monedas = app.listar_monedas().unwrap();
    assert_eq!(monedas.len(), 5);
    assert_eq!(
        monedas[4].codigo, "BTC",
        "las propias van después de las de fábrica"
    );

    let cuenta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Billetera fría".to_string(),
            tipo: "billetera_digital".to_string(),
            moneda: "BTC".to_string(),
            saldo_inicial: "0,00012345".to_string(),
        })
        .unwrap();
    assert_eq!(cuenta.saldo_actual, "0.00012345");

    app.registrar_movimiento(NuevoMovimiento {
        cuenta_id: cuenta.id.clone(),
        tipo: "ingreso".to_string(),
        monto: "0.00000005".to_string(),
        fecha: "2026-10-19".to_string(),
        categoria_id: "Regalos".to_string(),
        nota: None,
    })
    .unwrap();
    let lista = app.listar_cuentas().unwrap();
    assert_eq!(lista.cuentas[0].saldo_actual, "0.00012350");
    assert_eq!(lista.subtotales[0].moneda, "BTC");

    // Repetir el código o redefinir una de fábrica se rechaza por campo.
    for codigo in ["BTC", "USD"] {
        let err = app
            .crear_moneda(NuevaMoneda {
                codigo: codigo.to_string(),
                nombre: "Otra".to_string(),
                simbolo: "O".to_string(),
                decimales: "2".to_string(),
            })
            .unwrap_err();
        match err {
            ErrorAplicacion::Validacion(lista) => {
                assert_eq!(lista[0].campo.as_deref(), Some("codigo"), "{codigo}")
            }
            otro => panic!("se esperaba validación, llegó {otro:?}"),
        }
    }
    assert_eq!(app.listar_monedas().unwrap().len(), 5);

    // Decimales que no son un número: se informa ese campo junto con los demás.
    let err = app
        .crear_moneda(NuevaMoneda {
            codigo: "x".to_string(),
            nombre: "Otra".to_string(),
            simbolo: "O".to_string(),
            decimales: "dos".to_string(),
        })
        .unwrap_err();
    match err {
        ErrorAplicacion::Validacion(lista) => {
            let campos: Vec<_> = lista.iter().filter_map(|e| e.campo.as_deref()).collect();
            assert_eq!(campos, vec!["codigo", "decimales"]);
        }
        otro => panic!("se esperaba validación, llegó {otro:?}"),
    }
}

#[test]
fn test_entrega4_ejemplo_2_3_completo_y_patrimonio() {
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-10-23T18:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    // 1. Cuentas del ejemplo 2.3
    app.crear_cuenta(NuevaCuenta {
        nombre: "Banco Provincial".into(),
        tipo: "banco".into(),
        moneda: "VES".into(),
        saldo_inicial: "88700.00".into(),
    })
    .unwrap();

    app.crear_cuenta(NuevaCuenta {
        nombre: "Banesco Panamá".into(),
        tipo: "banco".into(),
        moneda: "USD".into(),
        saldo_inicial: "200.00".into(),
    })
    .unwrap();

    app.crear_cuenta(NuevaCuenta {
        nombre: "Binance USDT".into(),
        tipo: "billetera_digital".into(),
        moneda: "USDT".into(),
        saldo_inicial: "50.00".into(),
    })
    .unwrap();

    // 2. Tasas del ejemplo
    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USD".into(),
        moneda_destino: "VES".into(),
        valor: "887.00".into(),
        fecha: "2026-10-23".into(),
        fuente: "BCV".into(),
    })
    .unwrap();

    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USDT".into(),
        moneda_destino: "VES".into(),
        valor: "1000.00".into(),
        fecha: "2026-10-23".into(),
        fuente: "P2P".into(),
    })
    .unwrap();

    // 3. Consolidación con base VES
    app.cambiar_moneda_base("VES").unwrap();
    assert_eq!(app.obtener_moneda_base().unwrap(), "VES");

    let pat_ves = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_ves.moneda_base, "VES");
    assert_eq!(pat_ves.total, Some("316100.00".into()));
    assert!(pat_ves.faltantes.is_empty());
    assert_eq!(pat_ves.tasas_usadas.len(), 2);

    // 4. Cambiar base a USD: falta tasa USDT -> USD
    app.cambiar_moneda_base("USD").unwrap();
    let pat_usd_incompleto = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_usd_incompleto.moneda_base, "USD");
    assert_eq!(pat_usd_incompleto.total, None);
    assert_eq!(pat_usd_incompleto.faltantes.len(), 1);
    assert_eq!(pat_usd_incompleto.faltantes[0].origen, "USDT");
    assert_eq!(pat_usd_incompleto.faltantes[0].destino, "USD");
    // Tasa usada para VES -> USD es la inversa (1 / 887.00)
    assert_eq!(pat_usd_incompleto.tasas_usadas.len(), 1);
    assert!(pat_usd_incompleto.tasas_usadas[0].es_inversa);
    assert_eq!(pat_usd_incompleto.tasas_usadas[0].origen, "USD");
    assert_eq!(pat_usd_incompleto.tasas_usadas[0].destino, "VES");

    // 5. Registrar tasa USDT -> USD: 1 USDT = 1.00 USD
    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USDT".into(),
        moneda_destino: "USD".into(),
        valor: "1.00".into(),
        fecha: "2026-10-23".into(),
        fuente: "P2P".into(),
    })
    .unwrap();

    // 6. Reconsultar con base USD: total = 350.00 USD
    let pat_usd_completo = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_usd_completo.moneda_base, "USD");
    assert_eq!(pat_usd_completo.total, Some("350.00".into()));
    assert!(pat_usd_completo.faltantes.is_empty());
    assert_eq!(pat_usd_completo.tasas_usadas.len(), 2);
}

/// Comprueba la selección de la tasa vigente, desempate en la misma fecha (gana la última registrada)
/// y descarte de tasas con fecha posterior a la de consulta.
#[test]
fn test_entrega4_seleccion_tasa_vigente_y_desempate() {
    let reloj = RelojModificable::nuevo("2026-10-20T12:00:00Z");
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(reloj.clone()),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    app.crear_cuenta(NuevaCuenta {
        nombre: "Cuenta VES".into(),
        tipo: "banco".into(),
        moneda: "VES".into(),
        saldo_inicial: "1000.00".into(),
    })
    .unwrap();

    // Registrar varias tasas USD -> VES
    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USD".into(),
        moneda_destino: "VES".into(),
        valor: "880.00".into(),
        fecha: "2026-10-20".into(),
        fuente: "BCV".into(),
    })
    .unwrap();

    reloj.avanzar("2026-10-22T12:00:00Z");
    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USD".into(),
        moneda_destino: "VES".into(),
        valor: "885.00".into(),
        fecha: "2026-10-22".into(),
        fuente: "BCV mañana".into(),
    })
    .unwrap();

    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USD".into(),
        moneda_destino: "VES".into(),
        valor: "887.00".into(),
        fecha: "2026-10-22".into(),
        fuente: "BCV tarde".into(),
    })
    .unwrap();

    reloj.avanzar("2026-10-23T12:00:00Z");

    // Consulta al 2026-10-21: toma la del 2026-10-20 (880.00)
    let pat_21 = app.consultar_patrimonio(Some("2026-10-21".into())).unwrap();
    assert_eq!(pat_21.tasas_usadas[0].valor, "880.00");

    // Consulta al 2026-10-23: desempate en la fecha 2026-10-22, gana la registrada más recientemente en el libro (887.00)
    let pat_23 = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_23.tasas_usadas[0].valor, "887.00");
    assert_eq!(pat_23.tasas_usadas[0].fuente, "BCV tarde");
}

/// Comprueba la eliminación lógica de tasas y su impacto en las consultas de patrimonio.
#[test]
fn test_entrega4_eliminar_tasa_y_efectos() {
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-10-23T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    app.crear_cuenta(NuevaCuenta {
        nombre: "Banesco".into(),
        tipo: "banco".into(),
        moneda: "USD".into(),
        saldo_inicial: "100.00".into(),
    })
    .unwrap();

    app.cambiar_moneda_base("VES").unwrap();

    let tasa = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "USD".into(),
            moneda_destino: "VES".into(),
            valor: "887.00".into(),
            fecha: "2026-10-23".into(),
            fuente: "BCV".into(),
        })
        .unwrap();

    assert_eq!(app.listar_tasas().unwrap().len(), 1);
    let pat_con_tasa = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_con_tasa.total, Some("88700.00".into()));

    // Eliminar la tasa
    app.eliminar_tasa(&tasa.id).unwrap();
    assert!(app.listar_tasas().unwrap().is_empty());

    // Ahora la consolidación falla por falta de tasa
    let pat_sin_tasa = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_sin_tasa.total, None);
    assert_eq!(pat_sin_tasa.faltantes.len(), 1);

    // Intentar eliminarla otra vez falla
    let err = app.eliminar_tasa(&tasa.id).unwrap_err();
    assert!(matches!(err, ErrorAplicacion::Validacion(_)));
}

/// Comprueba la creación de monedas propias y su uso en cuentas y listados (RF-CAM-01).
#[test]
fn test_entrega4_monedas_propias_y_cuentas() {
    let mut app = preparar_entorno();

    // Listar inicial (4 monedas de fábrica)
    let iniciales = app.listar_monedas().unwrap();
    assert_eq!(iniciales.len(), 4);
    assert!(iniciales.iter().all(|m| m.de_fabrica));

    // Crear moneda propia
    let btc = app
        .crear_moneda(NuevaMonedaPropia {
            codigo: "BTC".into(),
            nombre: "Bitcoin".into(),
            simbolo: "₿".into(),
            decimales: "8".into(),
        })
        .unwrap();

    assert_eq!(btc.codigo, "BTC");
    assert_eq!(btc.decimales, 8);
    assert!(!btc.de_fabrica);

    // Listar ahora contiene 5 monedas
    let monedas = app.listar_monedas().unwrap();
    assert_eq!(monedas.len(), 5);
    assert!(monedas.iter().any(|m| m.codigo == "BTC" && !m.de_fabrica));

    // Crear cuenta con la moneda propia
    let cta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cold Wallet".into(),
            tipo: "billetera_digital".into(),
            moneda: "BTC".into(),
            saldo_inicial: "0.12345678".into(),
        })
        .unwrap();
    assert_eq!(cta.saldo_actual, "0.12345678");

    // Registrar gasto en BTC
    app.registrar_movimiento(NuevoMovimiento {
        cuenta_id: cta.id.clone(),
        tipo: "gasto".into(),
        monto: "0.02345678".into(),
        fecha: "2026-10-19".into(),
        categoria_id: "gasto:otros".into(),
        nota: Some("Transferencia".into()),
    })
    .unwrap();

    let detalle = app.detalle_cuenta(&cta.id).unwrap();
    assert_eq!(detalle.cuenta.saldo_actual, "0.10000000");
}

/// Comprueba todas las validaciones de `RegistrarTasa` y `CrearMoneda`.
#[test]
fn test_entrega4_validaciones_registrar_tasa_y_crear_moneda() {
    let mut app = preparar_entorno();

    // Moneda inexistente
    let err_tasa1 = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "XYZ".into(),
            moneda_destino: "USD".into(),
            valor: "1.00".into(),
            fecha: "2026-10-19".into(),
            fuente: "Fuente".into(),
        })
        .unwrap_err();
    assert!(matches!(err_tasa1, ErrorAplicacion::Validacion(_)));

    // Mismas monedas
    let err_tasa2 = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "USD".into(),
            moneda_destino: "USD".into(),
            valor: "1.00".into(),
            fecha: "2026-10-19".into(),
            fuente: "Fuente".into(),
        })
        .unwrap_err();
    assert!(matches!(err_tasa2, ErrorAplicacion::Validacion(_)));

    // Valor no válido / cero / negativo
    let err_tasa3 = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "USD".into(),
            moneda_destino: "VES".into(),
            valor: "0.00".into(),
            fecha: "2026-10-19".into(),
            fuente: "BCV".into(),
        })
        .unwrap_err();
    assert!(matches!(err_tasa3, ErrorAplicacion::Validacion(_)));

    // Fecha futura
    let err_tasa4 = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "USD".into(),
            moneda_destino: "VES".into(),
            valor: "887.00".into(),
            fecha: "2026-10-25".into(),
            fuente: "BCV".into(),
        })
        .unwrap_err();
    assert!(matches!(err_tasa4, ErrorAplicacion::Validacion(_)));

    // Validaciones de CrearMoneda
    // Código corto
    let err_mon1 = app
        .crear_moneda(NuevaMonedaPropia {
            codigo: "BT".into(),
            nombre: "Bitcoin".into(),
            simbolo: "₿".into(),
            decimales: "8".into(),
        })
        .unwrap_err();
    assert!(matches!(err_mon1, ErrorAplicacion::Validacion(_)));

    // Código existente
    let err_mon2 = app
        .crear_moneda(NuevaMonedaPropia {
            codigo: "USD".into(),
            nombre: "Dólar".into(),
            simbolo: "$".into(),
            decimales: "2".into(),
        })
        .unwrap_err();
    assert!(matches!(err_mon2, ErrorAplicacion::Validacion(_)));

    // Decimales > 8
    let err_mon3 = app
        .crear_moneda(NuevaMonedaPropia {
            codigo: "ETH".into(),
            nombre: "Ethereum".into(),
            simbolo: "Ξ".into(),
            decimales: "9".into(),
        })
        .unwrap_err();
    assert!(matches!(err_mon3, ErrorAplicacion::Validacion(_)));
}

/// Comprueba que `MovimientoListado` entregue el `monto_vigente` correcto tanto
/// para movimientos vigentes como para movimientos corregidos (C-03).
#[test]
fn test_entrega4_c03_monto_vigente_en_movimientos_corregidos() {
    let mut app = preparar_entorno();

    let cta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Banesco".into(),
            tipo: "banco".into(),
            moneda: "USD".into(),
            saldo_inicial: "100.00".into(),
        })
        .unwrap();

    let mov = app
        .registrar_movimiento(NuevoMovimiento {
            cuenta_id: cta.id.clone(),
            tipo: "gasto".into(),
            monto: "50.00".into(),
            fecha: "2026-10-19".into(),
            categoria_id: "gasto:alimentacion".into(),
            nota: Some("Almuerzo".into()),
        })
        .unwrap();

    // Inicialmente vigente: monto_vigente es Some("50.00")
    assert_eq!(mov.estado, "vigente");
    assert_eq!(mov.monto_vigente, Some("50.00".into()));

    // Corregir a 40.00
    app.corregir_movimiento(DatosCorregirMovimiento {
        movimiento_id: mov.id.clone(),
        monto_correcto: "40.00".into(),
    })
    .unwrap();

    let detalle = app.detalle_cuenta(&cta.id).unwrap();
    let original = detalle.movimientos.iter().find(|m| m.id == mov.id).unwrap();
    assert_eq!(original.estado, "corregido");
    assert_eq!(original.monto_vigente, Some("40.00".into()));

    // Corregir otra vez a 30.00
    app.corregir_movimiento(DatosCorregirMovimiento {
        movimiento_id: mov.id.clone(),
        monto_correcto: "30.00".into(),
    })
    .unwrap();

    let detalle2 = app.detalle_cuenta(&cta.id).unwrap();
    let original2 = detalle2
        .movimientos
        .iter()
        .find(|m| m.id == mov.id)
        .unwrap();
    assert_eq!(original2.monto_vigente, Some("30.00".into()));
}

/// Comprueba el filtro por fecha de corte en `consultar_patrimonio`
/// y que rechace fechas futuras (relativas al reloj).
#[test]
fn test_entrega4_fecha_corte_patrimonio() {
    let reloj = RelojModificable::nuevo("2026-10-20T12:00:00Z");
    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(reloj.clone()),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-1")),
    );

    let cta = app
        .crear_cuenta(NuevaCuenta {
            nombre: "Cuenta USD".into(),
            tipo: "banco".into(),
            moneda: "USD".into(),
            saldo_inicial: "100.00".into(),
        })
        .unwrap();

    reloj.avanzar("2026-10-21T12:00:00Z");
    app.registrar_movimiento(NuevoMovimiento {
        cuenta_id: cta.id.clone(),
        tipo: "ingreso".into(),
        monto: "50.00".into(),
        fecha: "2026-10-21".into(),
        categoria_id: "ingreso:salario".into(),
        nota: None,
    })
    .unwrap();

    reloj.avanzar("2026-10-23T12:00:00Z");
    app.registrar_movimiento(NuevoMovimiento {
        cuenta_id: cta.id.clone(),
        tipo: "ingreso".into(),
        monto: "30.00".into(),
        fecha: "2026-10-23".into(),
        categoria_id: "ingreso:otros".into(),
        nota: None,
    })
    .unwrap();

    // Consulta al 2026-10-21: solo cuenta el saldo inicial + primer ingreso = 150.00
    let pat_21 = app.consultar_patrimonio(Some("2026-10-21".into())).unwrap();
    assert_eq!(pat_21.cuentas[0].saldo, "150.00");
    assert_eq!(pat_21.total, Some("150.00".into()));

    // Consulta al 2026-10-23: cuenta ambos ingresos = 180.00
    let pat_23 = app.consultar_patrimonio(Some("2026-10-23".into())).unwrap();
    assert_eq!(pat_23.cuentas[0].saldo, "180.00");
    assert_eq!(pat_23.total, Some("180.00".into()));

    // Fecha futura relativa a reloj (2026-10-24 > 2026-10-23)
    let err_futura = app
        .consultar_patrimonio(Some("2026-10-24".into()))
        .unwrap_err();
    assert!(matches!(err_futura, ErrorAplicacion::Validacion(_)));
}
