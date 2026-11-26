//! Capa de aplicación: casos de uso, puertos de salida y modelos de lectura.
//!
//! Orquesta la lógica del crate `dominio` comunicándose con el exterior únicamente
//! a través de las interfaces (traits) definidas en el módulo [`puertos`].

pub mod casos_de_uso;
pub mod categorias;
pub mod codec;
pub mod contrato;
pub mod contrato_transporte;
pub mod errores;
pub mod fusion;
pub mod memoria;
pub mod proyector;
pub mod puertos;
pub mod respaldo;
pub mod secretos;
pub mod sincronizar_libro;
pub mod transporte;
pub mod transporte_memoria;
pub mod unidades;
pub mod vistas;

pub use casos_de_uso::{
    CasosDeUso, DatosCorregirMovimiento, DatosPrevisualizarMovimiento, NuevaCategoria, NuevaCuenta,
    NuevaMoneda, NuevaTasa, NuevoMovimiento, TIPO_CATEGORIA_CREADA, TIPO_CUENTA_CREADA,
    TIPO_MOVIMIENTO_CORREGIDO, TIPO_MOVIMIENTO_REGISTRADO,
};
pub type NuevaMonedaPropia = NuevaMoneda;

pub use casos_de_uso::{DatosParteDeGrupo, ParteDeGrupoEntrante, ResultadoAplicar};
pub use errores::{ErrorAplicacion, ErrorValidacion};
pub use fusion::{EventoRechazado, InformeFusion, MotivoRechazo, clave_orden, fusionar};
pub use memoria::{
    DispositivoFijo, GeneradorSecuencial, RelojFijo, RelojModificable, RepositorioEnMemoria,
};
pub use proyector::{
    TIPO_MONEDA_BASE_CAMBIADA, TIPO_MONEDA_CREADA, TIPO_TASA_ELIMINADA, TIPO_TASA_REGISTRADA,
};
pub use puertos::{
    CategoriaProyectada, CuentaProyectada, DispositivoOrigen, ErrorRepositorio, EventoRegistrado,
    GeneradorIdentificadores, MonedaProyectada, MovimientoProyectado, Reloj, RepositorioEventos,
    ResultadoGuardado, TasaProyectada,
};
pub use respaldo::{
    Cifrador, ErrorRespaldo, LARGO_MINIMO_FRASE, exportar_libro, leer_libro, libro_a_texto,
};
pub use secretos::{
    AlmacenSecretos, AlmacenSecretosEnMemoria, CredencialesGit, ErrorSecretos,
    bateria_contrato_almacen_secretos, validar_credenciales_git, validar_frase_de_paso,
};
pub use transporte::{
    ErrorTransporte, LARGO_MAXIMO_NOMBRE, LoteDeDispositivo, TransporteSincronizacion,
    lotes_por_leer, nombre_de_lote, validar_nombre,
};
pub use transporte_memoria::{DestinoEnMemoria, TransporteEnMemoria};
pub use vistas::{
    CategoriaListada, CategoriaResumen, ComposicionVista, CuentaListada, DetalleCuentaVista,
    ListaCuentas, MonedaDto, MovimientoListado, ParFaltanteVista, PatrimonioVista,
    PrevisualizacionMovimiento, Subtotal, TasaListada, TasaUsadaVista,
};
