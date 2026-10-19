//! Puertos: lo que la aplicación necesita del mundo exterior.
//!
//! Cada puerto tiene (al menos) dos implementaciones: una de prueba en
//! `memoria.rs` (este crate) y una real en `infraestructura` (Katherine).
//!
//! Los puertos exigen `Send` porque Tauri guarda los casos de uso dentro de un
//! `Mutex` compartido entre hilos (ese `Arc<Mutex<>>` vive solo en el crate `app`).

use thiserror::Error;

/// Un evento ya "sellado": lo que se guarda tal cual en la tabla `eventos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoRegistrado {
    /// UUID v4 como texto. Identificador único e inmutable (INV-13).
    pub id: String,
    /// Identificador del tipo de evento (`"CuentaCreada"`, `"CategoriaCreada"`,
    /// `"MovimientoRegistrado"`, `"MovimientoCorregido"`, ...).
    pub tipo: String,
    /// ISO 8601 en UTC, por ejemplo "2026-10-12T18:00:00Z".
    pub momento_registro: String,
    /// UUID del dispositivo (se genera la primera vez que se abre la app).
    pub dispositivo_origen: String,
    /// Contenido del evento, en JSON.
    pub datos: String,
}

/// Resultado de intentar guardar un evento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoGuardado {
    /// El evento era nuevo y se guardó.
    Guardado,
    /// Ya había un evento con ese `id`: no se hizo nada (idempotencia, INV-13).
    YaExistia,
}

/// Fallo del almacenamiento. El texto es solo para registros internos:
/// nunca se muestra a la persona usuaria.
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
    /// Código de la moneda (`"VES"`, `"USD"`, ...).
    pub moneda: String,
    /// Saldo con el que se creó la cuenta. Es fijo: nunca cambia (D-11).
    pub saldo_inicial: String,
    /// `saldo_inicial` más el efecto de los movimientos vigentes (D-11).
    pub saldo_actual: String,
    /// Estado de la cuenta (por ahora siempre `"activa"`).
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

/// Puerto del reloj: devuelve el momento actual en ISO 8601 UTC ("2026-10-12T18:00:00Z").
pub trait Reloj: Send {
    fn ahora_utc(&self) -> String;
}

/// Puerto del generador de identificadores: UUID v4 como texto.
pub trait GeneradorIdentificadores: Send {
    fn nuevo_id(&mut self) -> String;
}

/// Puerto del dispositivo de origen: UUID generado la primera vez que se abre la
/// app y guardado en la base. Se reutiliza en todos los eventos.
pub trait DispositivoOrigen: Send {
    fn id_dispositivo(&self) -> String;
}
