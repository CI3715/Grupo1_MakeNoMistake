//! Implementaciones en memoria y deterministas de los puertos de salida.
//!
//! Permiten ejecutar los casos de uso y las pruebas automatizadas de forma
//! aislada y reproducible sin depender de sistemas de archivos, bases de datos
//! ni relojes del sistema operativo.

use crate::puertos::{
    DispositivoOrigen, ErrorRepositorio, EventoRegistrado, GeneradorIdentificadores, Reloj,
    RepositorioEventos, ResultadoGuardado,
};

/// Almacén de eventos volátil respaldado por un vector en memoria.
#[derive(Debug, Default)]
pub struct RepositorioEnMemoria {
    eventos: Vec<EventoRegistrado>,
}

impl RepositorioEnMemoria {
    /// Crea un repositorio en memoria inicialmente vacío.
    pub fn nuevo() -> RepositorioEnMemoria {
        RepositorioEnMemoria {
            eventos: Vec::new(),
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
        self.eventos.push(evento.clone());
        Ok(ResultadoGuardado::Guardado)
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        Ok(self.eventos.clone())
    }
}

/// Implementación de [`Reloj`] que devuelve siempre un instante constante.
#[derive(Debug, Clone)]
pub struct RelojFijo {
    momento: String,
}

impl RelojFijo {
    /// Crea un reloj fijo configurado con la marca de tiempo ISO 8601 indicada.
    pub fn nuevo(momento: &str) -> RelojFijo {
        RelojFijo {
            momento: momento.to_string(),
        }
    }
}

impl Reloj for RelojFijo {
    fn ahora_utc(&self) -> String {
        self.momento.clone()
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
