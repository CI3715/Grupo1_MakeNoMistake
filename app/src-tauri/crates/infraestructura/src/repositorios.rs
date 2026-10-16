use crate::db::{self, insertar_evento};
use aplicacion::puertos::{
    CuentaProyectada, ErrorRepositorio, EventoRegistrado, RepositorioEventos, ResultadoGuardado,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Implementación persistente de [`RepositorioEventos`] con SQLite.
///
/// Guarda el libro de eventos y su proyección `cuentas` en la misma transacción
/// para conservar la atomicidad. `listar_cuentas` lee directamente la proyección
/// y `reconstruir` vuelve a materializarla desde los eventos.
pub struct RepositorioEventosSqlite {
    conn: Arc<Mutex<Connection>>,
}

impl RepositorioEventosSqlite {
    pub fn nuevo(conn: Connection) -> Self {
        RepositorioEventosSqlite {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn with_conn(conn: Arc<Mutex<Connection>>) -> Self {
        RepositorioEventosSqlite { conn }
    }
}

impl RepositorioEventos for RepositorioEventosSqlite {
    fn guardar(
        &mut self,
        evento: &EventoRegistrado,
    ) -> Result<ResultadoGuardado, ErrorRepositorio> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;

        match insertar_evento(
            &tx,
            &evento.id,
            &evento.tipo,
            &evento.momento_registro,
            &evento.dispositivo_origen,
            &evento.datos,
        ) {
            Ok(false) => {
                tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
                return Ok(ResultadoGuardado::YaExistia);
            }
            Ok(true) => {}
            Err(e) => return Err(ErrorRepositorio(e.to_string())),
        }

        if let Some(cuenta) = aplicacion::proyector::proyectar_cuenta(evento)
            .map_err(|e| ErrorRepositorio(e.to_string()))?
        {
            let fila = db::CuentaRow {
                id: cuenta.id,
                nombre: cuenta.nombre,
                tipo: cuenta.tipo,
                moneda: cuenta.moneda,
                saldo_inicial: cuenta.saldo_actual.clone(),
                saldo_actual: cuenta.saldo_actual,
                estado: cuenta.estado,
                evento_origen: evento.id.clone(),
                actualizado_en: evento.momento_registro.clone(),
            };
            db::insertar_cuenta(&tx, &fila).map_err(|e| ErrorRepositorio(e.to_string()))?;
        }

        tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(ResultadoGuardado::Guardado)
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_eventos(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| EventoRegistrado {
                    id: r.id,
                    tipo: r.tipo,
                    momento_registro: r.momento_registro,
                    dispositivo_origen: r.dispositivo_origen,
                    datos: r.datos,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn listar_cuentas(&self) -> Result<Vec<CuentaProyectada>, ErrorRepositorio> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        match db::listar_cuentas(&conn) {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| CuentaProyectada {
                    id: r.id,
                    nombre: r.nombre,
                    tipo: r.tipo,
                    moneda: r.moneda,
                    saldo_actual: r.saldo_actual,
                    estado: r.estado,
                })
                .collect()),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| ErrorRepositorio(e.to_string()))?;

        db::borrar_cuentas(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;

        let eventos = db::listar_eventos(&tx).map_err(|e| ErrorRepositorio(e.to_string()))?;
        for row in &eventos {
            let evento = EventoRegistrado {
                id: row.id.clone(),
                tipo: row.tipo.clone(),
                momento_registro: row.momento_registro.clone(),
                dispositivo_origen: row.dispositivo_origen.clone(),
                datos: row.datos.clone(),
            };
            if let Some(cuenta) = aplicacion::proyector::proyectar_cuenta(&evento)
                .map_err(|e| ErrorRepositorio(e.to_string()))?
            {
                let fila = db::CuentaRow {
                    id: cuenta.id,
                    nombre: cuenta.nombre,
                    tipo: cuenta.tipo,
                    moneda: cuenta.moneda,
                    saldo_inicial: cuenta.saldo_actual.clone(),
                    saldo_actual: cuenta.saldo_actual,
                    estado: cuenta.estado,
                    evento_origen: evento.id.clone(),
                    actualizado_en: evento.momento_registro.clone(),
                };
                db::insertar_cuenta(&tx, &fila).map_err(|e| ErrorRepositorio(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use aplicacion::contrato::bateria_contrato_repositorio;

    fn repositorio_en_memoria_sqlite() -> RepositorioEventosSqlite {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::inicializar(&conn).unwrap();
        RepositorioEventosSqlite::nuevo(conn)
    }

    #[test]
    fn contrato_sqlite_pasa_bateria_completa() {
        bateria_contrato_repositorio(repositorio_en_memoria_sqlite);
    }

    #[test]
    fn las_cuentas_persisten_al_cerrar_y_reabrir_el_archivo() {
        use crate::db::BaseDatos;
        use aplicacion::contrato::{datos_cuenta, evento_de_tipo};

        let ruta =
            std::env::temp_dir().join(format!("cuentas_claras_{}.sqlite", uuid::Uuid::new_v4()));

        {
            let bd = BaseDatos::persistente(&ruta).expect("abrir base");
            let mut repo = RepositorioEventosSqlite::with_conn(bd.conexion());
            let evento = evento_de_tipo("e1", "CuentaCreada", &datos_cuenta("c1", "Cartera"));
            assert_eq!(repo.guardar(&evento).unwrap(), ResultadoGuardado::Guardado);
        }

        {
            let bd = BaseDatos::persistente(&ruta).expect("reabrir base");
            let repo = RepositorioEventosSqlite::with_conn(bd.conexion());
            let cuentas = repo.listar_cuentas().unwrap();
            assert_eq!(cuentas.len(), 1);
            assert_eq!(cuentas[0].nombre, "Cartera");
            assert_eq!(repo.listar().unwrap().len(), 1);
        }

        for sufijo in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", ruta.display(), sufijo));
        }
    }
}
