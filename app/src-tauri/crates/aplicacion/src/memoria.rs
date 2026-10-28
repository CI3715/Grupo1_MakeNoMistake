//! Implementaciones en memoria y deterministas de los puertos de salida.
//!
//! Permiten ejecutar los casos de uso y las pruebas automatizadas de forma
//! aislada y reproducible sin depender de sistemas de archivos, bases de datos
//! ni relojes del sistema operativo.

use crate::proyector;
use crate::puertos::{
    CategoriaProyectada, CuentaProyectada, DispositivoOrigen, ErrorRepositorio, EventoRegistrado,
    GeneradorIdentificadores, MonedaProyectada, MovimientoProyectado, Reloj, RepositorioEventos,
    ResultadoGuardado, TasaProyectada,
};

/// Convierte un error de la capa de aplicación en un error de repositorio.
fn error_repo(e: crate::errores::ErrorAplicacion) -> ErrorRepositorio {
    ErrorRepositorio(e.to_string())
}

/// Almacén de eventos volátil respaldado por vectores en memoria.
///
/// Mantiene el libro de eventos y sus proyecciones (`cuentas`, `categorias`,
/// `movimientos`, `monedas_propias`, `tasas` y moneda base) sincronizados igual
/// que el adaptador de SQLite: al guardar un evento nuevo, las proyecciones se
/// actualizan acto seguido.
#[derive(Debug, Default)]
pub struct RepositorioEnMemoria {
    eventos: Vec<EventoRegistrado>,
    cuentas: Vec<CuentaProyectada>,
    categorias: Vec<CategoriaProyectada>,
    movimientos: Vec<MovimientoProyectado>,
    monedas: Vec<MonedaProyectada>,
    tasas: Vec<TasaProyectada>,
    moneda_base: Option<String>,
}

impl RepositorioEnMemoria {
    /// Crea un repositorio en memoria inicialmente vacío.
    pub fn nuevo() -> RepositorioEnMemoria {
        RepositorioEnMemoria::default()
    }

    /// Aplica los efectos de un evento sobre las proyecciones, en el orden en que
    /// las comparte con el adaptador de SQLite.
    ///
    /// Si el evento se rechaza, ninguna proyección cambia: todas las
    /// comprobaciones ocurren antes de escribir.
    fn aplicar(&mut self, evento: &EventoRegistrado) -> Result<(), ErrorRepositorio> {
        if let Some(moneda) = proyector::proyectar_moneda(evento).map_err(error_repo)? {
            if self.monedas.iter().any(|m| m.codigo == moneda.codigo) {
                return Err(ErrorRepositorio(format!(
                    "ya existe la moneda {}",
                    moneda.codigo
                )));
            }
            self.monedas.push(moneda);
        }

        let catalogo = proyector::catalogo_completo(&self.monedas);

        if let Some(cuenta) = proyector::proyectar_cuenta(evento, &catalogo).map_err(error_repo)? {
            self.cuentas.push(cuenta);
        }

        if let Some(tasa) = proyector::proyectar_tasa(evento, &catalogo).map_err(error_repo)? {
            if self.tasas.iter().any(|t| t.id == tasa.id) {
                return Err(ErrorRepositorio(format!("ya existe la tasa {}", tasa.id)));
            }
            self.tasas.push(tasa);
        }

        if let Some(tasa_id) = proyector::tasa_eliminada(evento).map_err(error_repo)? {
            let Some(tasa) = self
                .tasas
                .iter_mut()
                .find(|t| t.id == tasa_id && t.estado == "vigente")
            else {
                return Err(ErrorRepositorio(format!(
                    "la tasa {tasa_id} no existe o ya fue eliminada"
                )));
            };
            tasa.estado = "eliminada".to_string();
        }

        if let Some(moneda) =
            proyector::moneda_base_cambiada(evento, &catalogo).map_err(error_repo)?
        {
            self.moneda_base = Some(moneda);
        }

        if let Some(categoria) =
            proyector::proyectar_categoria(evento).map_err(|e| ErrorRepositorio(e.to_string()))?
            && !self.categorias.iter().any(|c| c.id == categoria.id)
        {
            self.categorias.push(categoria);
        }

        let mov_proyectado = match proyector::proyectar_movimiento(evento)
            .map_err(|e| ErrorRepositorio(e.to_string()))?
        {
            Some(m) => Some(m),
            None => {
                proyector::proyectar_correccion_con_original(evento, &self.movimientos, &catalogo)
                    .map_err(error_repo)?
            }
        };

        if let Some(movimiento) = mov_proyectado {
            if !self.cuentas.iter().any(|c| c.id == movimiento.cuenta_id) {
                return Err(ErrorRepositorio(format!(
                    "la cuenta {} no existe en la proyección",
                    movimiento.cuenta_id
                )));
            }
            if !self
                .categorias
                .iter()
                .any(|c| c.id == movimiento.categoria_id)
                && !dominio::categorias_de_fabrica()
                    .iter()
                    .any(|c| c.id == movimiento.categoria_id)
            {
                return Err(ErrorRepositorio(format!(
                    "la categoría {} no existe en la proyección",
                    movimiento.categoria_id
                )));
            }
            if let Some(original) = movimiento.reemplaza_a.clone() {
                for fila in self
                    .movimientos
                    .iter_mut()
                    .filter(|m| m.id == original || m.reemplaza_a.as_deref() == Some(&original))
                {
                    if fila.estado == "vigente" {
                        fila.estado = "corregido".to_string();
                        fila.corregido_por_evento = Some(evento.id.clone());
                    }
                }
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

        let catalogo = proyector::catalogo_completo(&self.monedas);
        let Some(cuenta) = self.cuentas.iter_mut().find(|c| c.id == cuenta_id) else {
            return Ok(());
        };
        let decimales = proyector::decimales_de(&cuenta.moneda, &catalogo).map_err(error_repo)?;
        cuenta.saldo_actual =
            proyector::calcular_saldo_actual(&cuenta.saldo_inicial, decimales, &vigentes)
                .map_err(error_repo)?;
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

    fn listar_monedas_propias(&self) -> Result<Vec<MonedaProyectada>, ErrorRepositorio> {
        Ok(self.monedas.clone())
    }

    fn listar_tasas(&self) -> Result<Vec<TasaProyectada>, ErrorRepositorio> {
        Ok(self.tasas.clone())
    }

    fn moneda_base(&self) -> Result<Option<String>, ErrorRepositorio> {
        Ok(self.moneda_base.clone())
    }

    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio> {
        self.cuentas.clear();
        self.categorias.clear();
        self.movimientos.clear();
        self.monedas.clear();
        self.tasas.clear();
        self.moneda_base = None;

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
