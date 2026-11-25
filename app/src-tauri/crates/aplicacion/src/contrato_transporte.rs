//! Batería de pruebas de contrato para el puerto [`TransporteSincronizacion`]
//! (Entrega 8: Ernesto - Tarea 4; RNF-14).
//!
//! Cualquier adaptador (el falso en memoria, el de carpeta y el de Git) debe superar
//! exactamente las mismas comprobaciones. Para conectar un adaptador nuevo basta con
//! implementar [`BancoDeDestino`] y llamar a [`bateria_contrato_transporte`] desde una
//! prueba:
//!
//! ```ignore
//! #[test]
//! fn el_adaptador_git_cumple_el_contrato() {
//!     bateria_contrato_transporte(&|| -> Box<dyn BancoDeDestino> { Box::new(BancoGit::nuevo()) });
//! }
//! ```
//!
//! Las comprobaciones solo exigen lo que el contrato garantiza: siempre se llama a
//! `publicar` antes de que otro dispositivo haga `traer`, y nunca se exige que un lote
//! sin publicar sea invisible para los demás (una carpeta lo hace visible al instante).

use crate::transporte::{ErrorTransporte, TransporteSincronizacion};

/// Un destino de pruebas del que se pueden obtener transportes para varios dispositivos
/// y al que se le puede cortar el acceso.
pub trait BancoDeDestino {
    /// Transporte de `dispositivo` conectado al mismo destino que los demás.
    fn conectar(&self, dispositivo: &str) -> Box<dyn TransporteSincronizacion>;

    /// Deja el destino inaccesible: sin red, carpeta desmontada o remoto caído.
    fn dejar_inaccesible(&self);

    /// Devuelve el destino a su estado normal. Lo escrito antes sigue ahí.
    fn restablecer(&self);
}

/// Ejecuta todas las comprobaciones, cada una sobre un destino nuevo. Entra en pánico
/// con un mensaje que nombra la comprobación que falló.
pub fn bateria_contrato_transporte(nuevo_banco: &dyn Fn() -> Box<dyn BancoDeDestino>) {
    el_transporte_conoce_su_dispositivo(&*nuevo_banco());
    escribir_y_leer_un_lote(&*nuevo_banco());
    un_lote_vacio_se_conserva(&*nuevo_banco());
    los_lotes_se_listan_en_orden(&*nuevo_banco());
    dos_dispositivos_no_se_pisan(&*nuevo_banco());
    un_lote_escrito_no_se_modifica(&*nuevo_banco());
    publicar_y_traer_son_idempotentes(&*nuevo_banco());
    solo_llegan_los_lotes_nuevos(&*nuevo_banco());
    un_lote_inexistente_se_informa(&*nuevo_banco());
    los_nombres_peligrosos_se_rechazan(&*nuevo_banco());
    un_destino_inaccesible_conserva_lo_pendiente(&*nuevo_banco());
}

/// Publica lo escrito por `emisor` y lo trae `receptor`.
fn publicar_y_traer(
    emisor: &mut dyn TransporteSincronizacion,
    receptor: &mut dyn TransporteSincronizacion,
) {
    emisor
        .publicar()
        .expect("publicar debe funcionar con el destino accesible");
    receptor
        .traer()
        .expect("traer debe funcionar con el destino accesible");
}

fn nombres(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|n| n.to_string()).collect()
}

pub fn el_transporte_conoce_su_dispositivo(banco: &dyn BancoDeDestino) {
    let a = banco.conectar("laptop");
    assert_eq!(
        a.dispositivo_local(),
        "laptop",
        "el_transporte_conoce_su_dispositivo"
    );
}

