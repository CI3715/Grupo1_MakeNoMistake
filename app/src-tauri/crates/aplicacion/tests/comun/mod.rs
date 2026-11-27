//! Libro de ejemplo compartido por las pruebas del formato de exportación (Entrega 8:
//! John - Tareas 2 y 4). Tiene un evento de cada tipo y los casos que cambian un saldo:
//! ingresos, gastos, correcciones (de las dos formas), partes de grupo y monedas propias.

#![allow(dead_code)]

use aplicacion::{
    CasosDeUso, Cifrador, DatosCorregirMovimiento, DatosParteDeGrupo, DispositivoFijo,
    ErrorRespaldo, EventoRegistrado, GeneradorSecuencial, NuevaCategoria, NuevaCuenta, NuevaMoneda,
    NuevaTasa, NuevoMovimiento, RelojFijo, RepositorioEnMemoria,
};

pub const AHORA: &str = "2026-11-26T10:00:00Z";

/// Cifrador de mentira: las pruebas del formato usan el texto sin cifrar.
pub struct CifradorFalso;

impl Cifrador for CifradorFalso {
    fn cifrar(&self, _frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(claro.iter().map(|b| b ^ 0x5A).collect())
    }
    fn descifrar(&self, _frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(archivo.iter().map(|b| b ^ 0x5A).collect())
    }
}

pub fn libro_vacio(dispositivo: &str) -> CasosDeUso {
    CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo(AHORA)),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo(dispositivo)),
    )
}

fn cuenta(app: &mut CasosDeUso, nombre: &str, tipo: &str, moneda: &str, saldo: &str) -> String {
    app.crear_cuenta(NuevaCuenta {
        nombre: nombre.to_string(),
        tipo: tipo.to_string(),
        moneda: moneda.to_string(),
        saldo_inicial: saldo.to_string(),
    })
    .unwrap()
    .id
}

fn movimiento(
    app: &mut CasosDeUso,
    cuenta_id: &str,
    tipo: &str,
    monto: &str,
    categoria: &str,
    nota: Option<&str>,
) -> String {
    app.registrar_movimiento(NuevoMovimiento {
        cuenta_id: cuenta_id.to_string(),
        tipo: tipo.to_string(),
        monto: monto.to_string(),
        fecha: "2026-11-20".to_string(),
        categoria_id: categoria.to_string(),
        nota: nota.map(str::to_string),
    })
    .unwrap()
    .id
}

fn corregir(app: &mut CasosDeUso, movimiento_id: &str, monto: &str) {
    app.corregir_movimiento(DatosCorregirMovimiento {
        movimiento_id: movimiento_id.to_string(),
        monto_correcto: monto.to_string(),
    })
    .unwrap();
}

