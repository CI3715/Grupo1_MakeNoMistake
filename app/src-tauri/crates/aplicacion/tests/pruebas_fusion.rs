//! Pruebas de la fusión del libro y su conexión con importar copias (Entrega 8).
//!
//! Cubre idempotencia, orden indistinto, conflicto por id repetido, lotes antiguos
//! que no borran eventos y el caso de las partes de grupo de la Entrega 7.

use aplicacion::{
    Cifrador, DatosParteDeGrupo, DispositivoFijo, ErrorRespaldo, EventoRegistrado,
    GeneradorSecuencial, InformeFusion, RelojFijo, RepositorioEnMemoria, exportar_libro,
    id_evento_parte_de_grupo,
};

struct CifradorFalso;

impl Cifrador for CifradorFalso {
    fn cifrar(&self, _frase: &str, claro: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(claro.iter().map(|b| b ^ 0x5A).collect())
    }
    fn descifrar(&self, _frase: &str, archivo: &[u8]) -> Result<Vec<u8>, ErrorRespaldo> {
        Ok(archivo.iter().map(|b| b ^ 0x5A).collect())
    }
}

const FECHA: &str = "2026-11-24T15:00:00Z";

fn casos() -> aplicacion::CasosDeUso {
    aplicacion::CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-24T12:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("laptop")),
    )
}

fn evento(id: &str, tipo: &str, momento: &str, dispositivo: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: tipo.to_string(),
        momento_registro: momento.to_string(),
        dispositivo_origen: dispositivo.to_string(),
        datos: datos.to_string(),
    }
}

fn cuenta(id: &str, nombre: &str, momento: &str, dispositivo: &str) -> EventoRegistrado {
    evento(
        id,
        "CuentaCreada",
        momento,
        dispositivo,
        &format!(
            r#"{{"id":"{id}","nombre":"{nombre}","tipo":"efectivo","moneda":"USD","saldo_inicial":"0.00"}}"#
        ),
    )
}

const DATOS_UNO: &str =
    r#"{"id":"c1","nombre":"Uno","tipo":"efectivo","moneda":"USD","saldo_inicial":"0.00"}"#;
const DATOS_DOS: &str =
    r#"{"id":"c2","nombre":"Dos","tipo":"efectivo","moneda":"USD","saldo_inicial":"0.00"}"#;
const DATOS_DISTINTOS: &str =
    r#"{"id":"c1","nombre":"Otro","tipo":"efectivo","moneda":"USD","saldo_inicial":"0.00"}"#;

#[test]
fn fusionar_el_mismo_libro_dos_veces_no_cambia_nada() {
    let mut casos = casos();
    let eventos = vec![
        evento(
            "e1",
            "CuentaCreada",
            "2026-11-24T10:00:00Z",
            "laptop",
            DATOS_UNO,
        ),
        evento(
            "e2",
            "CuentaCreada",
            "2026-11-24T11:00:00Z",
            "laptop",
            DATOS_DOS,
        ),
    ];

    let primera = casos.fusionar_eventos(&eventos).expect("primera fusión");
    assert_eq!(primera.nuevos.len(), 2);

    let segunda = casos.fusionar_eventos(&eventos).expect("segunda fusión");
    assert_eq!(
        segunda,
        InformeFusion {
            nuevos: Vec::new(),
            ya_existian: 2,
            rechazados: Vec::new(),
        }
    );
    assert_eq!(casos.eventos_del_libro().expect("libro").len(), 2);
}

