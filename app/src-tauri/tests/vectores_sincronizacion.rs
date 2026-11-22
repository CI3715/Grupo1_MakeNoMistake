//! Lee `docs/vectores-sincronizacion.json` y comprueba que la división de la app
//! (`dominio::division::dividir`) da las mismas partes que fija el archivo para cada
//! evento de un gasto de grupo, y que los escenarios del libro no se contradicen.
//!
//! El servidor lee el mismo archivo en `servidor/tests/test_vectores_sincronizacion.py`
//! (RF-COM-10, RNF-17, INV-13, INV-15, INV-17, INV-18, ESC-05).
//!
//! Los escenarios los reproduce aquí un modelo de referencia mínimo, solo para
//! comprobar que el archivo es coherente. La prueba del núcleo real (aplicar un lote
//! con el repositorio en memoria y con SQLite) debe recorrer estos mismos escenarios.

use std::collections::BTreeMap;
use std::path::PathBuf;

use dominio::division::{Participante, dividir};
use serde_json::Value;

const CAMPOS_DEL_EVENTO: [&str; 9] = [
    "id",
    "posicion",
    "tipo",
    "grupo",
    "gasto",
    "fecha",
    "moneda",
    "monto",
    "incluye_redondeo",
];

/// Palabras que nunca deben aparecer como clave de lo que viaja al servidor (INV-16).
const CLAVES_DEL_LIBRO: [&str; 5] = ["cuenta", "saldo", "movimiento", "tasa", "categoria"];

fn cargar() -> Value {
    let ruta =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/vectores-sincronizacion.json");
    let texto = std::fs::read_to_string(&ruta)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", ruta.display()));
    serde_json::from_str(&texto).expect("vectores-sincronizacion.json no es un JSON válido")
}

fn texto<'a>(valor: &'a Value, campo: &str) -> &'a str {
    valor[campo]
        .as_str()
        .unwrap_or_else(|| panic!("falta el campo de texto `{campo}`"))
}

fn lista<'a>(valor: &'a Value, campo: &str) -> &'a Vec<Value> {
    valor[campo]
        .as_array()
        .unwrap_or_else(|| panic!("falta la lista `{campo}`"))
}

fn entero(valor: &Value, campo: &str) -> i64 {
    valor[campo]
        .as_i64()
        .unwrap_or_else(|| panic!("falta el entero `{campo}`"))
}

/// Un monto del archivo (texto con punto y exactamente dos decimales) en céntimos.
fn centimos(monto: &str) -> i64 {
    let (parte_entera, decimales) = monto
        .split_once('.')
        .unwrap_or_else(|| panic!("el monto `{monto}` no trae punto decimal"));
    assert_eq!(
        decimales.len(),
        2,
        "el monto `{monto}` no trae dos decimales"
    );
    let negativo = parte_entera.starts_with('-');
    let e: i64 = parte_entera
        .trim_start_matches('-')
        .parse()
        .unwrap_or_else(|_| panic!("el monto `{monto}` no es un número"));
    let d: i64 = decimales
        .parse()
        .unwrap_or_else(|_| panic!("el monto `{monto}` no es un número"));
    let total = e * 100 + d;
    if negativo { -total } else { total }
}

fn claves(valor: &Value, salida: &mut Vec<String>) {
    match valor {
        Value::Object(mapa) => {
            for (clave, hijo) in mapa {
                salida.push(clave.clone());
                claves(hijo, salida);
            }
        }
        Value::Array(items) => {
            for hijo in items {
                claves(hijo, salida);
            }
        }
        _ => {}
    }
}

fn ids_de(valor: &Value, campo: &str) -> Vec<String> {
    lista(valor, campo)
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("`{campo}` debe ser una lista de textos"))
                .to_string()
        })
        .collect()
}

// ---------- el archivo contra el enunciado ----------