pub fn escribir_y_leer_un_lote(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    let contenido: Vec<u8> = vec![0, 1, 2, 255, 254, 10, 13, 0, 128];

    a.escribir_lote("lote-0001", &contenido)
        .expect("escribir_y_leer_un_lote: escribir");
    assert_eq!(
        a.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"]),
        "escribir_y_leer_un_lote: el autor ve su lote"
    );
    assert_eq!(
        a.leer_lote("laptop", "lote-0001").unwrap(),
        contenido,
        "escribir_y_leer_un_lote: el autor lee su lote"
    );

    publicar_y_traer(a.as_mut(), b.as_mut());
    assert!(
        b.listar_dispositivos()
            .unwrap()
            .contains(&"laptop".to_string()),
        "escribir_y_leer_un_lote: el otro dispositivo ve al autor"
    );
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"]),
        "escribir_y_leer_un_lote: el otro dispositivo ve el lote"
    );
    assert_eq!(
        b.leer_lote("laptop", "lote-0001").unwrap(),
        contenido,
        "escribir_y_leer_un_lote: los bytes llegan idénticos"
    );
}

pub fn un_lote_vacio_se_conserva(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    a.escribir_lote("lote-vacio", &[])
        .expect("un_lote_vacio_se_conserva: escribir");
    publicar_y_traer(a.as_mut(), b.as_mut());
    assert_eq!(
        b.leer_lote("laptop", "lote-vacio").unwrap(),
        Vec::<u8>::new(),
        "un_lote_vacio_se_conserva"
    );
}

pub fn los_lotes_se_listan_en_orden(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    for lote in ["lote-0003", "lote-0001", "lote-0002"] {
        a.escribir_lote(lote, lote.as_bytes())
            .expect("los_lotes_se_listan_en_orden: escribir");
    }
    publicar_y_traer(a.as_mut(), b.as_mut());
    let esperado = nombres(&["lote-0001", "lote-0002", "lote-0003"]);
    assert_eq!(
        a.listar_lotes("laptop").unwrap(),
        esperado,
        "los_lotes_se_listan_en_orden: autor"
    );
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        esperado,
        "los_lotes_se_listan_en_orden: lector"
    );
}

pub fn dos_dispositivos_no_se_pisan(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    // Mismo nombre de lote en los dos dispositivos: nunca es un choque,
    // porque cada uno escribe solo en su propia carpeta.
    a.escribir_lote("lote-0001", b"de la laptop")
        .expect("dos_dispositivos_no_se_pisan: escribir en a");
    b.escribir_lote("lote-0001", b"del telefono")
        .expect("dos_dispositivos_no_se_pisan: escribir en b");
    a.publicar()
        .expect("dos_dispositivos_no_se_pisan: publicar a");
    b.publicar()
        .expect("dos_dispositivos_no_se_pisan: publicar b");
    a.traer().expect("dos_dispositivos_no_se_pisan: traer a");
    b.traer().expect("dos_dispositivos_no_se_pisan: traer b");

    for lector in [&a, &b] {
        assert_eq!(
            lector.leer_lote("laptop", "lote-0001").unwrap(),
            b"de la laptop".to_vec(),
            "dos_dispositivos_no_se_pisan: lote de la laptop"
        );
        assert_eq!(
            lector.leer_lote("telefono", "lote-0001").unwrap(),
            b"del telefono".to_vec(),
            "dos_dispositivos_no_se_pisan: lote del teléfono"
        );
        assert_eq!(
            lector.listar_dispositivos().unwrap(),
            nombres(&["laptop", "telefono"]),
            "dos_dispositivos_no_se_pisan: dispositivos"
        );
    }
}

pub fn un_lote_escrito_no_se_modifica(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    a.escribir_lote("lote-0001", b"original")
        .expect("un_lote_escrito_no_se_modifica: escribir");

    let antes_de_publicar = a.escribir_lote("lote-0001", b"otro contenido");
    assert!(
        matches!(antes_de_publicar, Err(ErrorTransporte::LoteYaExiste { .. })),
        "un_lote_escrito_no_se_modifica: reescribir antes de publicar debe fallar, fue {antes_de_publicar:?}"
    );

    publicar_y_traer(a.as_mut(), b.as_mut());
    let despues_de_publicar = a.escribir_lote("lote-0001", b"otro contenido");
    assert!(
        matches!(
            despues_de_publicar,
            Err(ErrorTransporte::LoteYaExiste { .. })
        ),
        "un_lote_escrito_no_se_modifica: reescribir después de publicar debe fallar, fue {despues_de_publicar:?}"
    );

    assert_eq!(
        a.leer_lote("laptop", "lote-0001").unwrap(),
        b"original".to_vec(),
        "un_lote_escrito_no_se_modifica: el autor sigue viendo el original"
    );
    assert_eq!(
        b.leer_lote("laptop", "lote-0001").unwrap(),
        b"original".to_vec(),
        "un_lote_escrito_no_se_modifica: el lector sigue viendo el original"
    );
}

