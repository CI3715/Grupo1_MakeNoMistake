//! Formato de exportación legible y reconstruible (Entrega 8: John - Tarea 2; RF-INT-01,
//! RF-INT-05, RNF-04). El formato está documentado en `docs/formato-exportacion.md`.

mod comun;

use aplicacion::{
    ErrorRespaldo, EventoRegistrado, VERSION_FORMATO, leer_encabezado, leer_libro, libro_a_texto,
    texto_a_libro,
};
use comun::{
    AHORA, CifradorFalso, libro_completo, libro_vacio, saldos_de_la_app, totales_de_la_app,
};

fn evento(id: &str, datos: &str) -> EventoRegistrado {
    EventoRegistrado {
        id: id.to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: "2026-11-26T10:00:00Z".to_string(),
        dispositivo_origen: "laptop".to_string(),
        datos: datos.to_string(),
    }
}

fn texto_valido() -> String {
    libro_a_texto(
        &[evento("e1", r#"{"a":1}"#), evento("e2", r#"{"b":2}"#)],
        AHORA,
    )
}

fn rechazado(texto: &str) -> ErrorRespaldo {
    texto_a_libro(texto).expect_err(&format!("debía rechazarse:\n{texto}"))
}

// ---------- Escritura ----------

#[test]
fn el_encabezado_lleva_version_fecha_y_cantidad() {
    let app = libro_completo();
    let eventos = app.eventos_del_libro().unwrap();
    let texto = libro_a_texto(&eventos, AHORA);
    let primera = texto.lines().next().unwrap();
    assert_eq!(
        primera,
        format!(
            "# cuentas-claras-respaldo version=1 exportado={AHORA} eventos={}",
            eventos.len()
        )
    );
    let encabezado = leer_encabezado(primera).unwrap();
    assert_eq!(encabezado.version, VERSION_FORMATO);
    assert_eq!(encabezado.exportado, AHORA);
    assert_eq!(encabezado.eventos, eventos.len());
    assert_eq!(texto.lines().count(), eventos.len() + 1);
    assert!(texto.ends_with('\n'));
}

#[test]
fn cada_linea_es_un_evento_con_las_claves_en_orden_y_los_datos_como_objeto() {
    let app = libro_completo();
    let texto = libro_a_texto(&app.eventos_del_libro().unwrap(), AHORA);
    for linea in texto.lines().skip(1) {
        assert!(linea.starts_with("{\"id\":\""), "línea: {linea}");
        let valor: serde_json::Value = serde_json::from_str(linea).unwrap();
        let objeto = valor.as_object().unwrap();
        assert_eq!(objeto.len(), 5, "línea: {linea}");
        let posiciones: Vec<usize> = [
            "\"id\":",
            "\"tipo\":",
            "\"momento_registro\":",
            "\"dispositivo_origen\":",
            "\"datos\":{",
        ]
        .iter()
        .map(|clave| linea.find(clave).expect("falta una clave"))
        .collect();
        assert!(posiciones.is_sorted(), "claves fuera de orden: {linea}");
        assert!(
            objeto["datos"].is_object(),
            "datos debe ser un objeto: {linea}"
        );
    }
    // Legible: los nombres se ven tal cual, sin desescapar.
    assert!(texto.contains(r#""nombre":"Banco «Bicentenario»""#));
    assert!(texto.contains(r#""saldo_inicial":"1500.50""#));
}

#[test]
fn el_texto_no_lleva_frase_ni_credenciales() {
    let app = libro_completo();
    let texto = libro_a_texto(&app.eventos_del_libro().unwrap(), AHORA);
    let minusculas = texto.to_lowercase();
    for prohibido in ["frase", "token", "contraseña", "password", "credencial"] {
        assert!(!minusculas.contains(prohibido), "aparece «{prohibido}»");
    }
}

// ---------- Ida y vuelta ----------

#[test]
fn ida_y_vuelta_sin_perdidas_con_un_libro_completo() {
    let app = libro_completo();
    let eventos = app.eventos_del_libro().unwrap();
    let tipos: std::collections::BTreeSet<&str> = eventos.iter().map(|e| e.tipo.as_str()).collect();
    for tipo in [
        "CuentaCreada",
        "CategoriaCreada",
        "MovimientoRegistrado",
        "MovimientoCorregido",
        "GastoDeGrupoAplicado",
        "MonedaCreada",
        "TasaRegistrada",
        "TasaEliminada",
        "MonedaBaseCambiada",
    ] {
        assert!(tipos.contains(tipo), "el libro de ejemplo no tiene {tipo}");
    }

    let leidos = texto_a_libro(&libro_a_texto(&eventos, AHORA)).unwrap();
    assert_eq!(leidos, eventos);
}

#[test]
fn ida_y_vuelta_conserva_los_datos_byte_por_byte() {
    let cargas = [
        r#"{"z":1,"a":2}"#,                   // claves sin ordenar
        r#"{"nombre":"Café ₿ «x» \"y\"\n"}"#, // escapes y unicode
        r#"{"monto":"10.50","n":1.0e2}"#,     // números tal cual
        r#"{ "con" : "espacios" }"#,          // espacios internos
        "{}",
        " {\"a\":1}",  // espacio alrededor: viaja como cadena
        "{\"a\":\n1}", // salto de línea: viaja como cadena
        "[1,2]",       // no es un objeto
        "no es json",
        "",
    ];
    let eventos: Vec<EventoRegistrado> = cargas
        .iter()
        .enumerate()
        .map(|(i, datos)| evento(&format!("e{i}"), datos))
        .collect();
    let texto = libro_a_texto(&eventos, AHORA);
    assert_eq!(
        texto.lines().count(),
        eventos.len() + 1,
        "una línea por evento"
    );
    assert_eq!(texto_a_libro(&texto).unwrap(), eventos);
}

#[test]
fn un_libro_vacio_tambien_va_y_vuelve() {
    let texto = libro_a_texto(&[], AHORA);
    assert_eq!(
        texto,
        format!("# cuentas-claras-respaldo version=1 exportado={AHORA} eventos=0\n")
    );
    assert!(texto_a_libro(&texto).unwrap().is_empty());
}

#[test]
fn importar_la_exportacion_en_un_libro_nuevo_reproduce_saldos_movimientos_y_tasas() {
    let origen = libro_completo();
    let bytes = origen.exportar_copia(None, &CifradorFalso).unwrap();

    let mut destino = libro_vacio("instalacion-nueva");
    let informe = destino
        .importar_copia(&bytes, None, &CifradorFalso)
        .unwrap();
    assert!(informe.rechazados.is_empty(), "{:?}", informe.rechazados);

    assert_eq!(saldos_de_la_app(&destino), saldos_de_la_app(&origen));
    assert_eq!(totales_de_la_app(&destino), totales_de_la_app(&origen));
    assert_eq!(
        destino.listar_tasas().unwrap(),
        origen.listar_tasas().unwrap()
    );
    assert_eq!(
        destino.obtener_moneda_base().unwrap(),
        origen.obtener_moneda_base().unwrap()
    );
    assert_eq!(
        destino.listar_monedas().unwrap(),
        origen.listar_monedas().unwrap()
    );
    for (id, ..) in saldos_de_la_app(&origen) {
        assert_eq!(
            destino.detalle_cuenta(&id).unwrap(),
            origen.detalle_cuenta(&id).unwrap(),
            "movimientos de la cuenta {id}"
        );
    }
}

#[test]
fn leer_libro_acepta_el_texto_y_el_cifrado() {
    let app = libro_completo();
    let eventos = app.eventos_del_libro().unwrap();
    let claro = app.exportar_copia(None, &CifradorFalso).unwrap();
    let cifrado = app
        .exportar_copia(Some("una frase de paso larga"), &CifradorFalso)
        .unwrap();
    assert_eq!(leer_libro(&claro, None, &CifradorFalso).unwrap(), eventos);
    assert_eq!(
        leer_libro(&cifrado, Some("una frase de paso larga"), &CifradorFalso).unwrap(),
        eventos
    );
}

// ---------- Lectura estricta ----------

#[test]
fn el_texto_valido_se_lee() {
    assert_eq!(texto_a_libro(&texto_valido()).unwrap().len(), 2);
}

#[test]
fn una_version_distinta_se_rechaza_con_su_numero() {
    let texto = texto_valido().replace("version=1", "version=2");
    assert_eq!(
        rechazado(&texto),
        ErrorRespaldo::VersionNoSoportada {
            archivo: 2,
            soportada: 1
        }
    );
}

#[test]
fn un_encabezado_mal_formado_se_rechaza() {
    let base = texto_valido();
    let casos = [
        base.replace("# cuentas-claras-respaldo", "# otra-aplicacion"),
        base.replace("# cuentas-claras-respaldo ", "# cuentas-claras-respaldo  "),
        base.replace("version=1", "version=01"),
        base.replace("version=1", "version=uno"),
        base.replace("version=1 ", ""),
        base.replace(" eventos=2", ""),
        base.replace(&format!("exportado={AHORA}"), "exportado=ayer"),
        base.replace(&format!("exportado={AHORA}"), "exportado=2026-11-26"),
        base.replace(&format!("exportado={AHORA} "), ""),
        base.replace("eventos=2", "eventos=2 extra=1"),
        base.replace("eventos=2", "eventos=-2"),
        base.replace("eventos=2", "eventos=+2"),
        base.replace("version=1 exportado", "exportado"),
        String::new(),
        "\n".to_string(),
    ];
    for texto in casos {
        assert_eq!(
            rechazado(&texto),
            ErrorRespaldo::ArchivoInvalido,
            "{texto:?}"
        );
    }
}

#[test]
fn una_cantidad_que_no_coincide_se_rechaza() {
    for cantidad in ["eventos=1", "eventos=3", "eventos=0"] {
        let texto = texto_valido().replace("eventos=2", cantidad);
        assert_eq!(
            rechazado(&texto),
            ErrorRespaldo::ArchivoInvalido,
            "{cantidad}"
        );
    }
}

#[test]
fn un_evento_mal_formado_rechaza_el_archivo_completo() {
    let base = texto_valido();
    let primera = base.lines().nth(1).unwrap().to_string();
    let casos = [
        // JSON roto, línea vacía o sin salto final.
        base.replace(&primera, "{\"id\":\"e1\""),
        base.replace(&primera, ""),
        base.trim_end_matches('\n').to_string(),
        format!("{base}\n"),
        base.replace('\n', "\r\n"),
        // Campos que faltan, sobran, se repiten o están vacíos.
        base.replace(&primera, &primera.replace("\"tipo\":\"CuentaCreada\",", "")),
        base.replace(&primera, &primera.replace("{\"id\"", "{\"extra\":1,\"id\"")),
        base.replace(
            &primera,
            &primera.replace("{\"id\":\"e1\"", "{\"id\":\"e1\",\"id\":\"e9\""),
        ),
        base.replace(&primera, &primera.replace("\"id\":\"e1\"", "\"id\":\"\"")),
        base.replace(
            &primera,
            &primera.replace("\"tipo\":\"CuentaCreada\"", "\"tipo\":\"\""),
        ),
        base.replace(&primera, &primera.replace("\"id\":\"e1\"", "\"id\":7")),
        // Datos que no son objeto ni cadena.
        base.replace(
            &primera,
            &primera.replace("\"datos\":{\"a\":1}", "\"datos\":17"),
        ),
        base.replace(
            &primera,
            &primera.replace("\"datos\":{\"a\":1}", "\"datos\":null"),
        ),
        base.replace(
            &primera,
            &primera.replace("\"datos\":{\"a\":1}", "\"datos\":[1]"),
        ),
        // Una línea que no es un objeto.
        base.replace(&primera, "[\"e1\"]"),
    ];
    for texto in casos {
        assert_eq!(
            rechazado(&texto),
            ErrorRespaldo::ArchivoInvalido,
            "{texto:?}"
        );
    }
}

#[test]
fn un_id_repetido_rechaza_el_archivo() {
    let texto = libro_a_texto(&[evento("e1", "{}"), evento("e1", "{}")], AHORA);
    assert_eq!(rechazado(&texto), ErrorRespaldo::ArchivoInvalido);
}

#[test]
fn bytes_que_no_son_utf8_se_rechazan() {
    let mut bytes = texto_valido().into_bytes();
    let n = bytes.len();
    bytes[n - 5] = 0xFF;
    assert_eq!(
        leer_libro(&bytes, None, &CifradorFalso),
        Err(ErrorRespaldo::ArchivoInvalido)
    );
}

#[test]
fn un_archivo_rechazado_no_toca_el_libro() {
    let mut app = libro_completo();
    let antes = app.eventos_del_libro().unwrap();
    let mut texto = libro_a_texto(&[evento("nuevo-1", "{}")], AHORA);
    texto.push_str("{roto}\n");
    let texto = texto.replace("eventos=1", "eventos=2");
    assert_eq!(
        app.importar_copia(texto.as_bytes(), None, &CifradorFalso),
        Err(ErrorRespaldo::ArchivoInvalido)
    );
    assert_eq!(app.eventos_del_libro().unwrap(), antes);
}

#[test]
fn la_fecha_de_exportacion_admite_fraccion_de_segundo() {
    let texto = libro_a_texto(&[], "2026-11-26T10:00:00.123Z");
    assert!(texto_a_libro(&texto).is_ok());
    for mala in [
        "2026-11-26T10:00:00.Z",
        "2026-11-26T10:00:00+00:00",
        "2026-11-26 10:00:00Z",
    ] {
        let texto = format!("# cuentas-claras-respaldo version=1 exportado={mala} eventos=0\n");
        assert_eq!(rechazado(&texto), ErrorRespaldo::ArchivoInvalido, "{mala}");
    }
}
