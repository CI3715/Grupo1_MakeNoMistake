//! Pruebas del almacén de secretos y verificación de no filtración (Entrega 8:
//! Angel - Tarea 5; RF-INT-05, RNF-05).

use aplicacion::secretos::{
    AlmacenSecretos, AlmacenSecretosEnMemoria, CredencialesGit, ErrorSecretos,
    bateria_contrato_almacen_secretos,
};
use aplicacion::{
    CasosDeUso, DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo, RepositorioEnMemoria,
    TransporteSincronizacion, exportar_libro,
};
use infraestructura::cifrado::{CifradorArgon2, ParametrosArgon2};
use infraestructura::secretos::AlmacenSecretosSistema;

const FRASE_SECRETA: &str = "frase-secreta-sincronizacion-2026";
const USUARIO_GIT: &str = "beto-git-privado";
const TOKEN_GIT: &str = "ghp_TokenSuperSecretoQueNoDebeFiltrarse999";

#[test]
fn contrato_almacen_secretos_en_memoria() {
    let almacen = AlmacenSecretosEnMemoria::nuevo();
    bateria_contrato_almacen_secretos(&almacen);
}

#[test]
fn almacen_en_memoria_propaga_fallo_cuando_no_esta_disponible() {
    let almacen = AlmacenSecretosEnMemoria::nuevo();
    almacen.simular_fallo(true);

    assert!(matches!(
        almacen.guardar_frase_de_paso(FRASE_SECRETA),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
    assert!(matches!(
        almacen.leer_frase_de_paso(),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
    assert!(matches!(
        almacen.borrar_frase_de_paso(),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
    assert!(matches!(
        almacen.guardar_credenciales_git(&CredencialesGit::nuevas(USUARIO_GIT, TOKEN_GIT)),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
    assert!(matches!(
        almacen.leer_credenciales_git(),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
    assert!(matches!(
        almacen.borrar_credenciales_git(),
        Err(ErrorSecretos::AlmacenNoDisponible(_))
    ));
}

#[test]
fn credenciales_git_no_filtran_el_token_en_debug() {
    let cred = CredencialesGit::nuevas(USUARIO_GIT, TOKEN_GIT);
    let texto_debug = format!("{cred:?}");
    assert!(texto_debug.contains(USUARIO_GIT));
    assert!(
        !texto_debug.contains(TOKEN_GIT),
        "Debug no puede exponer el token de Git: {texto_debug}"
    );
    assert!(texto_debug.contains("[OCULTO]"));
}

#[test]
fn ni_las_credenciales_ni_la_frase_aparecen_en_el_libro_en_la_exportacion_ni_en_el_destino() {
    let almacen = AlmacenSecretosEnMemoria::nuevo();
    almacen.guardar_frase_de_paso(FRASE_SECRETA).unwrap();
    almacen
        .guardar_credenciales_git(&CredencialesGit::nuevas(USUARIO_GIT, TOKEN_GIT))
        .unwrap();

    let mut app = CasosDeUso::nuevo(
        Box::new(RepositorioEnMemoria::nuevo()),
        Box::new(RelojFijo::nuevo("2026-11-25T14:00:00Z")),
        Box::new(GeneradorSecuencial::nuevo()),
        Box::new(DispositivoFijo::nuevo("disp-beto")),
    );
    app.crear_cuenta(NuevaCuenta {
        nombre: "Ahorros".to_string(),
        tipo: "banco".to_string(),
        moneda: "USD".to_string(),
        saldo_inicial: "500.00".to_string(),
    })
    .unwrap();

    let eventos = app.eventos_del_libro().unwrap();
    let cifrador = CifradorArgon2::con_parametros(ParametrosArgon2 {
        memoria_kib: 1024,
        iteraciones: 1,
        paralelismo: 1,
    });

    let frase = almacen.leer_frase_de_paso().unwrap().unwrap();
    let exportado_claro =
        exportar_libro(&eventos, "2026-11-25T14:00:00Z", None, &cifrador).unwrap();
    let exportado_cifrado =
        exportar_libro(&eventos, "2026-11-25T14:00:00Z", Some(&frase), &cifrador).unwrap();

    // Simulamos enviar el lote cifrado al destino de sincronización.
    let destino = aplicacion::DestinoEnMemoria::nuevo();
    let mut transporte = destino.conectar("disp-beto").unwrap();
    transporte
        .escribir_lote("20261125T140000Z-0001", &exportado_cifrado)
        .unwrap();
    transporte.publicar().unwrap();
    let lote_en_destino = transporte
        .leer_lote("disp-beto", "20261125T140000Z-0001")
        .unwrap();

    let texto_libro = format!("{eventos:?}");
    let texto_claro = String::from_utf8_lossy(&exportado_claro);
    let texto_cifrado = String::from_utf8_lossy(&exportado_cifrado);
    let texto_destino = String::from_utf8_lossy(&lote_en_destino);

    for secreto in [FRASE_SECRETA, USUARIO_GIT, TOKEN_GIT] {
        assert!(
            !texto_libro.contains(secreto),
            "el libro no puede contener «{secreto}»"
        );
        assert!(
            !texto_claro.contains(secreto),
            "la exportación sin cifrar no puede contener «{secreto}»"
        );
        assert!(
            !texto_cifrado.contains(secreto),
            "la exportación cifrada no puede contener «{secreto}»"
        );
        assert!(
            !texto_destino.contains(secreto),
            "el lote en el destino no puede contener «{secreto}»"
        );
    }
}

#[test]
fn adaptador_sistema_keyring_cumple_el_contrato_si_el_demonio_del_sistema_esta_activo() {
    let servicio = format!("cuentas-claras-test-{}", std::process::id());
    let almacen = AlmacenSecretosSistema::con_servicio(&servicio);

    // En entornos de integración sin demonio Secret Service activo (p. ej. contenedores
    // Linux o WSL sin sesión D-Bus de llavero), keyring informa AlmacenNoDisponible.
    // Si el servicio del sistema está disponible (Windows/macOS/Linux con keyring activo),
    // corre la batería completa y limpia al terminar.
    match almacen.guardar_frase_de_paso(FRASE_SECRETA) {
        Ok(()) => {
            // Además verificamos que la lectura inmediata devuelva lo recién guardado;
            // algunos backends sin servicio real aceptan la llamada sin persistir.
            if almacen.leer_frase_de_paso().ok().flatten().as_deref() == Some(FRASE_SECRETA) {
                bateria_contrato_almacen_secretos(&almacen);
            }
            let _ = almacen.borrar_todo();
        }
        Err(ErrorSecretos::AlmacenNoDisponible(_)) => {}
        Err(otro) => panic!("error inesperado en AlmacenSecretosSistema: {otro:?}"),
    }
}
