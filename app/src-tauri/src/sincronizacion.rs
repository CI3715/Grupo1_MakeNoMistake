//! Comandos de sincronización (Entrega 7).
//!
//! Este archivo toca el registro personal y no habla con el servidor por su cuenta:
//! recibe un cliente que sabe pedir lo nuevo (`sincronizacion_cliente`) y convierte lo
//! recibido con `sincronizacion_libro`. Un candado impide dos sincronizaciones a la vez
//! y ningún candado del registro se mantiene mientras se espera a la red.
//!
//!   sincronizar                baja lo nuevo, lote por lote, hasta que no hay más
//!   estado_sincronizacion      última sincronización, partes pendientes y cola
//!   cuentas_para_grupo         cuentas activas en la moneda del grupo
//!   elegir_cuenta_grupo        guarda la elección y aplica las partes pendientes
//!   listar_partes_pendientes   partes sin cuenta elegida
//!   reiniciar_cursor           pone el cursor en 0 (para la demostración)

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use aplicacion::{CasosDeUso, ParteDeGrupoEntrante};
use infraestructura::sincronizacion::{GastoEncolado, SincronizacionSqlite};
use tauri::State;

use crate::EstadoCasosDeUso;
use crate::cambio_libro::LibroActual;
use crate::cuentas::bloquear;
use crate::dto::ErroresDto;
use crate::dto_sincronizacion::{
    ConsultaCuentasGrupoDto, CuentaParaGrupoDto, CuentasParaGrupoDto, ElegirCuentaGrupoDto,
    EstadoSincronizacionDto, GastoEnColaDto, ParteGrupoPendienteDto, ResultadoGastoGrupoDto,
};
use crate::dto_social::NuevoGastoGrupoDto;
use crate::sesion::{Almacen, ClienteHttp, ahora_iso, inesperado, sin_campo};
use crate::sincronizacion_cliente::pedir_cambios;
use crate::sincronizacion_libro::{aplicar_pendientes_del_grupo, armar_lote};
use crate::social_cliente::ClienteSocial;
use crate::social_envio::{ResultadoEnvio, enviar_gasto};
use crate::social_real::{almacen_de, cuerpo_de_gasto};

static SINCRONIZANDO: AtomicBool = AtomicBool::new(false);

/// Quien lo tiene es el único que sincroniza; se suelta solo al terminar o fallar.
struct Turno;

impl Turno {
    fn tomar() -> Option<Turno> {
        SINCRONIZANDO
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Turno)
    }
}

impl Drop for Turno {
    fn drop(&mut self) {
        SINCRONIZANDO.store(false, Ordering::SeqCst);
    }
}

fn almacenamiento() -> ErroresDto {
    sin_campo("No se pudo leer o guardar en el almacenamiento del dispositivo. Intenta de nuevo.")
}

fn libro_sync(libro: &LibroActual) -> Result<SincronizacionSqlite, ErroresDto> {
    libro.sincronizacion().map_err(|m| sin_campo(&m))
}

fn a_dto(p: ParteDeGrupoEntrante) -> ParteGrupoPendienteDto {
    ParteGrupoPendienteDto {
        evento_id: p.evento_servidor_id,
        grupo_id: p.grupo_id,
        grupo_nombre: p.grupo_nombre,
        concepto: p.concepto,
        monto: p.monto,
        moneda: p.moneda,
        fecha: p.fecha,
    }
}

fn a_dto_cola(g: GastoEncolado) -> GastoEnColaDto {
    GastoEnColaDto {
        clave: g.clave,
        grupo_id: g.grupo_id,
        grupo_nombre: g.grupo_nombre,
        concepto: g.concepto,
        monto: g.monto,
        moneda: g.moneda,
        fecha: g.fecha,
        estado: g.estado,
        motivo: g.motivo,
    }
}

fn estado_local(
    sync: &SincronizacionSqlite,
    sincronizando: bool,
) -> Result<EstadoSincronizacionDto, ErroresDto> {
    let partes = sync.partes_pendientes().map_err(|_| almacenamiento())?;
    let cola = sync.cola().map_err(|_| almacenamiento())?;
    Ok(EstadoSincronizacionDto {
        ultima_sincronizacion: sync.ultima_sincronizacion().map_err(|_| almacenamiento())?,
        sincronizando,
        partes_pendientes: partes.into_iter().map(a_dto).collect(),
        cola: cola.into_iter().map(a_dto_cola).collect(),
    })
}