pub fn publicar_y_traer_son_idempotentes(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");

    // Con el destino vacío no hay nada que traer ni que publicar.
    a.publicar()
        .expect("publicar_y_traer_son_idempotentes: publicar sin nada");
    b.traer()
        .expect("publicar_y_traer_son_idempotentes: traer sin nada");
    assert!(
        b.listar_dispositivos().unwrap().is_empty(),
        "publicar_y_traer_son_idempotentes: destino vacío"
    );
    assert!(
        b.listar_lotes("laptop").unwrap().is_empty(),
        "publicar_y_traer_son_idempotentes: dispositivo sin lotes"
    );

    a.escribir_lote("lote-0001", b"uno")
        .expect("publicar_y_traer_son_idempotentes: escribir");
    publicar_y_traer(a.as_mut(), b.as_mut());
    publicar_y_traer(a.as_mut(), b.as_mut());
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"]),
        "publicar_y_traer_son_idempotentes: repetir no duplica"
    );
    assert_eq!(
        b.leer_lote("laptop", "lote-0001").unwrap(),
        b"uno".to_vec(),
        "publicar_y_traer_son_idempotentes: repetir no cambia el contenido"
    );
}

pub fn solo_llegan_los_lotes_nuevos(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");

    a.escribir_lote("lote-0001", b"primero")
        .expect("solo_llegan_los_lotes_nuevos: escribir 1");
    publicar_y_traer(a.as_mut(), b.as_mut());
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"]),
        "solo_llegan_los_lotes_nuevos: primera ronda"
    );

    a.escribir_lote("lote-0002", b"segundo")
        .expect("solo_llegan_los_lotes_nuevos: escribir 2");
    publicar_y_traer(a.as_mut(), b.as_mut());
    let lotes = b.listar_lotes("laptop").unwrap();
    assert_eq!(
        lotes,
        nombres(&["lote-0001", "lote-0002"]),
        "solo_llegan_los_lotes_nuevos: segunda ronda"
    );

    // Quien sincroniza sabe que ya leyó el primero y pide solo el que falta.
    let faltantes: Vec<&String> = lotes.iter().filter(|l| l.as_str() != "lote-0001").collect();
    assert_eq!(
        faltantes.len(),
        1,
        "solo_llegan_los_lotes_nuevos: faltantes"
    );
    assert_eq!(
        b.leer_lote("laptop", faltantes[0]).unwrap(),
        b"segundo".to_vec(),
        "solo_llegan_los_lotes_nuevos: el lote nuevo se lee solo"
    );
    assert_eq!(
        b.leer_lote("laptop", "lote-0001").unwrap(),
        b"primero".to_vec(),
        "solo_llegan_los_lotes_nuevos: el lote viejo no cambió"
    );
}

pub fn un_lote_inexistente_se_informa(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    a.escribir_lote("lote-0001", b"uno")
        .expect("un_lote_inexistente_se_informa: escribir");
    publicar_y_traer(a.as_mut(), b.as_mut());

    let lectura = b.leer_lote("laptop", "no-existe");
    assert!(
        matches!(lectura, Err(ErrorTransporte::LoteNoExiste { .. })),
        "un_lote_inexistente_se_informa: lote, fue {lectura:?}"
    );
    let lectura = b.leer_lote("desconocido", "lote-0001");
    assert!(
        matches!(lectura, Err(ErrorTransporte::LoteNoExiste { .. })),
        "un_lote_inexistente_se_informa: dispositivo, fue {lectura:?}"
    );
    assert!(
        b.listar_lotes("desconocido").unwrap().is_empty(),
        "un_lote_inexistente_se_informa: un dispositivo desconocido no tiene lotes"
    );
}