/// Un libro con un evento de cada tipo, varias monedas y todo lo que mueve un saldo.
pub fn libro_completo() -> CasosDeUso {
    let mut app = libro_vacio("laptop-ana");

    app.crear_moneda(NuevaMoneda {
        codigo: "BTC".to_string(),
        nombre: "Bitcoin".to_string(),
        simbolo: "₿".to_string(),
        decimales: "8".to_string(),
    })
    .unwrap();
    let mascotas = app
        .crear_categoria(NuevaCategoria {
            nombre: "Mascotas \"y\" más".to_string(),
            tipo: "gasto".to_string(),
        })
        .unwrap()
        .id;

    let efectivo = cuenta(&mut app, "Efectivo", "efectivo", "USD", "100.00");
    let banco = cuenta(&mut app, "Banco «Bicentenario»", "banco", "VES", "1500.50");
    let euros = cuenta(&mut app, "Ahorro en euros", "banco", "EUR", "0.00");
    let cripto = cuenta(&mut app, "Billetera fría", "efectivo", "BTC", "0.01250000");
    let usdt = cuenta(&mut app, "Binance", "efectivo", "USDT", "50.00");

    // Ingresos y gastos.
    movimiento(
        &mut app,
        &efectivo,
        "ingreso",
        "250.00",
        "ingreso:salario",
        Some("Quincena"),
    );
    let almuerzo = movimiento(
        &mut app,
        &efectivo,
        "egreso",
        "10.50",
        "gasto:alimentacion",
        Some("Almuerzo con \"Luis\"\nen el centro"),
    );
    let veterinario = movimiento(&mut app, &efectivo, "egreso", "35.00", &mascotas, None);
    movimiento(
        &mut app,
        &banco,
        "egreso",
        "320.25",
        "gasto:alimentacion",
        None,
    );
    movimiento(
        &mut app,
        &banco,
        "ingreso",
        "4000.00",
        "ingreso:salario",
        None,
    );
    movimiento(
        &mut app,
        &euros,
        "ingreso",
        "120.00",
        "ingreso:salario",
        None,
    );
    movimiento(
        &mut app,
        &cripto,
        "egreso",
        "0.00012345",
        "gasto:alimentacion",
        None,
    );
    let comision = movimiento(
        &mut app,
        &usdt,
        "egreso",
        "1.00",
        "gasto:alimentacion",
        None,
    );

    // Correcciones: una simple, otra corregida dos veces y una que sube un gasto.
    corregir(&mut app, &almuerzo, "12.00");
    corregir(&mut app, &veterinario, "30.00");
    corregir(&mut app, &veterinario, "31.75");
    corregir(&mut app, &comision, "0.50");

    // Tasas: una vigente, otra eliminada, y cambio de moneda base.
    app.registrar_tasa(NuevaTasa {
        moneda_origen: "USD".to_string(),
        moneda_destino: "VES".to_string(),
        valor: "36.50".to_string(),
        fecha: "2026-11-20".to_string(),
        fuente: "BCV".to_string(),
    })
    .unwrap();
    let errada = app
        .registrar_tasa(NuevaTasa {
            moneda_origen: "EUR".to_string(),
            moneda_destino: "USD".to_string(),
            valor: "10.00".to_string(),
            fecha: "2026-11-20".to_string(),
            fuente: "Manual".to_string(),
        })
        .unwrap()
        .id;
    app.eliminar_tasa(&errada).unwrap();
    app.cambiar_moneda_base("VES").unwrap();

    // Partes de gastos de grupo aplicadas a cuentas del libro.
    for (evento, cuenta_id, monto, moneda) in [
        ("srv-evt-0001", &efectivo, "5.00", "USD"),
        ("srv-evt-0002", &banco, "182.75", "VES"),
    ] {
        app.aplicar_parte_de_grupo(DatosParteDeGrupo {
            evento_servidor_id: evento.to_string(),
            grupo_id: "grupo-1".to_string(),
            gasto_id: format!("gasto-{evento}"),
            cuenta_id: cuenta_id.to_string(),
            monto: monto.to_string(),
            moneda: moneda.to_string(),
            fecha: "2026-11-21".to_string(),
            concepto: "Viaje a Mérida · Hotel".to_string(),
            incluye_redondeo: false,
        })
        .unwrap();
    }

    // Una corrección sin `efecto`, la forma que guardaron bases anteriores: llega desde
    // otro dispositivo. La aplicación no la cuenta en el saldo y el script tampoco.
    let taxi = movimiento(
        &mut app,
        &efectivo,
        "egreso",
        "8.00",
        "gasto:alimentacion",
        None,
    );
    let reemplazo = EventoRegistrado {
        id: "evt-correccion-completa".to_string(),
        tipo: "MovimientoCorregido".to_string(),
        momento_registro: "2026-11-26T11:00:00Z".to_string(),
        dispositivo_origen: "telefono-ana".to_string(),
        datos: serde_json::json!({
            "id": "mov-reemplazo-taxi",
            "cuenta_id": efectivo,
            "tipo": "egreso",
            "monto": "7.25",
            "fecha": "2026-11-20",
            "categoria_id": "gasto:alimentacion",
            "movimiento_original_id": taxi,
        })
        .to_string(),
    };
    let informe = app.fusionar_eventos(&[reemplazo]).unwrap();
    assert_eq!(
        informe.nuevos.len(),
        1,
        "la corrección antigua debía guardarse"
    );
    assert!(informe.rechazados.is_empty());

    app
}

/// (id, nombre, moneda, saldo) de cada cuenta según la aplicación, ordenado por id.
pub fn saldos_de_la_app(app: &CasosDeUso) -> Vec<(String, String, String, String)> {
    let mut saldos: Vec<_> = app
        .listar_cuentas()
        .unwrap()
        .cuentas
        .into_iter()
        .map(|c| (c.id, c.nombre, c.moneda, c.saldo_actual))
        .collect();
    saldos.sort();
    saldos
}

/// (moneda, total) de los subtotales por moneda según la aplicación, ordenado por moneda.
pub fn totales_de_la_app(app: &CasosDeUso) -> Vec<(String, String)> {
    let mut totales: Vec<_> = app
        .listar_cuentas()
        .unwrap()
        .subtotales
        .into_iter()
        .map(|s| (s.moneda, s.total))
        .collect();
    totales.sort();
    totales
}