/// Cuentas que se pueden elegir para un grupo: las de su moneda, y solo si esa moneda usa
/// dos decimales, que son los que trae el servidor (limitación anotada en la ADR-014).
fn cuentas_elegibles(
    casos: &mut CasosDeUso,
    moneda: &str,
) -> Result<Vec<CuentaParaGrupoDto>, ErroresDto> {
    if moneda.trim().is_empty() {
        return Err(sin_campo("Falta la moneda del grupo."));
    }
    let con_dos_decimales: Vec<String> = casos
        .listar_monedas()
        .map_err(|_| almacenamiento())?
        .into_iter()
        .filter(|m| m.decimales == 2)
        .map(|m| m.codigo)
        .collect();
    if !con_dos_decimales.iter().any(|c| c == moneda) {
        return Ok(Vec::new());
    }
    let lista = casos.listar_cuentas().map_err(|_| almacenamiento())?;
    Ok(lista
        .cuentas
        .into_iter()
        .filter(|c| c.moneda == moneda)
        .map(|c| CuentaParaGrupoDto {
            id: c.id,
            nombre: c.nombre,
            moneda: c.moneda,
        })
        .collect())
}

fn cuentas_del_grupo(
    casos: &Mutex<CasosDeUso>,
    sync: &SincronizacionSqlite,
    grupo_id: &str,
    moneda: &str,
) -> Result<CuentasParaGrupoDto, ErroresDto> {
    let cuentas = cuentas_elegibles(&mut bloquear(casos), moneda)?;
    let elegida = sync
        .cuenta_de_grupo(grupo_id)
        .map_err(|_| almacenamiento())?
        .filter(|id| cuentas.iter().any(|c| &c.id == id));
    Ok(CuentasParaGrupoDto {
        grupo_id: grupo_id.to_string(),
        cuenta_elegida: elegida,
        cuentas,
    })
}

/// Guarda la cuenta del grupo y aplica sus partes pendientes. Solo afecta a las partes que
/// ya estaban pendientes y a las que lleguen después.
fn elegir_en_libro(
    casos: &Mutex<CasosDeUso>,
    sync: &SincronizacionSqlite,
    grupo_id: &str,
    cuenta_id: &str,
    moneda: &str,
) -> Result<CuentasParaGrupoDto, ErroresDto> {
    {
        let mut c = bloquear(casos);
        let elegibles = cuentas_elegibles(&mut c, moneda)?;
        if !elegibles.iter().any(|x| x.id == cuenta_id) {
            return Err(sin_campo("Elige una cuenta activa en la moneda del grupo."));
        }
        sync.elegir_cuenta(grupo_id, cuenta_id)
            .map_err(|_| almacenamiento())?;
        let lote = aplicar_pendientes_del_grupo(&c, sync, grupo_id, cuenta_id)?;
        sync.guardar_lote(&lote).map_err(|_| almacenamiento())?;
    }
    cuentas_del_grupo(casos, sync, grupo_id, moneda)
}

/// Envía los gastos de la cola en orden. Si uno no se puede enviar ahora (sin red, sin
/// sesión), se detiene ahí para no saltarse el orden; los rechazados quedan marcados.
async fn vaciar_cola<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    sync: &SincronizacionSqlite,
) -> Result<(), ErroresDto> {
    let pendientes = sync.cola_pendiente().map_err(|_| almacenamiento())?;
    for g in pendientes {
        match enviar_gasto(cliente, almacen, ahora, &g.grupo_id, &g.cuerpo, &g.clave).await {
            ResultadoEnvio::Enviado(_) => sync.quitar(&g.clave).map_err(|_| almacenamiento())?,
            ResultadoEnvio::Rechazado(motivo) => sync
                .marcar_rechazada(&g.clave, &motivo)
                .map_err(|_| almacenamiento())?,
            ResultadoEnvio::Reintentar => break,
        }
    }
    Ok(())
}