#[test]
fn el_archivo_trae_el_ejemplo_y_los_casos_pedidos() {
    let documento = cargar();
    let gastos: Vec<&str> = lista(&documento, "gastos")
        .iter()
        .map(|g| texto(g, "id"))
        .collect();
    for esperado in ["hotel", "cena", "taxi"] {
        assert!(gastos.contains(&esperado), "falta el gasto `{esperado}`");
    }
    let escenarios: Vec<&str> = lista(&documento, "escenarios")
        .iter()
        .map(|e| texto(e, "id"))
        .collect();
    for esperado in [
        "ejemplo-2-6",
        "lote-repetido",
        "tipo-desconocido",
        "parte-sin-cuenta",
        "parte-con-redondeo",
    ] {
        assert!(
            escenarios.contains(&esperado),
            "falta el escenario `{esperado}`"
        );
    }
}

#[test]
fn cada_evento_trae_exactamente_los_campos_acordados() {
    let documento = cargar();
    let mut ids = Vec::new();
    for gasto in lista(&documento, "gastos") {
        for fila in lista(&gasto["esperado"], "eventos") {
            let evento = &fila["evento"];
            let mut presentes: Vec<&str> = evento
                .as_object()
                .expect("el evento debe ser un objeto")
                .keys()
                .map(String::as_str)
                .collect();
            presentes.sort_unstable();
            let mut acordados = CAMPOS_DEL_EVENTO.to_vec();
            acordados.sort_unstable();
            assert_eq!(presentes, acordados, "campos del evento de {fila}");

            assert_eq!(texto(evento, "tipo"), "gasto_de_grupo");
            assert!(uuid::Uuid::parse_str(texto(evento, "id")).is_ok());
            assert!(uuid::Uuid::parse_str(texto(&evento["gasto"], "id")).is_ok());
            assert!(uuid::Uuid::parse_str(texto(&evento["grupo"], "id")).is_ok());
            assert!(entero(evento, "posicion") >= 1);
            assert!(evento["incluye_redondeo"].is_boolean());
            centimos(texto(evento, "monto"));
            ids.push(texto(evento, "id").to_string());
        }
    }
    let total = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), total, "hay ids de evento repetidos");
}

#[test]
fn las_posiciones_son_crecientes_por_usuario_y_solo_reciben_los_participantes() {
    let documento = cargar();
    let miembros = ["ana", "beto", "carla"];
    let mut por_usuario: BTreeMap<String, Vec<i64>> = BTreeMap::new();

    for gasto in lista(&documento, "gastos") {
        let id = texto(gasto, "id");
        let mut participantes: Vec<&str> = lista(gasto, "participantes")
            .iter()
            .map(|p| texto(p, "usuario"))
            .collect();
        participantes.sort_unstable();

        let filas = lista(&gasto["esperado"], "eventos");
        let con_evento: Vec<&str> = filas.iter().map(|f| texto(f, "usuario")).collect();
        assert_eq!(con_evento, participantes, "{id}: quién recibe evento");

        let mut sin_evento: Vec<String> = ids_de(&gasto["esperado"], "sin_evento");
        sin_evento.sort();
        let esperados: Vec<String> = miembros
            .iter()
            .filter(|m| !participantes.contains(*m))
            .map(|m| (*m).to_string())
            .collect();
        assert_eq!(sin_evento, esperados, "{id}: quién no recibe nada");

        for fila in filas {
            por_usuario
                .entry(texto(fila, "usuario").to_string())
                .or_default()
                .push(entero(&fila["evento"], "posicion"));
        }
    }

    for (usuario, posiciones) in por_usuario {
        // Un excluido no consume posición: las que recibe son 1, 2, 3...
        let esperadas: Vec<i64> = (1..=i64::try_from(posiciones.len()).unwrap()).collect();
        assert_eq!(posiciones, esperadas, "posiciones de {usuario}");
    }
}

