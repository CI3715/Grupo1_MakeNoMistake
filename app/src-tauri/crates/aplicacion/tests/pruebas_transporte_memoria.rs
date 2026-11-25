//! Pruebas del puerto `TransporteSincronizacion` y de su adaptador falso en memoria
//! (Entrega 8: Ernesto - Tarea 2 y Tarea 4; RNF-14).
//!
//! La primera prueba pasa al adaptador falso por la misma batería de contrato que el
//! de carpeta y el de Git. El resto comprueba lo propio del falso: que simula varios
//! dispositivos, que se comporta como Git (publicar y traer) y que reproduce los
//! fallos de la sección 2.7 sin perder nada.

use aplicacion::contrato_transporte::{BancoDeDestino, bateria_contrato_transporte};
use aplicacion::{
    DestinoEnMemoria, ErrorTransporte, TransporteEnMemoria, TransporteSincronizacion,
    nombre_de_lote, validar_nombre,
};

struct BancoMemoria {
    destino: DestinoEnMemoria,
}

impl BancoMemoria {
    fn nuevo() -> BancoMemoria {
        BancoMemoria {
            destino: DestinoEnMemoria::nuevo(),
        }
    }
}

impl BancoDeDestino for BancoMemoria {
    fn conectar(&self, dispositivo: &str) -> Box<dyn TransporteSincronizacion> {
        Box::new(self.destino.conectar(dispositivo).unwrap())
    }

    fn dejar_inaccesible(&self) {
        self.destino.dejar_inaccesible();
    }

    fn restablecer(&self) {
        self.destino.restablecer();
    }
}

fn nombres(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|n| n.to_string()).collect()
}

/// Todo lo que un dispositivo ve: (dispositivo, lote, contenido), en orden.
fn vista(t: &dyn TransporteSincronizacion) -> Vec<(String, String, Vec<u8>)> {
    let mut todo = Vec::new();
    for dispositivo in t.listar_dispositivos().unwrap() {
        for lote in t.listar_lotes(&dispositivo).unwrap() {
            let contenido = t.leer_lote(&dispositivo, &lote).unwrap();
            todo.push((dispositivo.clone(), lote, contenido));
        }
    }
    todo
}

/// Una sincronización completa desde el punto de vista del transporte: traer y publicar.
fn sincronizar(t: &mut TransporteEnMemoria) {
    t.traer().unwrap();
    t.publicar().unwrap();
}

// ---------- Contrato común ----------

#[test]
fn el_adaptador_falso_cumple_el_contrato() {
    bateria_contrato_transporte(&|| -> Box<dyn BancoDeDestino> { Box::new(BancoMemoria::nuevo()) });
}

// ---------- Comportamiento propio del adaptador falso ----------

#[test]
fn lo_escrito_no_lo_ven_los_demas_hasta_publicar_y_traer() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();

    a.escribir_lote("lote-0001", b"uno").unwrap();
    b.traer().unwrap();
    assert!(b.listar_lotes("laptop").unwrap().is_empty());
    assert!(destino.lotes_publicados_de("laptop").is_empty());

    a.publicar().unwrap();
    assert_eq!(destino.lotes_publicados_de("laptop"), nombres(&["lote-0001"]));
    // Publicado, pero el teléfono todavía no hizo traer.
    assert!(b.listar_lotes("laptop").unwrap().is_empty());

    b.traer().unwrap();
    assert_eq!(b.listar_lotes("laptop").unwrap(), nombres(&["lote-0001"]));
}

#[test]
fn sin_conexion_no_pierde_lo_pendiente() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();

    a.escribir_lote("lote-0001", b"uno").unwrap();
    a.quedar_sin_conexion();
    assert_eq!(a.publicar(), Err(ErrorTransporte::SinConexion));
    assert_eq!(a.traer(), Err(ErrorTransporte::SinConexion));
    assert_eq!(a.pendientes_de_publicar(), 1);

    // Lo local sigue funcionando sin red.
    a.escribir_lote("lote-0002", b"dos").unwrap();
    assert_eq!(
        a.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001", "lote-0002"])
    );
    assert!(destino.lotes_publicados_de("laptop").is_empty());

    // Al volver la conexión, lo pendiente sale en la siguiente sincronización.
    a.recuperar_conexion();
    a.publicar().unwrap();
    assert_eq!(a.pendientes_de_publicar(), 0);
    b.traer().unwrap();
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001", "lote-0002"])
    );
}