/// Baja lo nuevo hasta que no queden más lotes. Cada lote se guarda junto con su cursor.
pub async fn sincronizar_libro<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    casos: &Mutex<CasosDeUso>,
    sync: &SincronizacionSqlite,
) -> Result<EstadoSincronizacionDto, ErroresDto> {
    vaciar_cola(cliente, almacen, ahora, sync).await?;
    loop {
        let desde = sync.cursor().map_err(|_| almacenamiento())?;
        let pagina = pedir_cambios(cliente, almacen, ahora, desde).await?;
        let avanza = {
            let c = bloquear(casos);
            let lote = armar_lote(&c, sync, &pagina, desde, ahora)?;
            sync.guardar_lote(&lote).map_err(|_| almacenamiento())?;
            lote.cursor.unwrap_or(desde) > desde
        };
        if !pagina.hay_mas {
            break;
        }
        if !avanza {
            // Un servidor que dice «hay más» sin avanzar el cursor nos dejaría en bucle.
            return Err(inesperado());
        }
    }
    estado_local(sync, false)
}

/// Registra un gasto de grupo pasando por la cola local. Si la cola ya tenía gastos
/// pendientes, este espera su turno para no saltarse el orden. Sin conexión queda
/// "pendiente"; si el servidor lo rechaza se quita de la cola y se devuelve el error.
pub async fn registrar_con_cola<C: ClienteSocial, A: Almacen>(
    cliente: &C,
    almacen: &A,
    ahora: &str,
    sync: &SincronizacionSqlite,
    datos: &NuevoGastoGrupoDto,
    grupo_nombre: &str,
    moneda: &str,
) -> Result<ResultadoGastoGrupoDto, ErroresDto> {
    let clave = uuid::Uuid::new_v4().to_string();
    let cuerpo = cuerpo_de_gasto(datos).to_string();
    let hay_cola = !sync
        .cola_pendiente()
        .map_err(|_| almacenamiento())?
        .is_empty();
    sync.encolar(&GastoEncolado {
        clave: clave.clone(),
        grupo_id: datos.grupo_id.clone(),
        grupo_nombre: grupo_nombre.to_string(),
        concepto: datos.concepto.clone(),
        monto: datos.monto.clone(),
        moneda: moneda.to_string(),
        fecha: datos.fecha.clone(),
        cuerpo: cuerpo.clone(),
        estado: "pendiente".to_string(),
        motivo: None,
    })
    .map_err(|_| almacenamiento())?;
    let pendiente = ResultadoGastoGrupoDto {
        estado: "pendiente".to_string(),
        clave: clave.clone(),
        gasto: None,
    };
    if hay_cola {
        return Ok(pendiente);
    }
    match enviar_gasto(cliente, almacen, ahora, &datos.grupo_id, &cuerpo, &clave).await {
        ResultadoEnvio::Enviado(gasto) => {
            sync.quitar(&clave).map_err(|_| almacenamiento())?;
            Ok(ResultadoGastoGrupoDto {
                estado: "registrado".to_string(),
                clave,
                gasto: Some(gasto),
            })
        }
        ResultadoEnvio::Rechazado(motivo) => {
            sync.quitar(&clave).map_err(|_| almacenamiento())?;
            Err(sin_campo(&motivo))
        }
        ResultadoEnvio::Reintentar => Ok(pendiente),
    }
}

// ---------- Comandos de Tauri ----------

#[tauri::command]
pub async fn sincronizar(
    app: tauri::AppHandle,
    estado: State<'_, EstadoCasosDeUso>,
    libro: State<'_, LibroActual>,
) -> Result<EstadoSincronizacionDto, ErroresDto> {
    let sync = libro_sync(&libro)?;
    let Some(_turno) = Turno::tomar() else {
        return estado_local(&sync, true);
    };
    let almacen = almacen_de(&app)?;
    sincronizar_libro(
        &ClienteHttp::desde_entorno(),
        &almacen,
        &ahora_iso(),
        &estado,
        &sync,
    )
    .await
}