#[test]
fn nada_del_libro_viaja_al_servidor() {
    // INV-16, RES-02: ni los eventos ni las respuestas llevan datos del libro.
    let documento = cargar();
    let mut enviado = Vec::new();
    for gasto in lista(&documento, "gastos") {
        claves(&gasto["esperado"], &mut enviado);
    }
    for escenario in lista(&documento, "escenarios") {
        for paso in lista(escenario, "pasos") {
            if texto(paso, "accion") == "sincronizar" {
                claves(&paso["respuesta"], &mut enviado);
            }
        }
    }
    assert!(!enviado.is_empty());
    for clave in enviado {
        let minusculas = clave.to_lowercase();
        for palabra in CLAVES_DEL_LIBRO {
            assert!(
                !minusculas.contains(palabra),
                "la clave `{clave}` menciona `{palabra}` y viajaría al servidor"
            );
        }
    }
}

// ---------- la app calcula lo mismo que el archivo ----------

#[test]
fn la_division_de_la_app_coincide_con_los_eventos_del_archivo() {
    let documento = cargar();
    for gasto in lista(&documento, "gastos") {
        let id = texto(gasto, "id");
        let participantes: Vec<Participante> = lista(gasto, "participantes")
            .iter()
            .map(|p| Participante {
                usuario: texto(p, "usuario").to_string(),
                valor: texto(p, "valor").to_string(),
            })
            .collect();
        let division = dividir(texto(gasto, "monto"), texto(gasto, "regla"), &participantes)
            .unwrap_or_else(|e| panic!("{id}: debía dividirse y dio {e:?}"));

        let filas = lista(&gasto["esperado"], "eventos");
        assert_eq!(
            division.partes.len(),
            filas.len(),
            "{id}: cantidad de partes"
        );
        let mut suma = 0;
        for (parte, fila) in division.partes.iter().zip(filas) {
            let evento = &fila["evento"];
            assert_eq!(parte.usuario, texto(fila, "usuario"), "{id}: usuario");
            assert_eq!(
                parte.monto,
                texto(evento, "monto"),
                "{id}: monto de {}",
                parte.usuario
            );
            assert_eq!(
                Some(parte.incluye_redondeo),
                evento["incluye_redondeo"].as_bool(),
                "{id}: marca de redondeo de {}",
                parte.usuario
            );
            suma += centimos(&parte.monto);
        }
        assert_eq!(
            suma,
            centimos(texto(gasto, "monto")),
            "{id}: la suma de las partes"
        );

        for fila in filas {
            let evento = &fila["evento"];
            assert_eq!(texto(&evento["gasto"], "id"), texto(gasto, "gasto_id"));
            assert_eq!(
                texto(&evento["gasto"], "concepto"),
                texto(gasto, "concepto")
            );
            assert_eq!(texto(evento, "fecha"), texto(gasto, "fecha"));
            assert_eq!(texto(evento, "moneda"), texto(gasto, "moneda"));
            assert_eq!(evento["grupo"], gasto["grupo"]);
        }
    }
}

// ---------- los escenarios del libro ----------

/// Modelo de referencia: solo comprueba que los escenarios no se contradicen.
struct Libro {
    saldos: BTreeMap<String, i64>,
    elegida: Option<String>,
    cursor: i64,
    cola: i64,
    movimientos: Vec<Value>,
    pendientes: Vec<Value>,
}

impl Libro {
    fn nuevo(escenario: &Value) -> Self {
        let saldos = lista(escenario, "cuentas")
            .iter()
            .map(|c| {
                (
                    texto(c, "nombre").to_string(),
                    centimos(texto(c, "saldo_inicial")),
                )
            })
            .collect();
        Libro {
            saldos,
            elegida: None,
            cursor: 0,
            cola: 0,
            movimientos: Vec::new(),
            pendientes: Vec::new(),
        }
    }

    fn aplicar(&mut self, evento: &Value) {
        let cuenta = self.elegida.clone().expect("aplicar sin cuenta elegida");
        *self.saldos.get_mut(&cuenta).expect("la cuenta no existe") -=
            centimos(texto(evento, "monto"));
        self.movimientos.push(evento.clone());
    }

