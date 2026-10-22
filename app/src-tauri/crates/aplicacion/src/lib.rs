//! Capa de aplicación: casos de uso, puertos de salida y modelos de lectura.
//!
//! Orquesta la lógica del crate `dominio` comunicándose con el exterior únicamente
//! a través de las interfaces (traits) definidas en el módulo [`puertos`].

pub mod casos_de_uso;
pub mod codec;
pub mod contrato;
pub mod errores;
pub mod memoria;
pub mod proyector;
pub mod puertos;
pub mod vistas;

pub use casos_de_uso::{CasosDeUso, NuevaCuenta, TIPO_CUENTA_CREADA};
pub use errores::{ErrorAplicacion, ErrorValidacion};
pub use memoria::{DispositivoFijo, GeneradorSecuencial, RelojFijo, RepositorioEnMemoria};
pub use puertos::{
    CuentaProyectada, DispositivoOrigen, ErrorRepositorio, EventoRegistrado,
    GeneradorIdentificadores, Reloj, RepositorioEventos, ResultadoGuardado,
};
pub use vistas::{CuentaListada, ListaCuentas, MonedaDto, Subtotal};