#[test]
fn un_destino_inaccesible_afecta_a_todos_y_es_reintentable() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();
    a.escribir_lote("lote-0001", b"uno").unwrap();

    destino.dejar_inaccesible();
    assert_eq!(a.publicar(), Err(ErrorTransporte::DestinoInaccesible));
    assert_eq!(b.traer(), Err(ErrorTransporte::DestinoInaccesible));
    assert!(ErrorTransporte::DestinoInaccesible.es_reintentable());
    assert_eq!(a.pendientes_de_publicar(), 1);

    destino.restablecer();
    a.publicar().unwrap();
    b.traer().unwrap();
    assert_eq!(b.leer_lote("laptop", "lote-0001").unwrap(), b"uno".to_vec());
}

#[test]
fn las_credenciales_rechazadas_no_son_reintentables_y_se_corrigen() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    a.escribir_lote("lote-0001", b"uno").unwrap();

    destino.rechazar_credenciales(true);
    let error = a.publicar().unwrap_err();
    assert_eq!(error, ErrorTransporte::CredencialesRechazadas);
    assert!(!error.es_reintentable());
    assert!(error.to_string().contains("Actualízalas"));
    assert_eq!(a.pendientes_de_publicar(), 1);

    destino.rechazar_credenciales(false);
    a.publicar().unwrap();
    assert_eq!(destino.lotes_publicados_de("laptop"), nombres(&["lote-0001"]));
}

#[test]
fn si_el_destino_cambia_al_publicar_se_reintenta_y_no_se_pierde_nada() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    a.escribir_lote("lote-0001", b"uno").unwrap();
    destino.hacer_que_el_destino_cambie(2);

    // El paso 4 de la sincronización: si el remoto cambió, volver a traer y reintentar.
    let mut intentos = 0;
    loop {
        intentos += 1;
        assert!(intentos < 10, "el reintento no debe ser infinito");
        match a.publicar() {
            Ok(()) => break,
            Err(ErrorTransporte::DestinoCambio) => a.traer().unwrap(),
            Err(otro) => panic!("error inesperado: {otro:?}"),
        }
    }
    assert_eq!(intentos, 3);
    assert_eq!(destino.publicaciones(), 1);
    assert_eq!(destino.lotes_publicados_de("laptop"), nombres(&["lote-0001"]));
}

#[test]
fn traer_solo_copia_lo_que_falta() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();

    a.escribir_lote("lote-0001", b"uno").unwrap();
    a.publicar().unwrap();
    b.traer().unwrap();
    assert_eq!(b.lotes_traidos(), 1);

    // Sin novedades: nada se vuelve a copiar (RNF-10).
    b.traer().unwrap();
    b.traer().unwrap();
    assert_eq!(b.lotes_traidos(), 1);

    a.escribir_lote("lote-0002", b"dos").unwrap();
    a.publicar().unwrap();
    b.traer().unwrap();
    assert_eq!(b.lotes_traidos(), 2);
}

#[test]
fn las_lecturas_se_cuentan_por_lote() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();
    a.escribir_lote("lote-0001", b"uno").unwrap();
    a.escribir_lote("lote-0002", b"dos").unwrap();
    sincronizar(&mut a);
    b.traer().unwrap();

    // Listar no lee contenido; leer cuenta una lectura por lote.
    let _ = b.listar_lotes("laptop").unwrap();
    assert_eq!(b.lecturas(), 0);
    let _ = b.leer_lote("laptop", "lote-0002").unwrap();
    assert_eq!(b.lecturas(), 1);
}

#[test]
fn un_dispositivo_reconectado_recupera_sus_lotes_publicados() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    a.escribir_lote("lote-0001", b"uno").unwrap();
    a.publicar().unwrap();

    // La aplicación se cierra y se vuelve a abrir: el transporte nuevo empieza vacío.
    let mut a2 = destino.conectar("laptop").unwrap();
    assert!(a2.listar_lotes("laptop").unwrap().is_empty());
    a2.traer().unwrap();
    assert_eq!(a2.listar_lotes("laptop").unwrap(), nombres(&["lote-0001"]));
    assert!(matches!(
        a2.escribir_lote("lote-0001", b"otro"),
        Err(ErrorTransporte::LoteYaExiste { .. })
    ));
}

#[test]
fn publicar_un_lote_que_ya_estaba_publicado_no_publica_ninguno() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    a.escribir_lote("lote-0001", b"original").unwrap();
    a.publicar().unwrap();

    // Un transporte nuevo que no hizo traer no sabe que el lote ya existe.
    let mut a2 = destino.conectar("laptop").unwrap();
    a2.escribir_lote("lote-0001", b"distinto").unwrap();
    a2.escribir_lote("lote-0002", b"nuevo").unwrap();
    assert!(matches!(
        a2.publicar(),
        Err(ErrorTransporte::LoteYaExiste { .. })
    ));
    assert_eq!(destino.lotes_publicados_de("laptop"), nombres(&["lote-0001"]));

    let mut b = destino.conectar("telefono").unwrap();
    b.traer().unwrap();
    assert_eq!(b.leer_lote("laptop", "lote-0001").unwrap(), b"original".to_vec());
}