#[test]
fn el_orden_de_llegada_no_cambia_los_eventos_ni_los_saldos() {
    let a = evento(
        "e2",
        "CuentaCreada",
        "2026-11-24T10:00:00Z",
        "laptop",
        DATOS_DOS,
    );
    let b = evento(
        "e1",
        "CuentaCreada",
        "2026-11-24T11:00:00Z",
        "teléfono",
        DATOS_UNO,
    );
    let mut primero = casos();
    let mut segundo = casos();

    primero
        .fusionar_eventos(&[a.clone(), b.clone()])
        .expect("A luego B");
    segundo.fusionar_eventos(&[b, a]).expect("B luego A");

    assert_eq!(
        primero.eventos_del_libro().expect("libro A"),
        segundo.eventos_del_libro().expect("libro B")
    );
    assert_eq!(
        primero.listar_cuentas().expect("cuentas A"),
        segundo.listar_cuentas().expect("cuentas B")
    );
}

#[test]
fn un_conflicto_se_rechaza_y_el_resto_del_lote_se_aplica() {
    let mut casos = casos();
    let original = evento(
        "e1",
        "CuentaCreada",
        "2026-11-24T10:00:00Z",
        "laptop",
        DATOS_UNO,
    );
    casos.fusionar_eventos(&[original]).expect("evento local");

    let informe = casos
        .fusionar_eventos(&[
            evento(
                "e1",
                "CuentaCreada",
                "2026-11-24T12:00:00Z",
                "teléfono",
                DATOS_DISTINTOS,
            ),
            evento(
                "e2",
                "CuentaCreada",
                "2026-11-24T11:00:00Z",
                "teléfono",
                DATOS_DOS,
            ),
        ])
        .expect("fusión con conflicto");

    assert_eq!(informe.nuevos.len(), 1);
    assert_eq!(informe.nuevos[0].id, "e2");
    assert_eq!(informe.rechazados.len(), 1);
    assert_eq!(informe.rechazados[0].evento.id, "e1");
    assert!(!informe.rechazados[0].motivo.mensaje().is_empty());
    assert_eq!(casos.listar_cuentas().expect("cuentas").cuentas.len(), 2);
}

#[test]
fn un_lote_antiguo_no_elimina_eventos_locales() {
    let mut casos = casos();
    let eventos = vec![
        evento(
            "e1",
            "CuentaCreada",
            "2026-11-24T10:00:00Z",
            "laptop",
            DATOS_UNO,
        ),
        evento(
            "e2",
            "CuentaCreada",
            "2026-11-24T11:00:00Z",
            "laptop",
            DATOS_DOS,
        ),
    ];
    casos.fusionar_eventos(&eventos).expect("libro inicial");

    let informe = casos
        .fusionar_eventos(&[eventos[0].clone()])
        .expect("lote antiguo");

    assert!(informe.nuevos.is_empty());
    assert_eq!(informe.ya_existian, 1);
    assert_eq!(casos.eventos_del_libro().expect("libro").len(), 2);
}

fn libro_con_cuenta(dispositivo: &str, momento: &str) -> aplicacion::CasosDeUso {
    let mut casos = aplicacion::CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo(momento)),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo(dispositivo)),
    );
    casos
        .fusionar_eventos(&[cuenta(
            "c1",
            "Efectivo",
            "2026-11-24T09:00:00Z",
            dispositivo,
        )])
        .expect("cuenta inicial");
    casos
}

fn parte_de_grupo() -> DatosParteDeGrupo {
    DatosParteDeGrupo {
        evento_servidor_id: "e-100".to_string(),
        grupo_id: "g1".to_string(),
        gasto_id: "gasto-1".to_string(),
        cuenta_id: "c1".to_string(),
        monto: "5.00".to_string(),
        moneda: "USD".to_string(),
        fecha: "2026-11-24".to_string(),
        concepto: "Viaje · Hotel".to_string(),
        incluye_redondeo: false,
    }
}