    fn conocido(&self, id: &str) -> bool {
        self.movimientos
            .iter()
            .chain(self.pendientes.iter())
            .any(|e| texto(e, "id") == id)
    }

    /// Devuelve (aplicados, ignorados). Un evento ya aplicado, ya pendiente o de un
    /// tipo desconocido se ignora; el cursor queda en el de la respuesta.
    fn sincronizar(&mut self, respuesta: &Value) -> (i64, i64) {
        let (mut aplicados, mut ignorados) = (0, 0);
        for evento in lista(respuesta, "eventos") {
            if texto(evento, "tipo") != "gasto_de_grupo" || self.conocido(texto(evento, "id")) {
                ignorados += 1;
                continue;
            }
            if self.elegida.is_none() {
                self.pendientes.push(evento.clone());
            } else {
                self.aplicar(evento);
            }
            aplicados += 1;
        }
        self.cursor = entero(respuesta, "cursor");
        (aplicados, ignorados)
    }

    fn elegir(&mut self, cuenta: &str) {
        self.elegida = Some(cuenta.to_string());
        let mut pendientes = std::mem::take(&mut self.pendientes);
        pendientes.sort_by_key(|e| entero(e, "posicion"));
        for evento in &pendientes {
            self.aplicar(evento);
        }
    }
}

fn comprobar_estado(libro: &Libro, esperado: &Value, donde: &str) {
    assert_eq!(libro.cursor, entero(esperado, "cursor"), "{donde}: cursor");
    assert_eq!(
        libro.cola,
        entero(esperado, "cola"),
        "{donde}: cola de envío"
    );

    let saldos = esperado["saldos"]
        .as_object()
        .unwrap_or_else(|| panic!("{donde}: faltan los saldos"));
    assert_eq!(
        libro.saldos.len(),
        saldos.len(),
        "{donde}: cantidad de cuentas"
    );
    for (cuenta, monto) in saldos {
        let monto = monto
            .as_str()
            .unwrap_or_else(|| panic!("{donde}: el saldo de {cuenta} no es texto"));
        assert_eq!(
            libro.saldos.get(cuenta).copied(),
            Some(centimos(monto)),
            "{donde}: saldo de {cuenta}"
        );
    }

    let aplicados: Vec<String> = libro
        .movimientos
        .iter()
        .map(|e| texto(e, "id").to_string())
        .collect();
    assert_eq!(
        aplicados,
        ids_de(esperado, "movimientos"),
        "{donde}: movimientos"
    );
    let pendientes: Vec<String> = libro
        .pendientes
        .iter()
        .map(|e| texto(e, "id").to_string())
        .collect();
    assert_eq!(
        pendientes,
        ids_de(esperado, "pendientes"),
        "{donde}: pendientes"
    );
}

