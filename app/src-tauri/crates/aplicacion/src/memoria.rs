//! Implementaciones en memoria y deterministas de los puertos de salida.
//!
//! Permiten ejecutar los casos de uso y las pruebas automatizadas de forma
//! aislada y reproducible sin depender de sistemas de archivos, bases de datos
//! ni relojes del sistema operativo.

use crate::proyector;
use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, DispositivoOrigen, ErrorRepositorio, EventoRegistrado,
    GeneradorIdentificadores, MovimientoProyectado, Reloj, RepositorioEventos, ResultadoGuardado,
};

/// Almacén de eventos volátil respaldado por vectores en memoria.
///
/// Mantiene el libro de eventos y sus proyecciones `cuentas`, `categorias` y
/// `movimientos` sincronizados igual que el adaptador de SQLite: al guardar un
/// evento nuevo, las proyecciones se actualizan acto seguido.
#[derive(Debug, Default)]
pub struct RepositorioEnMemoria {
    eventos: Vec<EventoRegistrado>,
    cuentas: Vec<CuentaProyectada>,
    categorias: Vec<CategoriaProyectada>,
    movimientos: Vec<MovimientoProyectado>,
}

impl RepositorioEnMemoria {
    /// Crea un repositorio en memoria inicialmente vacío.
    pub fn nuevo() -> RepositorioEnMemoria {
        RepositorioEnMemoria {
            eventos: Vec::new(),
            cuentas: Vec::new(),
            categorias: Vec::new(),
            movimientos: Vec::new(),
        }
    }

    /// Aplica los efectos de un evento sobre las proyecciones, en el orden en que
    /// las comparte con el adaptador de SQLite.
    fn aplicar(&mut self, evento: &EventoRegistrado) -> Result<(), ErrorRepositorio> {
        if let Some(cuenta) =
            proyector::proyectar_cuenta(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
        {
            self.cuentas.push(cuenta);
        }

        if let Some(categoria) =
            proyector::proyectar_categoria(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
        {
            self.categorias.push(categoria);
        }

        if let Some(movimiento) =
            proyector::proyectar_movimiento(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
        {
            if let Some(original) = movimiento.reemplaza_a.clone()
                && let Some(fila) = self.movimientos.iter_mut().find(|m| m.id == original)
            {
                fila.estado = "corregido".to_string();
                fila.corregido_por_evento = Some(evento.id.clone());
            }
            let cuenta_id = movimiento.cuenta_id.clone();
            self.movimientos.push(movimiento);
            self.recalcular_saldo(&cuenta_id)?;
        }

        Ok(())
    }

    /// Recalcula `saldo_actual` de una cuenta a partir de su saldo inicial y sus
    /// movimientos vigentes, sin tocar `saldo_inicial` (D-11).
    fn recalcular_saldo(&mut self, cuenta_id: &str) -> Result<(), ErrorRepositorio> {
        let vigentes: Vec<(String, String)> = self
            .movimientos
            .iter()
            .filter(|m| m.cuenta_id == cuenta_id && m.estado == "vigente")
            .map(|m| (m.tipo.clone(), m.monto.clone()))
            .collect();

        let Some(cuenta) = self.cuentas.iter_mut().find(|c| c.id == cuenta_id) else {
            return Ok(());
        };
        cuenta.saldo_actual =
            proyector::calcular_saldo_actual(&cuenta.saldo_inicial, &cuenta.moneda, &vigentes)
                .map_err(|e| ErrorRepositorio(e.to_string()))?;
        Ok(())
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
        self.aplicar(evento)?;
        self.eventos.push(evento.clone());
        Ok(ResultadoGuardado::Guardado)
    }

    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio> {
        Ok(self.eventos.clone())
    }

    fn listar_cuentas(&self) -> Result<Vec<CuentaProyectada>, ErrorRepositorio> {
        Ok(self.cuentas.clone())
    }

    fn listar_categorias(&self) -> Result<Vec<CategoriaProyectada>, ErrorRepositorio> {
        Ok(self.categorias.clone())
    }

    fn listar_movimientos(&self) -> Result<Vec<MovimientoProyectado>, ErrorRepositorio> {
        Ok(self.movimientos.clone())
    }

    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio> {
        self.cuentas.clear();
        self.categorias.clear();
        self.movimientos.clear();

        let eventos = self.eventos.clone();
        for evento in &eventos {
            self.aplicar(evento)?;
        }
        Ok(())
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