#[test]
fn la_misma_parte_de_grupo_en_dos_dispositivos_se_deja_una_sola_vez() {
    let mut laptop = libro_con_cuenta("laptop", "2026-11-24T10:00:00Z");
    let mut telefono = libro_con_cuenta("teléfono", "2026-11-24T15:00:00Z");
    let datos = parte_de_grupo();

    let en_laptop = laptop
        .preparar_parte_de_grupo(datos.clone())
        .expect("preparar en la laptop")
        .expect("la parte aún no estaba aplicada");
    let en_telefono = telefono
        .preparar_parte_de_grupo(datos.clone())
        .expect("preparar en el teléfono")
        .expect("la parte aún no estaba aplicada");

    // El id local se deriva del id del servidor; el momento y el dispositivo son
    // propios de cada bajada y no alteran la identidad del evento.
    assert_eq!(en_laptop.id, id_evento_parte_de_grupo("e-100"));
    assert_eq!(en_telefono.id, en_laptop.id);
    assert_eq!(en_telefono.tipo, en_laptop.tipo);
    assert_eq!(en_telefono.datos, en_laptop.datos);
    assert_ne!(en_telefono.momento_registro, en_laptop.momento_registro);
    assert_ne!(en_telefono.dispositivo_origen, en_laptop.dispositivo_origen);

    // La idempotencia de la bajada sigue comprobándose por el id del servidor,
    // que viaja en la carga: repetirla no crea otro evento.
    assert_eq!(
        laptop
            .aplicar_parte_de_grupo(datos.clone())
            .expect("aplicar en la laptop"),
        aplicacion::ResultadoAplicar::Aplicada
    );
    assert_eq!(
        laptop
            .aplicar_parte_de_grupo(datos.clone())
            .expect("repetir la bajada en la laptop"),
        aplicacion::ResultadoAplicar::YaAplicada
    );
    assert!(
        laptop
            .preparar_parte_de_grupo(datos.clone())
            .expect("preparar de nuevo")
            .is_none()
    );

    // Cada dispositivo aplicó su propia copia; al fusionarlas queda una sola.
    assert_eq!(
        telefono
            .aplicar_parte_de_grupo(datos)
            .expect("aplicar en el teléfono"),
        aplicacion::ResultadoAplicar::Aplicada
    );
    let informe = telefono
        .fusionar_eventos(&[en_laptop])
        .expect("misma parte desde la laptop");

    assert!(informe.nuevos.is_empty());
    assert_eq!(informe.ya_existian, 1);
    assert!(informe.rechazados.is_empty());
    assert_eq!(telefono.eventos_del_libro().expect("libro").len(), 2);
    assert_eq!(
        telefono
            .detalle_cuenta("c1")
            .expect("detalle de la cuenta")
            .movimientos
            .len(),
        1
    );
    assert_eq!(
        laptop
            .listar_cuentas()
            .expect("cuentas de la laptop")
            .cuentas[0]
            .saldo_actual,
        telefono
            .listar_cuentas()
            .expect("cuentas del teléfono")
            .cuentas[0]
            .saldo_actual
    );
}

#[test]
fn importar_dos_veces_da_cero_nuevos_y_el_mismo_libro() {
    let mut casos = casos();
    let eventos = vec![
        evento(
            "e1",
            "CuentaCreada",
            "2026-11-24T10:00:00Z",
            "laptop",
            DATOS_UNO,
        ),
        evento(
            "e2",
            "CuentaCreada",
            "2026-11-24T11:00:00Z",
            "laptop",
            DATOS_DOS,
        ),
    ];
    let bytes = exportar_libro(&eventos, FECHA, None, &CifradorFalso).expect("exportar");
    let antes = casos.listar_cuentas().expect("cuentas vacías");

    let primera = casos
        .importar_copia(&bytes, None, &CifradorFalso)
        .expect("primera importación");
    let despues = casos.listar_cuentas().expect("cuentas importadas");
    let segunda = casos
        .importar_copia(&bytes, None, &CifradorFalso)
        .expect("segunda importación");

    assert_eq!(primera.nuevos.len(), 2);
    assert_eq!(segunda.nuevos.len(), 0, "0 nuevos al importar dos veces");
    assert_eq!(segunda.ya_existian, 2);
    assert!(antes.cuentas.is_empty());
    assert_eq!(casos.listar_cuentas().expect("cuentas finales"), despues);
}