#[tauri::command]
pub fn estado_sincronizacion(
    libro: State<'_, LibroActual>,
) -> Result<EstadoSincronizacionDto, ErroresDto> {
    estado_local(&libro_sync(&libro)?, SINCRONIZANDO.load(Ordering::SeqCst))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn registrar_gasto_grupo_con_cola(
    app: tauri::AppHandle,
    libro: State<'_, LibroActual>,
    datos: NuevoGastoGrupoDto,
    grupo_nombre: String,
    moneda: String,
) -> Result<ResultadoGastoGrupoDto, ErroresDto> {
    let sync = libro_sync(&libro)?;
    let almacen = almacen_de(&app)?;
    registrar_con_cola(
        &ClienteHttp::desde_entorno(),
        &almacen,
        &ahora_iso(),
        &sync,
        &datos,
        &grupo_nombre,
        &moneda,
    )
    .await
}

#[tauri::command]
pub fn cuentas_para_grupo(
    datos: ConsultaCuentasGrupoDto,
    estado: State<'_, EstadoCasosDeUso>,
    libro: State<'_, LibroActual>,
) -> Result<CuentasParaGrupoDto, ErroresDto> {
    cuentas_del_grupo(
        &estado,
        &libro_sync(&libro)?,
        &datos.grupo_id,
        &datos.moneda,
    )
}

#[tauri::command]
pub fn elegir_cuenta_grupo(
    datos: ElegirCuentaGrupoDto,
    estado: State<'_, EstadoCasosDeUso>,
    libro: State<'_, LibroActual>,
) -> Result<CuentasParaGrupoDto, ErroresDto> {
    elegir_en_libro(
        &estado,
        &libro_sync(&libro)?,
        &datos.grupo_id,
        &datos.cuenta_id,
        &datos.moneda,
    )
}

#[tauri::command]
pub fn listar_partes_pendientes(
    libro: State<'_, LibroActual>,
) -> Result<Vec<ParteGrupoPendienteDto>, ErroresDto> {
    Ok(estado_local(&libro_sync(&libro)?, false)?.partes_pendientes)
}

#[tauri::command]
pub fn reiniciar_cursor(
    libro: State<'_, LibroActual>,
) -> Result<EstadoSincronizacionDto, ErroresDto> {
    let sync = libro_sync(&libro)?;
    sync.reiniciar_cursor().map_err(|_| almacenamiento())?;
    estado_local(&sync, SINCRONIZANDO.load(Ordering::SeqCst))
}

#[cfg(test)]
#[path = "sincronizacion/pruebas.rs"]
mod pruebas_interrupcion;

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::sesion::{Respuesta, SesionGuardada, SinConexion};
    use crate::social_cliente::Peticion;
    use aplicacion::{DispositivoFijo, GeneradorSecuencial, NuevaCuenta, RelojFijo};
    use infraestructura::db::BaseDatos;
    use infraestructura::repositorios::RepositorioEventosSqlite;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Arc;

    const AHORA: &str = "2026-11-20T12:00:00Z";

    /// `ErroresDto` no implementa `Debug`: en las pruebas se abre con un mensaje fijo.
    fn ok<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(_) => panic!("se esperaba Ok y llegó un error de la aplicación"),
        }
    }

    struct Libro {
        casos: Mutex<CasosDeUso>,
        sync: SincronizacionSqlite,
        ruta: PathBuf,
        _base: BaseDatos,
    }

    impl Drop for Libro {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.ruta);
        }
    }

    fn libro(nombre: &str) -> Libro {
        let ruta =
            std::env::temp_dir().join(format!("cc_comando_{}_{nombre}.db", std::process::id()));
        let _ = std::fs::remove_file(&ruta);
        let base = BaseDatos::persistente(&ruta).unwrap();
        let conn = base.conexion();
        let casos = CasosDeUso::nuevo(
            Box::new(RepositorioEventosSqlite::with_conn(Arc::clone(&conn))),
            Box::new(RelojFijo::nuevo(AHORA)),
            Box::new(GeneradorSecuencial::nuevo()),
            Box::new(DispositivoFijo::nuevo("disp-1")),
        );
        let sync = SincronizacionSqlite::nuevo(conn).unwrap();
        Libro {
            casos: Mutex::new(casos),
            sync,
            ruta,
            _base: base,
        }
    }

    fn cuenta(l: &Libro, moneda: &str) -> String {
        bloquear(&l.casos)
            .crear_cuenta(NuevaCuenta {
                nombre: format!("Efectivo {moneda}"),
                tipo: "efectivo".to_string(),
                moneda: moneda.to_string(),
                saldo_inicial: "200.00".to_string(),
            })
            .unwrap()
            .id
    }

    fn saldo(l: &Libro, cuenta: &str) -> String {
        bloquear(&l.casos)
            .detalle_cuenta(cuenta)
            .unwrap()
            .cuenta
            .saldo_actual
    }

    fn movimientos(l: &Libro, cuenta: &str) -> usize {
        bloquear(&l.casos)
            .detalle_cuenta(cuenta)
            .unwrap()
            .movimientos
            .len()
    }

    /// Cliente falso: responde en orden; sin más respuestas, simula el servidor apagado.
    struct Guion {
        respuestas: Mutex<VecDeque<(u16, String)>>,
        pedidas: Mutex<Vec<String>>,
    }

    fn guion(r: Vec<(u16, String)>) -> Guion {
        Guion {
            respuestas: Mutex::new(r.into()),
            pedidas: Mutex::new(Vec::new()),
        }
    }

    impl ClienteSocial for Guion {
        async fn enviar_peticion(
            &self,
            peticion: &Peticion,
            _token: &str,
        ) -> Result<Respuesta, SinConexion> {
            self.pedidas.lock().unwrap().push(peticion.ruta.clone());
            match self.respuestas.lock().unwrap().pop_front() {
                Some((codigo, cuerpo)) => Ok(Respuesta { codigo, cuerpo }),
                None => Err(SinConexion),
            }
        }
    }

    struct ConSesion;

    impl Almacen for ConSesion {
        fn guardar(&self, _sesion: &SesionGuardada) -> Result<(), String> {
            Ok(())
        }
        fn leer(&self) -> Option<SesionGuardada> {
            Some(SesionGuardada {
                token: "t".to_string(),
                vence_en: "2099-01-01T00:00:00Z".to_string(),
                nombre_visible: "Ana".to_string(),
                usuario_id: "u-1".to_string(),
                nombre_usuario: "ana".to_string(),
                invalidada: false,
            })
        }
        fn borrar(&self) {}
    }

    fn evento(id: &str, posicion: i64, grupo: &str, monto: &str) -> String {
        format!(
            r#"{{"id":"{id}","posicion":{posicion},"tipo":"gasto_de_grupo","grupo":{{"id":"{grupo}","nombre":"Viaje a Mérida"}},"gasto":{{"id":"x-{id}","concepto":"Hotel"}},"fecha":"2026-11-20","moneda":"USD","monto":"{monto}","incluye_redondeo":false}}"#
        )
    }

    fn pagina(eventos: &[String], cursor: i64, hay_mas: bool) -> (u16, String) {
        (
            200,
            format!(
                r#"{{"eventos":[{}],"cursor":{cursor},"hay_mas":{hay_mas}}}"#,
                eventos.join(",")
            ),
        )
    }

    async fn sincronizar_con(l: &Libro, c: &Guion) -> Result<EstadoSincronizacionDto, ErroresDto> {
        sincronizar_libro(c, &ConSesion, AHORA, &l.casos, &l.sync).await
    }

    #[tokio::test]
    async fn sincroniza_varias_paginas_y_guarda_el_cursor() {
        let l = libro("paginas");
        let c = cuenta(&l, "USD");
        l.sync.elegir_cuenta("g1", &c).unwrap();
        let cliente = guion(vec![
            pagina(&[evento("e-100", 1, "g1", "33.33")], 1, true),
            pagina(&[evento("e-101", 2, "g1", "15.00")], 2, false),
        ]);
        let estado = ok(sincronizar_con(&l, &cliente).await);
        assert_eq!(saldo(&l, &c), "151.67");
        assert_eq!(l.sync.cursor().unwrap(), 2);
        assert_eq!(estado.ultima_sincronizacion.as_deref(), Some(AHORA));
        assert!(estado.partes_pendientes.is_empty());
        let pedidas = cliente.pedidas.lock().unwrap().clone();
        assert!(pedidas[0].ends_with("desde=0") && pedidas[1].ends_with("desde=1"));
    }

    #[tokio::test]
    async fn idempotencia_sincronizar_varias_veces_y_con_el_cursor_en_cero_no_duplica() {
        let l = libro("idempotencia");
        let c = cuenta(&l, "USD");
        l.sync.elegir_cuenta("g1", &c).unwrap();
        let misma = || pagina(&[evento("e-100", 1, "g1", "33.33")], 1, false);
        let cliente = guion(vec![misma(), misma(), misma(), misma()]);
        for _ in 0..3 {
            ok(sincronizar_con(&l, &cliente).await);
        }
        assert_eq!(
            (movimientos(&l, &c), saldo(&l, &c)),
            (1, "166.67".to_string())
        );
        l.sync.reiniciar_cursor().unwrap();
        ok(sincronizar_con(&l, &cliente).await);
        assert_eq!(
            (movimientos(&l, &c), saldo(&l, &c)),
            (1, "166.67".to_string())
        );
    }

    #[tokio::test]
    async fn sin_conexion_no_cambia_nada() {
        let l = libro("apagado");
        let c = cuenta(&l, "USD");
        l.sync.elegir_cuenta("g1", &c).unwrap();
        assert!(sincronizar_con(&l, &guion(vec![])).await.is_err());
        assert_eq!(l.sync.cursor().unwrap(), 0);
        assert_eq!(l.sync.ultima_sincronizacion().unwrap(), None);
        assert_eq!(saldo(&l, &c), "200.00");
    }

    #[tokio::test]
    async fn una_sesion_vencida_detiene_la_sincronizacion() {
        let l = libro("vencida");
        let cliente = guion(vec![(401, "{}".to_string())]);
        assert!(sincronizar_con(&l, &cliente).await.is_err());
        assert_eq!(l.sync.cursor().unwrap(), 0);
    }

    #[tokio::test]
    async fn un_servidor_que_dice_hay_mas_sin_avanzar_no_deja_en_bucle() {
        let l = libro("bucle");
        let cliente = guion(vec![pagina(&[], 0, true)]);
        assert!(sincronizar_con(&l, &cliente).await.is_err());
    }

    #[tokio::test]
    async fn elegir_la_cuenta_aplica_las_partes_que_estaban_pendientes() {
        let l = libro("elegir");
        let c = cuenta(&l, "USD");
        let cliente = guion(vec![pagina(&[evento("e-100", 1, "g1", "33.33")], 1, false)]);
        let estado = ok(sincronizar_con(&l, &cliente).await);
        assert_eq!(estado.partes_pendientes.len(), 1);
        assert_eq!(movimientos(&l, &c), 0);

        let dto = ok(elegir_en_libro(&l.casos, &l.sync, "g1", &c, "USD"));
        assert_eq!(dto.cuenta_elegida.as_deref(), Some(c.as_str()));
        assert_eq!(saldo(&l, &c), "166.67");
        assert!(
            ok(estado_local(&l.sync, false))
                .partes_pendientes
                .is_empty()
        );
    }

    #[test]
    fn una_cuenta_de_otra_moneda_no_se_puede_elegir() {
        let l = libro("otra_moneda");
        let euros = cuenta(&l, "EUR");
        assert!(elegir_en_libro(&l.casos, &l.sync, "g1", &euros, "USD").is_err());
        assert_eq!(l.sync.cuenta_de_grupo("g1").unwrap(), None);
    }

    #[test]
    fn solo_se_ofrecen_las_cuentas_de_la_moneda_del_grupo() {
        let l = libro("elegibles");
        cuenta(&l, "USD");
        cuenta(&l, "EUR");
        let dto = ok(cuentas_del_grupo(&l.casos, &l.sync, "g1", "USD"));
        assert_eq!(dto.cuentas.len(), 1);
        assert_eq!(dto.cuentas[0].moneda, "USD");
        assert!(
            cuentas_del_grupo(&l.casos, &l.sync, "g1", "").is_err(),
            "sin moneda debe ser un error"
        );
    }

    #[test]
    fn un_segundo_turno_no_se_toma_mientras_haya_uno() {
        let primero = Turno::tomar();
        assert!(primero.is_some());
        assert!(Turno::tomar().is_none());
        drop(primero);
        assert!(Turno::tomar().is_some());
    }
}
