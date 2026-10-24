//! Definición de los puertos de salida (interfaces) que la capa de aplicación
//! requiere para interactuar con servicios externos como almacenamiento, reloj
//! y generación de identificadores.
//!
//! Todos los traits exigen el bound `Send` para permitir su uso seguro en
//! contextos concurrentes protegidos por exclusión mutua (`Mutex`).

use thiserror::Error;

/// Representación de un evento de dominio serializado y listo para persistirse
/// en el almacén de eventos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoRegistrado {
    /// Identificador único e inmutable del evento en formato UUID v4.
    pub id: String,
    /// Identificador del tipo de evento (`"CuentaCreada"`, `"CategoriaCreada"`,
    /// `"MovimientoRegistrado"`, `"MovimientoCorregido"`, ...).
    pub tipo: String,
    /// Fecha y hora de creación en formato ISO 8601 UTC (`"YYYY-MM-DDTHH:MM:SSZ"`).
    pub momento_registro: String,
    /// Identificador único del dispositivo donde se originó el evento.
    pub dispositivo_origen: String,
    /// Carga útil del evento serializada en formato JSON.
    pub datos: String,
}

/// Resultado de la operación de escritura de un evento en el repositorio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoGuardado {
    /// El evento no existía previamente y fue almacenado con éxito.
    Guardado,
    /// Ya existía un evento registrado con el mismo `id`; se omitió la escritura para mantener idempotencia.
    YaExistia,
}

/// Error de lectura o escritura originado en el adaptador de persistencia.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct ErrorRepositorio(pub String);

/// Fila de la proyección materializada `cuentas` (modelo de lectura).
///
/// Es una caché derivada del libro de eventos: se puede borrar y reconstruir por
/// completo en cualquier momento. Nunca es la fuente de verdad. Los montos viajan
/// como texto ya formateado con la escala de su moneda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuentaProyectada {
    /// Identificador de la cuenta (coincide con el del evento `CuentaCreada`).
    pub id: String,
    /// Nombre único de la cuenta.
    pub nombre: String,
    /// Tipo en texto (`"efectivo"`, `"banco"`, `"billetera_digital"`).
    pub tipo: String,
    /// Código de la moneda (`"VES"`, `"USD"`, `"EUR"`, `"USDT"`).
    pub moneda: String,
    /// Saldo con el que se creó la cuenta. Es fijo: nunca cambia (D-11).
    pub saldo_inicial: String,
    /// `saldo_inicial` más el efecto de los movimientos vigentes (D-11).
    pub saldo_actual: String,
    /// Estado de la cuenta (por defecto `"activa"`).
    pub estado: String,
}

/// Fila de la proyección materializada `categorias` (modelo de lectura).
///
/// Recibe las categorías de fábrica (`es_fabrica`) y las creadas por quien usa
/// la aplicación. También es una caché derivada del libro de eventos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriaProyectada {
    /// Identificador de la categoría.
    pub id: String,
    /// Nombre visible (único dentro de su tipo).
    pub nombre: String,
    /// `"ingreso"` o `"egreso"`.
    pub tipo: String,
    /// `true` para las once categorías que trae el sistema.
    pub es_fabrica: bool,
    /// Estado (`"activa"` por defecto; `"inactiva"` al desactivarla).
    pub estado: String,
}

/// Fila de la proyección materializada `movimientos` (modelo de lectura).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoProyectado {
    /// Identificador del movimiento.
    pub id: String,
    /// Cuenta a la que pertenece.
    pub cuenta_id: String,
    /// `"ingreso"` o `"egreso"`.
    pub tipo: String,
    /// Monto positivo formateado con la escala de la moneda de la cuenta.
    pub monto: String,
    /// Fecha en formato `AAAA-MM-DD`.
    pub fecha: String,
    /// Categoría que clasifica el movimiento.
    pub categoria_id: String,
    /// Nota opcional.
    pub nota: Option<String>,
    /// `"vigente"` o `"corregido"`.
    pub estado: String,
    /// Si este movimiento es una corrección, apunta al movimiento original.
    pub reemplaza_a: Option<String>,
    /// Identificador del evento que corrigió este movimiento original.
    pub corregido_por_evento: Option<String>,
}

/// Puerto de persistencia para el libro inmutable de eventos y sus proyecciones.
///
/// Las implementaciones deben garantizar:
/// 1. `listar` devuelve los eventos en estricto orden de inserción.
/// 2. `guardar` es idempotente respecto al campo `id` y devuelve [`ResultadoGuardado::YaExistia`] si el identificador se repite.
/// 3. Los eventos almacenados son inmutables y su contenido no se sobrescribe.
/// 4. Al guardar un evento nuevo, las proyecciones `cuentas`, `categorias` y
///    `movimientos` se actualizan en la **misma transacción** (atomicidad): o se
///    persiste el evento y sus efectos, o ninguno.
/// 5. `listar_cuentas`, `listar_categorias` y `listar_movimientos` son simples
///    lecturas de las proyecciones (no reproyectan eventos).
/// 6. `reconstruir` borra las tres proyecciones y las recalcula desde el libro de
///    eventos; el resultado debe ser idéntico al que se obtendría tras un `guardar`
///    normal.
///
/// Además, `saldo_inicial` de una cuenta es inmutable y `saldo_actual` siempre
/// coincide con `saldo_inicial` más el efecto de los movimientos vigentes (D-11).
pub trait RepositorioEventos: Send {
    /// Almacena un evento en el repositorio si su identificador no ha sido registrado antes.
    fn guardar(&mut self, evento: &EventoRegistrado)
    -> Result<ResultadoGuardado, ErrorRepositorio>;

    /// Eventos en estricto orden de inserción (fuente de verdad).
    fn listar(&self) -> Result<Vec<EventoRegistrado>, ErrorRepositorio>;

    /// Lectura de la proyección `cuentas`, en orden de creación.
    fn listar_cuentas(&self) -> Result<Vec<CuentaProyectada>, ErrorRepositorio>;

    /// Lectura de la proyección `categorias`, en orden de creación.
    fn listar_categorias(&self) -> Result<Vec<CategoriaProyectada>, ErrorRepositorio>;

    /// Lectura de la proyección `movimientos`, en orden de registro.
    fn listar_movimientos(&self) -> Result<Vec<MovimientoProyectado>, ErrorRepositorio>;

    /// Reconstruye las proyecciones desde cero releyendo el libro de eventos.
    fn reconstruir(&mut self) -> Result<(), ErrorRepositorio>;
}

/// Proveedor de fecha y hora del sistema.
pub trait Reloj: Send {
    /// Devuelve el instante actual formateado en ISO 8601 UTC (`"2026-10-12T18:00:00Z"`).
    fn ahora_utc(&self) -> String;

    /// Devuelve la fecha actual según la hora local del dispositivo en formato `"AAAA-MM-DD"`.
    fn hoy(&self) -> String {
        let utc = self.ahora_utc();
        if utc.len() >= 10 {
            utc[..10].to_string()
        } else {
            utc
        }
    }
}

/// Proveedor de identificadores únicos para entidades y eventos.
pub trait GeneradorIdentificadores: Send {
    /// Genera y devuelve un nuevo identificador único como cadena de texto.
    fn nuevo_id(&mut self) -> String;
}

/// Proveedor del identificador único de la instalación o dispositivo local.
pub trait DispositivoOrigen: Send {
    /// Devuelve el identificador persistente asociado al dispositivo actual.
    fn id_dispositivo(&self) -> String;
}