pub fn los_nombres_peligrosos_se_rechazan(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let peligrosos = [
        "",
        "..",
        "../fuera",
        "a/b",
        "a\\b",
        "con espacio",
        "lote.lote",
        "lote.tmp",
        ".oculto",
    ];
    for nombre in peligrosos {
        let escritura = a.escribir_lote(nombre, b"x");
        assert!(
            matches!(escritura, Err(ErrorTransporte::NombreInvalido(_))),
            "los_nombres_peligrosos_se_rechazan: escribir {nombre:?}, fue {escritura:?}"
        );
        let lectura = a.leer_lote("laptop", nombre);
        assert!(
            matches!(lectura, Err(ErrorTransporte::NombreInvalido(_))),
            "los_nombres_peligrosos_se_rechazan: leer lote {nombre:?}, fue {lectura:?}"
        );
        let lectura = a.leer_lote(nombre, "lote-0001");
        assert!(
            matches!(lectura, Err(ErrorTransporte::NombreInvalido(_))),
            "los_nombres_peligrosos_se_rechazan: leer dispositivo {nombre:?}, fue {lectura:?}"
        );
        let listado = a.listar_lotes(nombre);
        assert!(
            matches!(listado, Err(ErrorTransporte::NombreInvalido(_))),
            "los_nombres_peligrosos_se_rechazan: listar {nombre:?}, fue {listado:?}"
        );
    }
    assert!(
        a.listar_lotes("laptop").unwrap().is_empty(),
        "los_nombres_peligrosos_se_rechazan: no debe quedar nada escrito"
    );
}

pub fn un_destino_inaccesible_conserva_lo_pendiente(banco: &dyn BancoDeDestino) {
    let mut a = banco.conectar("laptop");
    let mut b = banco.conectar("telefono");
    let contenido: &[u8] = b"pendiente de enviar";

    banco.dejar_inaccesible();

    // Escribir puede funcionar (copia local, como Git) o fallar (carpeta); en ningún
    // caso puede dejar algo a medias, y el fallo debe ser reintentable.
    match a.escribir_lote("lote-0001", contenido) {
        Ok(()) => {
            let error = a
                .publicar()
                .expect_err("un_destino_inaccesible: publicar debe fallar");
            assert!(
                error.es_reintentable(),
                "un_destino_inaccesible: publicar debe dar un error reintentable, dio {error:?}"
            );
        }
        Err(error) => assert!(
            error.es_reintentable(),
            "un_destino_inaccesible: escribir debe dar un error reintentable, dio {error:?}"
        ),
    }
    let error = b
        .traer()
        .expect_err("un_destino_inaccesible: traer debe fallar");
    assert!(
        error.es_reintentable(),
        "un_destino_inaccesible: traer debe dar un error reintentable, dio {error:?}"
    );

    banco.restablecer();

    // Se reintenta: el lote queda tal cual se escribió, sin duplicados ni restos.
    match a.escribir_lote("lote-0001", contenido) {
        Ok(()) | Err(ErrorTransporte::LoteYaExiste { .. }) => {}
        Err(error) => panic!("un_destino_inaccesible: el reintento de escribir falló: {error:?}"),
    }
    publicar_y_traer(a.as_mut(), b.as_mut());
    assert_eq!(
        b.listar_lotes("laptop").unwrap(),
        nombres(&["lote-0001"]),
        "un_destino_inaccesible: tras restablecer llega un solo lote"
    );
    assert_eq!(
        b.leer_lote("laptop", "lote-0001").unwrap(),
        contenido.to_vec(),
        "un_destino_inaccesible: el contenido llega completo"
    );
}