#[test]
fn el_destino_expone_lo_publicado_para_las_pruebas_de_privacidad() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();
    a.escribir_lote("lote-0001", b"cifrado-a").unwrap();
    b.escribir_lote("lote-0001", b"cifrado-b").unwrap();
    assert!(destino.contenidos_publicados().is_empty());

    a.publicar().unwrap();
    b.publicar().unwrap();
    let mut contenidos = destino.contenidos_publicados();
    contenidos.sort();
    assert_eq!(contenidos, vec![b"cifrado-a".to_vec(), b"cifrado-b".to_vec()]);
}

// ---------- Convergencia a nivel de transporte ----------

#[test]
fn dos_dispositivos_convergen_en_cualquier_orden_de_sincronizacion() {
    // Parten de cero, escriben cosas distintas sin conexión y sincronizan en los dos órdenes.
    let mut finales = Vec::new();
    for laptop_primero in [true, false] {
        let destino = DestinoEnMemoria::nuevo();
        let mut laptop = destino.conectar("laptop").unwrap();
        let mut telefono = destino.conectar("telefono").unwrap();
        laptop.escribir_lote("lote-0001", b"almuerzo y cafe").unwrap();
        telefono.escribir_lote("lote-0001", b"ingreso y gasto").unwrap();

        if laptop_primero {
            sincronizar(&mut laptop);
            sincronizar(&mut telefono);
            sincronizar(&mut laptop);
        } else {
            sincronizar(&mut telefono);
            sincronizar(&mut laptop);
            sincronizar(&mut telefono);
        }
        let vista_laptop = vista(&laptop);
        assert_eq!(vista_laptop, vista(&telefono));
        assert_eq!(vista_laptop.len(), 2);

        // Repetir la sincronización no cambia nada.
        sincronizar(&mut laptop);
        sincronizar(&mut telefono);
        assert_eq!(vista(&laptop), vista_laptop);
        assert_eq!(vista(&telefono), vista_laptop);
        finales.push(vista_laptop);
    }
    assert_eq!(finales[0], finales[1]);
}

#[test]
fn tres_dispositivos_ven_los_lotes_de_los_otros_dos() {
    let destino = DestinoEnMemoria::nuevo();
    let mut a = destino.conectar("laptop").unwrap();
    let mut b = destino.conectar("telefono").unwrap();
    let mut c = destino.conectar("tableta").unwrap();
    a.escribir_lote("lote-0001", b"a").unwrap();
    b.escribir_lote("lote-0001", b"b").unwrap();
    c.escribir_lote("lote-0001", b"c").unwrap();

    for _ in 0..2 {
        sincronizar(&mut a);
        sincronizar(&mut b);
        sincronizar(&mut c);
    }
    let esperado = nombres(&["laptop", "tableta", "telefono"]);
    assert_eq!(a.listar_dispositivos().unwrap(), esperado);
    assert_eq!(b.listar_dispositivos().unwrap(), esperado);
    assert_eq!(c.listar_dispositivos().unwrap(), esperado);
    assert_eq!(vista(&a), vista(&b));
    assert_eq!(vista(&b), vista(&c));
}

// ---------- Nombres ----------

#[test]
fn conectar_rechaza_un_dispositivo_con_nombre_peligroso() {
    let destino = DestinoEnMemoria::nuevo();
    for nombre in ["", "..", "a/b", "con espacio", "../fuera"] {
        assert!(
            matches!(
                destino.conectar(nombre),
                Err(ErrorTransporte::NombreInvalido(_))
            ),
            "debía rechazar {nombre:?}"
        );
    }
}

#[test]
fn los_nombres_validos_son_ascii_sin_separadores() {
    assert!(validar_nombre("laptop").is_ok());
    assert!(validar_nombre("3f2b8c1e-9d4a-4e7b-8a61-2c5d7e9f0a1b").is_ok());
    assert!(validar_nombre("lote_0001-b").is_ok());
    assert!(validar_nombre(&"a".repeat(128)).is_ok());
    assert!(validar_nombre(&"a".repeat(129)).is_err());
    assert!(validar_nombre("ñandú").is_err());
    assert!(validar_nombre("a.b").is_err());
}

#[test]
fn el_nombre_de_lote_se_ordena_como_el_tiempo() {
    let primero = nombre_de_lote("2026-11-24T12:00:00Z", 1);
    let segundo = nombre_de_lote("2026-11-24T12:00:00Z", 2);
    let tercero = nombre_de_lote("2026-11-24T18:30:05Z", 1);
    assert_eq!(primero, "20261124T120000Z-0001");
    assert!(primero < segundo && segundo < tercero);
    for nombre in [&primero, &segundo, &tercero] {
        assert!(validar_nombre(nombre).is_ok());
    }
}
