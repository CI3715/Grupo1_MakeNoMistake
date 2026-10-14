use crate::db::{insertar_evento, listar_eventos};
use aplicacion::puertos::{
    ErrorRepositorio, EventoRegistrado, RepositorioEventos, ResultadoGuardado,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Implementación persistente de RepositorioEventos con SQLite.
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
        let mut conn = self.conn.lock().map_err(|e| ErrorRepositorio(e.to_string()))?;
        match insertar_evento(
            &mut conn,
            &evento.id,
            &evento.tipo,
            &evento.momento_registro,
            &evento.dispositivo_origen,
            &evento.datos,
        ) {
            Ok(true) => Ok(ResultadoGuardado::Guardado),
            Ok(false) => Ok(ResultadoGuardado::YaExistia),
            Err(e) => Err(ErrorRepositorio(e.to_string())),
        }
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        let conn = self.conn.lock().map_err(|e| ErrorRepositorio(e.to_string()))?;
        match listar_eventos(&conn) {
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
}