#[test]
fn los_escenarios_del_libro_dan_el_estado_esperado() {
    let documento = cargar();
    for escenario in lista(&documento, "escenarios") {
        let id = texto(escenario, "id");
        let mut libro = Libro::nuevo(escenario);

        for paso in lista(escenario, "pasos") {
            let donde = format!("{id} · {}", texto(paso, "id"));
            let esperado = &paso["esperado"];
            match texto(paso, "accion") {
                "elegir_cuenta" => libro.elegir(texto(paso, "cuenta")),
                "sincronizar" => {
                    let respuesta = &paso["respuesta"];
                    let ultima = lista(respuesta, "eventos")
                        .iter()
                        .map(|e| entero(e, "posicion"))
                        .max()
                        .unwrap_or(libro.cursor);
                    assert_eq!(entero(paso, "desde"), libro.cursor, "{donde}: desde");
                    assert_eq!(
                        entero(respuesta, "cursor"),
                        ultima,
                        "{donde}: cursor del lote"
                    );
                    assert_eq!(
                        respuesta["hay_mas"].as_bool(),
                        Some(false),
                        "{donde}: hay_mas"
                    );

                    let (aplicados, ignorados) = libro.sincronizar(respuesta);
                    assert_eq!(
                        aplicados,
                        entero(esperado, "aplicados"),
                        "{donde}: aplicados"
                    );
                    assert_eq!(
                        ignorados,
                        entero(esperado, "ignorados"),
                        "{donde}: ignorados"
                    );
                }
                "reiniciar_cursor" => libro.cursor = 0,
                "encolar_gasto" => libro.cola += 1,
                "enviar_cola" => libro.cola -= 1,
                otra => panic!("{donde}: acción desconocida `{otra}`"),
            }
            comprobar_estado(&libro, esperado, &donde);
        }

        let detalle = lista(escenario, "movimientos_finales");
        assert_eq!(
            detalle.len(),
            libro.movimientos.len(),
            "{id}: movimientos finales"
        );
        for (movimiento, evento) in detalle.iter().zip(&libro.movimientos) {
            let concepto = format!(
                "{} · {}",
                texto(&evento["grupo"], "nombre"),
                texto(&evento["gasto"], "concepto")
            );
            assert_eq!(texto(movimiento, "evento_id"), texto(evento, "id"), "{id}");
            assert_eq!(texto(movimiento, "concepto"), concepto, "{id}");
            assert_eq!(texto(movimiento, "tipo"), "egreso", "{id}");
            assert_eq!(texto(movimiento, "categoria"), "Otros gastos", "{id}");
            assert_eq!(movimiento["de_grupo"].as_bool(), Some(true), "{id}");
            assert_eq!(texto(movimiento, "monto"), texto(evento, "monto"), "{id}");
            assert_eq!(texto(movimiento, "fecha"), texto(evento, "fecha"), "{id}");
        }
    }
}

#[test]
fn el_ejemplo_de_la_seccion_2_6_da_los_saldos_del_enunciado() {
    let documento = cargar();
    let ejemplo = lista(&documento, "escenarios")
        .iter()
        .find(|e| texto(e, "id") == "ejemplo-2-6")
        .expect("falta el escenario ejemplo-2-6");
    let pasos = lista(ejemplo, "pasos");

    let saldos: Vec<&str> = pasos
        .iter()
        .map(|p| texto(&p["esperado"]["saldos"], "Efectivo USD"))
        .collect();
    assert_eq!(
        saldos,
        [
            "200.00", "166.67", "166.67", "166.67", "166.67", "166.67", "151.67", "151.67",
            "151.67"
        ]
    );
    let colas: Vec<i64> = pasos
        .iter()
        .map(|p| entero(&p["esperado"], "cola"))
        .collect();
    assert_eq!(colas, [0, 0, 0, 0, 1, 0, 0, 0, 0]);
    let ultimo = pasos.last().expect("el ejemplo no trae pasos");
    assert_eq!(entero(&ultimo["esperado"], "ignorados"), 2);
}

#[test]
fn solo_se_ofrecen_cuentas_activas_en_la_moneda_del_grupo() {
    // INV-18.
    let documento = cargar();
    for escenario in lista(&documento, "escenarios") {
        let moneda = texto(escenario, "moneda");
        let ofrecidas: Vec<String> = lista(escenario, "cuentas")
            .iter()
            .filter(|c| c["activa"].as_bool() == Some(true) && texto(c, "moneda") == moneda)
            .map(|c| texto(c, "nombre").to_string())
            .collect();
        if escenario.get("cuentas_ofrecidas").is_some() {
            assert_eq!(ids_de(escenario, "cuentas_ofrecidas"), ofrecidas);
        }
        for paso in lista(escenario, "pasos") {
            if texto(paso, "accion") == "elegir_cuenta" {
                assert!(
                    ofrecidas.iter().any(|c| c == texto(paso, "cuenta")),
                    "{}: la cuenta elegida no se ofrece",
                    texto(escenario, "id")
                );
            }
        }
    }
}
