//! Implementaciones en memoria y deterministas de los puertos de salida.
//!
//! Permiten ejecutar los casos de uso y las pruebas automatizadas de forma
//! aislada y reproducible sin depender de sistemas de archivos, bases de datos
//! ni relojes del sistema operativo.

use crate::proyector;
use crate::puertos::{
    CuentaProyectada, DispositivoOrigen, ErrorRepositorio, EventoRegistrado,
    GeneradorIdentificadores, Reloj, RepositorioEventos, ResultadoGuardado,
};

/// Almacén de eventos volátil respaldado por vectores en memoria.
///
/// Mantiene el libro de eventos y su proyección `cuentas` sincronizados:
/// al guardar un evento nuevo, la proyección se actualiza acto seguido.
#[derive(Debug, Default)]
pub struct RepositorioEnMemoria {
    eventos: Vec<EventoRegistrado>,
    cuentas: Vec<CuentaProyectada>,
}

impl RepositorioEnMemoria {
    /// Crea un repositorio en memoria inicialmente vacío.
    pub fn nuevo() -> RepositorioEnMemoria {
        RepositorioEnMemoria {
            eventos: Vec::new(),
            cuentas: Vec::new(),
        }
    }
}

impl RepositorioEventos for RepositorioEnMemoria {
    fn guardar(
        &mut self,
        evento: &EventoRegistrado,
    ) -> Result<ResultadoGuardado, ErrorRepositorio> {
        if self.eventos.iter().any(|e| e.id == evento.id) {
            return Ok(ResultadoGuardado::YaExistia);
        }
        // Valida inmediatamente eventos CuentaCreada antes de insertar.
        let _ = proyector::proyectar_cuenta(evento).map_err(|e| ErrorRepositorio(e.to_string()))?;

        self.eventos.push(evento.clone());
        self.cuentas = proyector::proyectar_todas_las_cuentas(&self.eventos)
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(ResultadoGuardado::Guardado)
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        Ok(self.eventos.clone())
    }

    fn listar_cuentas(&self) -> Result<Vec<CuentaProyectada>, ErrorRepositorio> {
        Ok(self.cuentas.clone())
    }

    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio> {
        self.cuentas = proyector::proyectar_todas_las_cuentas(&self.eventos)
            .map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(())
    }
}

/// Implementación de [`Reloj`] que devuelve siempre un instante constante.
#[derive(Debug, Clone)]
pub struct RelojFijo {
    momento: String,
    fecha_local: String,
}

impl RelojFijo {
    /// Crea un reloj fijo configurado con la marca de tiempo ISO 8601 indicada.
    pub fn nuevo(momento: &str) -> RelojFijo {
        let fecha_local = if momento.len() >= 10 {
            momento[..10].to_string()
        } else {
            "2026-10-19".to_string()
        };
        RelojFijo {
            momento: momento.to_string(),
            fecha_local,
        }
    }

    /// Crea un reloj fijo especificando tanto el instante UTC como la fecha local (`"AAAA-MM-DD"`).
    pub fn con_fecha_local(momento: &str, fecha_local: &str) -> RelojFijo {
        RelojFijo {
            momento: momento.to_string(),
            fecha_local: fecha_local.to_string(),
        }
    }
}

impl Reloj for RelojFijo {
    fn ahora_utc(&self) -> String {
        self.momento.clone()
    }

    fn hoy(&self) -> String {
        self.fecha_local.clone()
    }
}

/// Generador determinista que produce identificadores secuenciales con formato UUID v4
/// (`00000000-0000-4000-8000-000000000001`, `...0002`, etc.).
#[derive(Debug, Default)]
pub struct GeneradorSecuencial {
    siguiente: u64,
}

impl GeneradorSecuencial {
    /// Crea un generador secuencial inicializado en `1`.
    pub fn nuevo() -> GeneradorSecuencial {
        GeneradorSecuencial { siguiente: 1 }
    }
}

impl GeneradorIdentificadores for GeneradorSecuencial {
    fn nuevo_id(&mut self) -> String {
        let n = if self.siguiente == 0 {
            1
        } else {
            self.siguiente
        };
        self.siguiente = n + 1;
        format!("00000000-0000-4000-8000-{n:012}")
    }
}

/// Implementación de [`DispositivoOrigen`] que devuelve un identificador estático.
#[derive(Debug, Clone)]
pub struct DispositivoFijo {
    id: String,
}

impl DispositivoFijo {
    /// Crea un proveedor de dispositivo con el identificador especificado.
    pub fn nuevo(id: &str) -> DispositivoFijo {
        DispositivoFijo { id: id.to_string() }
    }
}

impl DispositivoOrigen for DispositivoFijo {
    fn id_dispositivo(&self) -> String {
        self.id.clone()
    }
}
