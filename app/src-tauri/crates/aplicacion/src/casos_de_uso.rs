//! Casos de uso de la aplicaciÃ³n: gestiÃ³n de cuentas, categorÃ­as, registro y
//! correcciÃ³n de movimientos, previsualizaciÃ³n de saldo y consulta de detalles.

use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use dominio::{
    Ajuste, Campo, Categoria, Cuenta, CuentaCreada, DECIMALES_MAXIMOS, ErrorDominio,
    EstadoMovimiento, Moneda, Monto, Movimiento, MovimientoCorregido, MovimientoRegistrado,
    TipoCuenta, TipoMovimiento, calcular_saldo_y_composicion, categorias_de_fabrica,
    corregir_movimiento, crear_categoria, crear_moneda, registrar_movimiento_con_monedas,
    saldo_resultante,
};

use crate::categorias::id_categoria_canonico;
use crate::codec;
use crate::errores::{ErrorAplicacion, ErrorValidacion};
use crate::proyector::{self, TIPO_MONEDA_BASE_CAMBIADA, TIPO_MONEDA_CREADA, TIPO_TASA_ELIMINADA, TIPO_TASA_REGISTRADA};
use crate::puertos::{
    DispositivoOrigen, EventoRegistrado, GeneradorIdentificadores, Reloj, RepositorioEventos,
};
use crate::vistas::{
    CategoriaListada, CategoriaResumen, ComposicionVista, CuentaListada, CuentaPatrimonioVista,
    DetalleCuentaVista, ListaCuentas, MonedaDto, MovimientoListado, ParFaltanteVista,
    PatrimonioVista, PrevisualizacionMovimiento, Subtotal, TasaListada, TasaUsadaVista,
};

/// Identificador de tipo para los eventos de creaciÃ³n de cuenta.
pub const TIPO_CUENTA_CREADA: &str = "CuentaCreada";
/// Identificador de tipo para los eventos de registro de movimiento.
pub const TIPO_MOVIMIENTO_REGISTRADO: &str = "MovimientoRegistrado";
/// Identificador de tipo para los eventos de correcciÃ³n de movimiento (ajuste).
pub const TIPO_MOVIMIENTO_CORREGIDO: &str = "MovimientoCorregido";
/// Identificador de tipo para los eventos de creaciÃ³n de categorÃ­a personalizada.
pub const TIPO_CATEGORIA_CREADA: &str = "CategoriaCreada";

/// Datos de entrada requeridos para registrar una nueva cuenta.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaCuenta {
    /// Nombre de la cuenta (obligatorio y Ãºnico sin distinguir mayÃºsculas).
    pub nombre: String,
    /// Tipo de cuenta (`"efectivo"`, `"banco"` o `"billetera_digital"`).
    pub tipo: String,
    /// CÃ³digo de una moneda del catÃ¡logo (de fÃ¡brica o propia).
    pub moneda: String,
    /// Saldo inicial expresado como cadena decimal (por ejemplo, `"1250.50"`).
    pub saldo_inicial: String,
}

/// Datos de entrada requeridos para registrar una tasa de cambio (RF-CAM-02).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaTasa {
    /// Moneda de origen del par (ej. `"USD"`).
    pub moneda_origen: String,
    /// Moneda de destino del par (ej. `"VES"`).
    pub moneda_destino: String,
    /// Valor decimal de la tasa como texto (ej. `"887.00"`).
    pub valor: String,
    /// Fecha de vigencia en formato AAAA-MM-DD.
    pub fecha: String,
    /// Fuente de la cotización (ej. `"BCV"`).
    pub fuente: String,
}

/// Datos de entrada para crear una moneda propia. Los decimales llegan como
/// texto, tal como los escribiÃ³ la persona, y los valida el dominio.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaMoneda {
    /// CÃ³digo de 3 a 5 letras mayÃºsculas (por ejemplo, `"BTC"`).
    pub codigo: String,
    /// Nombre visible.
    pub nombre: String,
    /// SÃ­mbolo con el que se muestran los montos.
    pub simbolo: String,
    /// NÃºmero de decimales, de 0 a 8.
    pub decimales: String,
}

/// Datos de entrada requeridos para registrar un movimiento (ingreso o gasto).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevoMovimiento {
    /// Identificador de la cuenta donde se asienta el movimiento.
    pub cuenta_id: String,
    /// Tipo de movimiento (`"ingreso"` o `"gasto"`).
    pub tipo: String,
    /// Monto positivo expresado como texto con punto o coma decimal.
    pub monto: String,
    /// Fecha del movimiento en formato `"AAAA-MM-DD"`.
    pub fecha: String,
    /// Identificador o nombre de la categorÃ­a asociada.
    pub categoria_id: String,
    /// Nota descriptiva opcional.
    pub nota: Option<String>,
}

/// Datos de entrada requeridos para corregir un movimiento existente mediante un ajuste.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct DatosCorregirMovimiento {
    /// Identificador del movimiento original que se desea corregir.
    pub movimiento_id: String,
    /// Nuevo monto correcto expresado como texto decimal (`"0"` o `"0.00"` anula el movimiento).
    pub monto_correcto: String,
}

/// Datos de entrada para previsualizar el efecto de un movimiento sobre el saldo de una cuenta.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct DatosPrevisualizarMovimiento {
    /// Identificador de la cuenta afectada.
    pub cuenta_id: String,
    /// Tipo de movimiento (`"ingreso"` o `"gasto"`).
    pub tipo: String,
    /// Monto expresado como texto decimal.
    pub monto: String,
}

/// Datos de entrada requeridos para crear una nueva categorÃ­a de usuario.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaCategoria {
    /// Nombre de la categorÃ­a (Ãºnico dentro de su mismo tipo sin distinguir mayÃºsculas).
    pub nombre: String,
    /// Tipo de movimiento al que pertenece (`"ingreso"` o `"gasto"`).
    pub tipo: String,
}

/// Registro interno de un movimiento original proyectado junto con su orden de inserciÃ³n.
#[derive(Debug, Clone)]
struct MovimientoProyectado {
    orden: usize,
    movimiento: Movimiento,
}

/// Registro interno de un ajuste proyectado junto con su fecha y orden de inserciÃ³n.
#[derive(Debug, Clone)]
struct AjusteProyectado {
    orden: usize,
    fecha: String,
    ajuste: Ajuste,
}

/// Registro interno de una cuenta con su fecha de creación para respetar la fecha de corte en patrimonio.
#[derive(Debug, Clone)]
struct CuentaEnLibro {
    cuenta: CuentaCreada,
    fecha_creacion: String,
}

impl std::ops::Deref for CuentaEnLibro {
    type Target = CuentaCreada;
    fn deref(&self) -> &Self::Target {
        &self.cuenta
    }
}

/// Estado completo proyectado en memoria a partir de la secuencia de eventos del libro.
struct EstadoLibro {
    /// Catálogo de monedas: fábrica más las propias creadas en el libro.
    monedas: Vec<Moneda>,
    cuentas: Vec<CuentaEnLibro>,
    categorias: Vec<Categoria>,
    movimientos: Vec<MovimientoProyectado>,
    ajustes: Vec<AjusteProyectado>,
    tasas: Vec<dominio::Tasa>,
    moneda_base: String,
}

pub struct CasosDeUso {
    repositorio: Box<dyn RepositorioEventos>,
    reloj: Box<dyn Reloj>,
    ids: Box<dyn GeneradorIdentificadores>,
    dispositivo: Box<dyn DispositivoOrigen>,
}

impl CasosDeUso {
    /// Inicializa el servicio de casos de uso con los adaptadores proporcionados.
    pub fn nuevo(
        repositorio: Box<dyn RepositorioEventos>,
        reloj: Box<dyn Reloj>,
        ids: Box<dyn GeneradorIdentificadores>,
        dispositivo: Box<dyn DispositivoOrigen>,
    ) -> CasosDeUso {
        CasosDeUso {
            repositorio,
            reloj,
            ids,
            dispositivo,
        }
    }

    /// CatÃ¡logo completo de monedas: las cuatro de fÃ¡brica mÃ¡s las propias.
    fn catalogo(&self) -> Result<Vec<Moneda>, ErrorAplicacion> {
        Ok(proyector::catalogo_completo(
            &self.repositorio.listar_monedas_propias()?,
        ))
    }

    /// Devuelve el catÃ¡logo de monedas: las de fÃ¡brica y, despuÃ©s, las propias
    /// en orden de creaciÃ³n.
    pub fn listar_monedas(&self) -> Result<Vec<MonedaDto>, ErrorAplicacion> {
        Ok(self.catalogo()?.into_iter().map(moneda_a_dto).collect())
    }

    /// Valida y crea una moneda propia (RF-CAM-01) registrando el evento
    /// `MonedaCreada`. Desde ese momento puede usarse en una cuenta nueva.
    pub fn crear_moneda(&mut self, entrada: NuevaMoneda) -> Result<MonedaDto, ErrorAplicacion> {
        // Los decimales llegan como texto: si no son un nÃºmero entero, el
        // dominio valida el resto con 0 y se aÃ±ade el error del campo.
        let texto_decimales = entrada.decimales.trim();
        let decimales = texto_decimales.parse::<u32>();
        let resultado = crear_moneda(
            &entrada.codigo,
            &entrada.nombre,
            &entrada.simbolo,
            *decimales.as_ref().unwrap_or(&0),
            &self.catalogo()?,
        );
        let error_decimales = decimales.is_err().then(|| ErrorValidacion {
            campo: Some(Campo::Decimales.a_texto().to_string()),
            mensaje: if texto_decimales.is_empty() {
                "Indica cuÃ¡ntos decimales usa la moneda.".to_string()
            } else {
                format!("Los decimales deben ser un nÃºmero entero de 0 a {DECIMALES_MAXIMOS}.")
            },
        });
        let (moneda, evento_dominio) = match (resultado, error_decimales) {
            (Ok(creada), None) => creada,
            (resultado, error_decimales) => {
                let mut errores = resultado.err().map(convertir_errores).unwrap_or_default();
                errores.extend(error_decimales);
                return Err(ErrorAplicacion::Validacion(errores));
            }
        };

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_MONEDA_CREADA.to_string(),
            momento_registro: self.reloj.ahora_utc(),
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar_moneda_creada(&evento_dominio),
        };
        self.repositorio.guardar(&evento)?;
        Ok(moneda_a_dto(moneda))
    }

    /// Reconstruye el listado de cuentas con su saldo actual derivado de todos sus
    /// movimientos y ajustes (INV-05), y calcula los subtotales separados por moneda (INV-02).
    pub fn listar_cuentas(&self) -> Result<ListaCuentas, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let mut cuentas = Vec::new();
        let mut saldos_por_cuenta = Vec::new();

        for c in &estado.cuentas {
            let comp = calcular_composicion_de_cuenta(c, &estado)?;
            let saldo_actual_dec = comp.saldo_actual.cantidad();
            cuentas.push(CuentaListada {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                tipo: codec::tipo_a_texto(&c.tipo).to_string(),
                moneda: c.moneda.codigo.clone(),
                saldo_actual: formatear_monto(saldo_actual_dec, c.moneda.decimales),
            });
            saldos_por_cuenta.push((
                c.moneda.codigo.clone(),
                c.moneda.decimales,
                saldo_actual_dec,
            ));
        }

        let subtotales = calcular_subtotales(&saldos_por_cuenta);
        Ok(ListaCuentas {
            cuentas,
            subtotales,
        })
    }

    /// Valida los datos de entrada con una Ãºnica invocaciÃ³n a `Cuenta::crear` (D-10)
    /// y, si son vÃ¡lidos, sella y persiste el evento `CuentaCreada`.
    pub fn crear_cuenta(&mut self, entrada: NuevaCuenta) -> Result<CuentaListada, ErrorAplicacion> {
        let proyectadas = self.repositorio.listar_cuentas()?;
        let nombres_existentes: Vec<String> =
            proyectadas.iter().map(|c| c.nombre.clone()).collect();

        let tipo_resultado = codec::texto_a_tipo(&entrada.tipo);
        let tipo_para_validar = tipo_resultado.clone().unwrap_or(TipoCuenta::Efectivo);
        let saldo_normalizado = normalizar_separador_decimal(&entrada.saldo_inicial);

        // Una sola llamada a Cuenta::crear (D-10).
        let resultado_dominio = Cuenta::crear_con_monedas(
            String::new(),
            &entrada.nombre,
            tipo_para_validar,
            &entrada.moneda,
            &saldo_normalizado,
            &nombres_existentes,
            &self.catalogo()?,
        );

        match (resultado_dominio, tipo_resultado) {
            (Ok((_, mut evento_dominio)), Ok(_)) => {
                evento_dominio.id = self.ids.nuevo_id();
                let evento = EventoRegistrado {
                    id: self.ids.nuevo_id(),
                    tipo: TIPO_CUENTA_CREADA.to_string(),
                    momento_registro: self.reloj.ahora_utc(),
                    dispositivo_origen: self.dispositivo.id_dispositivo(),
                    datos: codec::serializar(&evento_dominio),
                };
                self.repositorio.guardar(&evento)?;

                Ok(CuentaListada {
                    id: evento_dominio.id.clone(),
                    nombre: evento_dominio.nombre.clone(),
                    tipo: codec::tipo_a_texto(&evento_dominio.tipo).to_string(),
                    moneda: evento_dominio.moneda.codigo.clone(),
                    saldo_actual: formatear_monto(
                        evento_dominio.saldo_inicial,
                        evento_dominio.moneda.decimales,
                    ),
                })
            }
            (res_dom, res_tipo) => {
                let mut errores_validacion = match res_dom {
                    Ok(_) => Vec::new(),
                    Err(errs) => convertir_errores(errs),
                };
                if res_tipo.is_err() {
                    let pos = errores_validacion
                        .iter()
                        .position(|e| e.campo.as_deref() != Some("nombre"))
                        .unwrap_or(errores_validacion.len());
                    errores_validacion.insert(
                        pos,
                        ErrorValidacion {
                            campo: Some("tipo".to_string()),
                            mensaje: "El tipo de cuenta no es vÃ¡lido.".to_string(),
                        },
                    );
                }
                Err(ErrorAplicacion::Validacion(errores_validacion))
            }
        }
    }

    /// Lista todas las categorÃ­as disponibles (las 11 de fÃ¡brica mÃ¡s las creadas por el usuario).
    pub fn listar_categorias(&self) -> Result<Vec<CategoriaListada>, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        Ok(estado
            .categorias
            .into_iter()
            .map(|c| CategoriaListada {
                id: c.id,
                nombre: c.nombre,
                tipo: codec::tipo_movimiento_a_texto(&c.tipo).to_string(),
                de_fabrica: c.de_fabrica,
            })
            .collect())
    }

    /// Crea una categorÃ­a personalizada validando unicidad por tipo (sin distinguir mayÃºsculas)
    /// y persiste el evento `CategoriaCreada`.
    pub fn crear_categoria(
        &mut self,
        entrada: NuevaCategoria,
    ) -> Result<CategoriaListada, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let tipo_res = codec::texto_a_tipo_movimiento(&entrada.tipo);

        match tipo_res {
            Ok(tipo) => {
                let (mut cat, mut ev) =
                    crear_categoria(String::new(), &entrada.nombre, tipo, &estado.categorias)
                        .map_err(|errs| ErrorAplicacion::Validacion(convertir_errores(errs)))?;

                let id_cat = self.ids.nuevo_id();
                cat.id = id_cat.clone();
                ev.id_categoria = id_cat;

                let evento = EventoRegistrado {
                    id: self.ids.nuevo_id(),
                    tipo: TIPO_CATEGORIA_CREADA.to_string(),
                    momento_registro: self.reloj.ahora_utc(),
                    dispositivo_origen: self.dispositivo.id_dispositivo(),
                    datos: codec::serializar_categoria(&ev),
                };
                self.repositorio.guardar(&evento)?;

                Ok(CategoriaListada {
                    id: cat.id,
                    nombre: cat.nombre,
                    tipo: codec::tipo_movimiento_a_texto(&cat.tipo).to_string(),
                    de_fabrica: cat.de_fabrica,
                })
            }
            Err(_) => {
                let mut errs = Vec::new();
                if entrada.nombre.trim().is_empty() {
                    errs.push(ErrorValidacion {
                        campo: Some("nombre".to_string()),
                        mensaje: "El nombre de la categorÃ­a no puede estar vacÃ­o".to_string(),
                    });
                }
                errs.push(ErrorValidacion {
                    campo: Some("tipo".to_string()),
                    mensaje: "El tipo de movimiento debe ser 'ingreso' o 'gasto'".to_string(),
                });
                Err(ErrorAplicacion::Validacion(errs))
            }
        }
    }

    /// Registra un nuevo movimiento (ingreso o gasto) en una cuenta existente,
    /// validando todas las reglas de negocio contra el dominio y la fecha local del reloj.
    pub fn registrar_movimiento(
        &mut self,
        entrada: NuevoMovimiento,
    ) -> Result<MovimientoListado, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;

        let cuenta_ev = estado
            .cuentas
            .iter()
            .find(|c| c.id == entrada.cuenta_id.trim());

        let moneda_codigo = match cuenta_ev {
            Some(c) => c.moneda.codigo.clone(),
            None => "USD".to_string(),
        };

        let tipo_res = codec::texto_a_tipo_movimiento(&entrada.tipo);
        let tipo_val = tipo_res.clone().unwrap_or(TipoMovimiento::Gasto);

        let id_cat_resuelto = resolver_id_categoria(&entrada.categoria_id, &estado.categorias);
        let monto_normalizado = normalizar_separador_decimal(&entrada.monto);
        let fecha_usar = if entrada.fecha.trim().is_empty() {
            self.reloj.hoy()
        } else {
            entrada.fecha.trim().to_string()
        };
        let hoy = self.reloj.hoy();

        let res_dominio = registrar_movimiento_con_monedas(
            String::new(),
            entrada.cuenta_id.clone(),
            tipo_val,
            &monto_normalizado,
            &moneda_codigo,
            &fecha_usar,
            &hoy,
            &id_cat_resuelto,
            entrada.nota.clone(),
            &estado.categorias,
            &estado.monedas,
        );

        let mut errores = Vec::new();
        if cuenta_ev.is_none() {
            errores.push(ErrorValidacion {
                campo: Some("cuenta".to_string()),
                mensaje: "La cuenta indicada no existe.".to_string(),
            });
        }
        if tipo_res.is_err() {
            errores.push(ErrorValidacion {
                campo: Some("tipo".to_string()),
                mensaje: "El tipo de movimiento debe ser 'ingreso' o 'gasto'.".to_string(),
            });
        }

        match res_dominio {
            Err(errs_dom) => {
                for e in convertir_errores(errs_dom) {
                    if e.campo.as_deref() == Some("cuenta") && cuenta_ev.is_none() {
                        continue;
                    }
                    errores.push(e);
                }
                Err(ErrorAplicacion::Validacion(errores))
            }
            Ok((_mov, mut ev_dom)) => {
                if !errores.is_empty() {
                    return Err(ErrorAplicacion::Validacion(errores));
                }

                ev_dom.id_movimiento = self.ids.nuevo_id();
                let evento = EventoRegistrado {
                    id: self.ids.nuevo_id(),
                    tipo: TIPO_MOVIMIENTO_REGISTRADO.to_string(),
                    momento_registro: self.reloj.ahora_utc(),
                    dispositivo_origen: self.dispositivo.id_dispositivo(),
                    datos: codec::serializar_movimiento(&ev_dom),
                };
                self.repositorio.guardar(&evento)?;

                let cat = buscar_resumen_categoria(&ev_dom.id_categoria, &estado.categorias);
                Ok(armar_movimiento_original_listado(&ev_dom, cat, &[]))
            }
        }
    }

    /// Corrige un movimiento registrado creando un movimiento de ajuste por la diferencia
    /// que referencia al original, sin modificar ni borrar el registro original (INV-04).
    pub fn corregir_movimiento(
        &mut self,
        entrada: DatosCorregirMovimiento,
    ) -> Result<MovimientoListado, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let id_buscado = entrada.movimiento_id.trim();

        let mut movimientos_para_dominio: Vec<Movimiento> = estado
            .movimientos
            .iter()
            .map(|m| m.movimiento.clone())
            .collect();

        if let Some(aj_existente) = estado
            .ajustes
            .iter()
            .find(|a| a.ajuste.id_ajuste == id_buscado)
            && let Some(orig) = estado
                .movimientos
                .iter()
                .find(|m| m.movimiento.id == aj_existente.ajuste.id_movimiento_original)
        {
            movimientos_para_dominio.push(Movimiento {
                id: aj_existente.ajuste.id_ajuste.clone(),
                id_cuenta: orig.movimiento.id_cuenta.clone(),
                tipo: orig.movimiento.tipo,
                monto: orig.movimiento.monto.clone(),
                fecha: aj_existente.fecha.clone(),
                id_categoria: orig.movimiento.id_categoria.clone(),
                nota: None,
                estado: EstadoMovimiento::Ajuste {
                    id_movimiento_original: orig.movimiento.id.clone(),
                },
            });
        }

        let ajustes_previos: Vec<Ajuste> =
            estado.ajustes.iter().map(|a| a.ajuste.clone()).collect();
        let monto_normalizado = normalizar_separador_decimal(&entrada.monto_correcto);

        let (_ajuste, mut ev_corr) = corregir_movimiento(
            String::new(),
            id_buscado,
            &monto_normalizado,
            &movimientos_para_dominio,
            &ajustes_previos,
        )
        .map_err(|errs| ErrorAplicacion::Validacion(convertir_errores(errs)))?;

        ev_corr.id_ajuste = self.ids.nuevo_id();
        let momento = self.reloj.ahora_utc();
        let fecha_ajuste = if momento.len() >= 10 {
            momento[..10].to_string()
        } else {
            self.reloj.hoy()
        };

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_MOVIMIENTO_CORREGIDO.to_string(),
            momento_registro: momento,
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar_correccion(&ev_corr),
        };
        self.repositorio.guardar(&evento)?;

        let orig = estado
            .movimientos
            .iter()
            .find(|m| m.movimiento.id == ev_corr.id_movimiento_original)
            .ok_or_else(|| {
                ErrorAplicacion::Interno("Movimiento original no encontrado".to_string())
            })?;

        let cat = buscar_resumen_categoria(&orig.movimiento.id_categoria, &estado.categorias);
        Ok(armar_ajuste_listado(
            &ev_corr,
            &orig.movimiento,
            cat,
            &fecha_ajuste,
        ))
    }

    /// Simula el efecto de un movimiento sobre el saldo actual de la cuenta sin persistir cambios.
    /// Permite a la interfaz advertir si un gasto deja el saldo en negativo (RF-LIB-04).
    pub fn previsualizar_movimiento(
        &self,
        entrada: DatosPrevisualizarMovimiento,
    ) -> Result<PrevisualizacionMovimiento, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;

        let cuenta_ev = estado
            .cuentas
            .iter()
            .find(|c| c.id == entrada.cuenta_id.trim())
            .ok_or_else(|| {
                ErrorAplicacion::Validacion(vec![ErrorValidacion {
                    campo: Some("cuenta".to_string()),
                    mensaje: "La cuenta indicada no existe.".to_string(),
                }])
            })?;

        let tipo = codec::texto_a_tipo_movimiento(&entrada.tipo).map_err(|_| {
            ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("tipo".to_string()),
                mensaje: "El tipo de movimiento debe ser 'ingreso' o 'gasto'.".to_string(),
            }])
        })?;

        let monto_norm = normalizar_separador_decimal(&entrada.monto);
        let mut cantidad = Decimal::from_str(&monto_norm).map_err(|_| {
            ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("monto".to_string()),
                mensaje: "El monto no es un nÃºmero vÃ¡lido".to_string(),
            }])
        })?;

        if cantidad <= Decimal::ZERO {
            return Err(ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("monto".to_string()),
                mensaje: "El monto debe ser mayor que cero".to_string(),
            }]));
        }

        let escala = cantidad.normalize().scale();
        if escala > cuenta_ev.moneda.decimales {
            return Err(ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("monto".to_string()),
                mensaje: format!(
                    "El monto tiene {} decimales, pero {} solo admite {}",
                    escala, cuenta_ev.moneda.codigo, cuenta_ev.moneda.decimales
                ),
            }]));
        }
        cantidad.rescale(cuenta_ev.moneda.decimales);

        let comp = calcular_composicion_de_cuenta(cuenta_ev, &estado)?;
        let (_, mov_hipotetico) = registrar_movimiento_con_monedas(
            "prev".to_string(),
            cuenta_ev.id.clone(),
            TipoMovimiento::Gasto,
            &cantidad.to_string(),
            &cuenta_ev.moneda.codigo,
            "2026-01-01",
            "2026-01-01",
            "gasto:alimentacion",
            None,
            &categorias_de_fabrica(),
            &estado.monedas,
        )
        .map_err(|errs| ErrorAplicacion::Validacion(convertir_errores(errs)))?;

        let monto_op = Monto::from_evento_interno(&mov_hipotetico, &cuenta_ev.moneda);

        let resultante = match tipo {
            TipoMovimiento::Gasto => saldo_resultante(&comp.saldo_actual, &monto_op)
                .map_err(|e| ErrorAplicacion::Validacion(convertir_errores(vec![e])))?,
            TipoMovimiento::Ingreso => comp
                .saldo_actual
                .sumar(&monto_op)
                .map_err(|e| ErrorAplicacion::Validacion(convertir_errores(vec![e])))?,
        };

        let cant_res = resultante.cantidad();
        let deja_negativo = matches!(tipo, TipoMovimiento::Gasto)
            && cant_res.is_sign_negative()
            && !cant_res.is_zero();

        Ok(PrevisualizacionMovimiento {
            saldo_resultante: formatear_monto(cant_res, cuenta_ev.moneda.decimales),
            deja_saldo_negativo: deja_negativo,
        })
    }

    /// Obtiene el detalle completo de una cuenta: datos generales, composiciÃ³n del saldo
    /// y lista de movimientos y ajustes del mÃ¡s reciente al mÃ¡s antiguo.
    pub fn detalle_cuenta(&self, cuenta_id: &str) -> Result<DetalleCuentaVista, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let id_limpio = cuenta_id.trim();

        let cuenta_ev = estado
            .cuentas
            .iter()
            .find(|c| c.id == id_limpio)
            .ok_or_else(|| {
                ErrorAplicacion::Validacion(vec![ErrorValidacion {
                    campo: Some("cuenta".to_string()),
                    mensaje: "La cuenta indicada no existe.".to_string(),
                }])
            })?;

        let comp = calcular_composicion_de_cuenta(cuenta_ev, &estado)?;
        let decs = cuenta_ev.moneda.decimales;

        let cuenta_vista = CuentaListada {
            id: cuenta_ev.id.clone(),
            nombre: cuenta_ev.nombre.clone(),
            tipo: codec::tipo_a_texto(&cuenta_ev.tipo).to_string(),
            moneda: cuenta_ev.moneda.codigo.clone(),
            saldo_actual: formatear_monto(comp.saldo_actual.cantidad(), decs),
        };

        let composicion_vista = ComposicionVista {
            saldo_inicial: formatear_monto(comp.saldo_inicial.cantidad(), decs),
            total_ingresos: formatear_monto(comp.total_ingresos.cantidad(), decs),
            total_gastos: formatear_monto(comp.total_gastos.cantidad(), decs),
            total_ajustes: formatear_monto(comp.total_ajustes.cantidad(), decs),
        };

        let mut items_ordenados: Vec<(String, usize, MovimientoListado)> = Vec::new();

        for mp in estado
            .movimientos
            .iter()
            .filter(|m| m.movimiento.id_cuenta == cuenta_ev.id)
        {
            let ajustes_mov: Vec<Ajuste> = estado
                .ajustes
                .iter()
                .filter(|a| a.ajuste.id_movimiento_original == mp.movimiento.id)
                .map(|a| a.ajuste.clone())
                .collect();

            let cat = buscar_resumen_categoria(&mp.movimiento.id_categoria, &estado.categorias);
            let ev_equiv = MovimientoRegistrado {
                id_movimiento: mp.movimiento.id.clone(),
                id_cuenta: mp.movimiento.id_cuenta.clone(),
                tipo: mp.movimiento.tipo,
                monto: mp.movimiento.monto.cantidad(),
                moneda: mp.movimiento.monto.moneda().clone(),
                fecha: mp.movimiento.fecha.clone(),
                id_categoria: mp.movimiento.id_categoria.clone(),
                nota: mp.movimiento.nota.clone(),
            };
            let vista = armar_movimiento_original_listado(&ev_equiv, cat, &ajustes_mov);
            items_ordenados.push((mp.movimiento.fecha.clone(), mp.orden, vista));
        }

        for ap in &estado.ajustes {
            if let Some(orig) = estado
                .movimientos
                .iter()
                .find(|m| m.movimiento.id == ap.ajuste.id_movimiento_original)
                && orig.movimiento.id_cuenta == cuenta_ev.id
            {
                let cat =
                    buscar_resumen_categoria(&orig.movimiento.id_categoria, &estado.categorias);
                let ev_corr = MovimientoCorregido {
                    id_ajuste: ap.ajuste.id_ajuste.clone(),
                    id_movimiento_original: ap.ajuste.id_movimiento_original.clone(),
                    monto_correcto: ap.ajuste.monto_correcto,
                    efecto: ap.ajuste.efecto,
                    moneda: ap.ajuste.moneda.clone(),
                };
                let vista = armar_ajuste_listado(&ev_corr, &orig.movimiento, cat, &ap.fecha);
                items_ordenados.push((ap.fecha.clone(), ap.orden, vista));
            }
        }

        items_ordenados.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

        let movimientos = items_ordenados.into_iter().map(|(_, _, m)| m).collect();

        Ok(DetalleCuentaVista {
            cuenta: cuenta_vista,
            composicion: composicion_vista,
            movimientos,
        })
    }

    /// Recorre todos los eventos del repositorio garantizando idempotencia por `id` (INV-13)
    /// y reconstruye el estado de cuentas, categorÃ­as, movimientos y ajustes.
    fn proyectar_libro(&self) -> Result<EstadoLibro, ErrorAplicacion> {
        let eventos = self.repositorio.listar()?;
        let mut vistos = HashSet::new();
        let mut monedas = Moneda::catalogo();
        let mut cuentas: Vec<CuentaEnLibro> = Vec::new();
        let mut tasas = Vec::new();
        let mut moneda_base = "USD".to_string();
        let mut categorias = categorias_de_fabrica();
        let mut movimientos: Vec<MovimientoProyectado> = Vec::new();
        let mut ajustes: Vec<AjusteProyectado> = Vec::new();

        for (indice, e) in eventos.iter().enumerate() {
            if !vistos.insert(e.id.clone()) {
                continue;
            }
            match e.tipo.as_str() {
                TIPO_MONEDA_CREADA => {
                    let ev = codec::deserializar_moneda_creada(&e.datos)?;
                    if Moneda::por_codigo_en(&ev.codigo, &monedas).is_none() {
                        monedas.push(Moneda::nueva_propia(
                            &ev.codigo,
                            &ev.nombre,
                            &ev.simbolo,
                            ev.decimales,
                        ));
                    }
                }
                TIPO_CUENTA_CREADA => {
                    let fecha_creacion = if e.momento_registro.len() >= 10 {
                        e.momento_registro[..10].to_string()
                    } else {
                        "1970-01-01".to_string()
                    };
                    let cuenta = codec::deserializar(&e.datos, &monedas)?;
                    cuentas.push(CuentaEnLibro {
                        cuenta,
                        fecha_creacion,
                    });
                }
                TIPO_CATEGORIA_CREADA => {
                    let ev = codec::deserializar_categoria(&e.datos)?;
                    let id = id_categoria_canonico(&ev.id);
                    if !categorias.iter().any(|c| c.id == id) {
                        let tipo = match ev.tipo.as_str() {
                            "ingreso" => TipoMovimiento::Ingreso,
                            _ => TipoMovimiento::Gasto,
                        };
                        categorias.push(Categoria {
                            id,
                            nombre: ev.nombre,
                            tipo,
                            de_fabrica: ev.es_fabrica,
                        });
                    }
                }
                TIPO_MOVIMIENTO_REGISTRADO => {
                    let ev = codec::deserializar_evento_movimiento(&e.datos, &monedas)?;
                    let monto = Monto::from_evento_interno(&ev, &ev.moneda);
                    movimientos.push(MovimientoProyectado {
                        orden: indice,
                        movimiento: Movimiento {
                            id: ev.id_movimiento,
                            id_cuenta: ev.id_cuenta,
                            tipo: ev.tipo,
                            monto,
                            fecha: ev.fecha,
                            id_categoria: id_categoria_canonico(&ev.id_categoria),
                            nota: ev.nota,
                            estado: EstadoMovimiento::Vigente,
                        },
                    });
                }
                TIPO_MOVIMIENTO_CORREGIDO => {
                    let ev = codec::deserializar_correccion(&e.datos, &monedas)?;
                    if let Some(orig) = movimientos
                        .iter_mut()
                        .find(|m| m.movimiento.id == ev.id_movimiento_original)
                    {
                        orig.movimiento.estado = EstadoMovimiento::Corregido;
                    }
                    let fecha = if e.momento_registro.len() >= 10 {
                        e.momento_registro[..10].to_string()
                    } else if let Some(orig) = movimientos
                        .iter()
                        .find(|m| m.movimiento.id == ev.id_movimiento_original)
                    {
                        orig.movimiento.fecha.clone()
                    } else {
                        "2026-10-19".to_string()
                    };
                    ajustes.push(AjusteProyectado {
                        orden: indice,
                        fecha,
                        ajuste: Ajuste {
                            id_ajuste: ev.id_ajuste,
                            id_movimiento_original: ev.id_movimiento_original,
                            monto_correcto: ev.monto_correcto,
                            efecto: ev.efecto,
                            moneda: ev.moneda,
                        },
                    });
                }
                TIPO_TASA_REGISTRADA => {
                    let ev = codec::deserializar_tasa_registrada(&e.datos)?;
                    tasas.push(dominio::Tasa {
                        id: ev.id,
                        moneda_origen: ev.moneda_origen,
                        moneda_destino: ev.moneda_destino,
                        valor: ev.valor,
                        fecha: ev.fecha,
                        fuente: ev.fuente,
                    });
                }
                TIPO_TASA_ELIMINADA => {
                    let id_tasa = codec::deserializar_tasa_eliminada(&e.datos)?;
                    tasas.retain(|t| t.id != id_tasa);
                }
                TIPO_MONEDA_BASE_CAMBIADA => {
                    let ev = codec::deserializar_moneda_base_cambiada(&e.datos)?;
                    moneda_base = ev;
                }
                _ => continue,
            }
        }

        Ok(EstadoLibro {
            monedas,
            cuentas,
            categorias,
            movimientos,
            ajustes,
            tasas,
            moneda_base,
        })
    }
    pub fn registrar_tasa(&mut self, entrada: NuevaTasa) -> Result<TasaListada, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let valor_limpio = normalizar_separador_decimal(&entrada.valor);

        let valor = dominio::validar_tasa(
            &entrada.moneda_origen,
            &entrada.moneda_destino,
            &valor_limpio,
            &entrada.fecha,
            &entrada.fuente,
            &self.reloj.hoy(),
            &estado.monedas,
        )
        .map_err(|errs| ErrorAplicacion::Validacion(convertir_errores(errs)))?;

        let id = self.ids.nuevo_id();
        let ev = dominio::TasaRegistrada {
            id: id.clone(),
            moneda_origen: entrada.moneda_origen.trim().to_uppercase(),
            moneda_destino: entrada.moneda_destino.trim().to_uppercase(),
            valor,
            fecha: entrada.fecha.trim().to_string(),
            fuente: entrada.fuente.trim().to_string(),
        };

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_TASA_REGISTRADA.to_string(),
            momento_registro: self.reloj.ahora_utc(),
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar_tasa(&ev),
        };
        self.repositorio.guardar(&evento)?;

        let decs = estado
            .monedas
            .iter()
            .find(|m| m.codigo.eq_ignore_ascii_case(&ev.moneda_destino))
            .map(|m| m.decimales)
            .unwrap_or(2);

        Ok(TasaListada {
            id: ev.id,
            moneda_origen: ev.moneda_origen,
            moneda_destino: ev.moneda_destino,
            valor: formatear_monto(ev.valor, decs),
            fecha: ev.fecha,
            fuente: ev.fuente,
        })
    }

    /// Elimina lÃ³gicamente una tasa de cambio existente registrando el evento `TasaEliminada` (RF-CAM-02).
    pub fn eliminar_tasa(&mut self, id: &str) -> Result<(), ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let id_limpio = id.trim();
        if !estado.tasas.iter().any(|t| t.id == id_limpio) {
            return Err(ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("tasa".to_string()),
                mensaje: format!("La tasa con id '{id_limpio}' no existe o ya fue eliminada"),
            }]));
        }

        let _ev = dominio::TasaEliminada {
            id: id_limpio.to_string(),
        };

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_TASA_ELIMINADA.to_string(),
            momento_registro: self.reloj.ahora_utc(),
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar_tasa_eliminada(id_limpio),
        };
        self.repositorio.guardar(&evento)?;
        Ok(())
    }

    /// Devuelve el historial de tasas de cambio activas (no eliminadas) en orden de registro.
    pub fn listar_tasas(&self) -> Result<Vec<TasaListada>, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        Ok(estado
            .tasas
            .iter()
            .map(|t| {
                let decs = estado
                    .monedas
                    .iter()
                    .find(|m| m.codigo.eq_ignore_ascii_case(&t.moneda_destino))
                    .map(|m| m.decimales)
                    .unwrap_or(2);
                TasaListada {
                    id: t.id.clone(),
                    moneda_origen: t.moneda_origen.clone(),
                    moneda_destino: t.moneda_destino.clone(),
                    valor: formatear_monto(t.valor, decs),
                    fecha: t.fecha.clone(),
                    fuente: t.fuente.clone(),
                }
            })
            .collect())
    }

    /// Modifica la moneda base utilizada para consolidar el patrimonio (RF-CAM-03).
    pub fn cambiar_moneda_base(&mut self, nueva_base: &str) -> Result<String, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        let base_limpia = nueva_base.trim().to_uppercase();

        if !estado
            .monedas
            .iter()
            .any(|m| m.codigo.eq_ignore_ascii_case(&base_limpia))
        {
            return Err(ErrorAplicacion::Validacion(vec![ErrorValidacion {
                campo: Some("moneda".to_string()),
                mensaje: format!("La moneda '{base_limpia}' no estÃ¡ en el catÃ¡logo"),
            }]));
        }

        let _ev = dominio::MonedaBaseCambiada {
            nueva_moneda_base: base_limpia.clone(),
        };

        let evento = EventoRegistrado {
            id: self.ids.nuevo_id(),
            tipo: TIPO_MONEDA_BASE_CAMBIADA.to_string(),
            momento_registro: self.reloj.ahora_utc(),
            dispositivo_origen: self.dispositivo.id_dispositivo(),
            datos: codec::serializar_moneda_base_cambiada(&base_limpia),
        };
        self.repositorio.guardar(&evento)?;
        Ok(base_limpia)
    }

    /// Obtiene la moneda base vigente para la consolidaciÃ³n de patrimonio (por defecto `"USD"`).
    pub fn obtener_moneda_base(&self) -> Result<String, ErrorAplicacion> {
        let estado = self.proyectar_libro()?;
        Ok(estado.moneda_base)
    }




    /// Consulta la posición patrimonial consolidada a una fecha dada (o la fecha del sistema).
    ///
    /// Agrupa las cuentas existentes a la fecha, calcula sus saldos, genera subtotales
    /// por moneda y, si existen tasas de cambio aplicables, consolida el total en la
    /// moneda base configurada.
    pub fn consultar_patrimonio(
        &self,
        fecha_consulta: Option<String>,
    ) -> Result<PatrimonioVista, ErrorAplicacion> {
        let fecha = match fecha_consulta {
            Some(f) => {
                let f_limpia = f.trim().to_string();
                dominio::validar_fecha(&f_limpia)
                    .map_err(|e| ErrorAplicacion::Validacion(convertir_errores(vec![e])))?;
                if f_limpia > self.reloj.hoy() {
                    return Err(ErrorAplicacion::Validacion(vec![ErrorValidacion {
                        campo: Some("fecha".to_string()),
                        mensaje: "La fecha de consulta no puede ser futura".to_string(),
                    }]));
                }
                f_limpia
            }
            None => self.reloj.hoy(),
        };

        let estado = self.proyectar_libro()?;
        let mut cuentas_vista = Vec::new();
        let mut saldos_por_cuenta = Vec::new();

        for cl in &estado.cuentas {
            // "una cuenta no aparece antes de su fecha de creación"
            if cl.fecha_creacion > fecha {
                continue;
            }

            // Calcular saldo de la cuenta a la fecha de consulta
            let comp = calcular_composicion_a_fecha(&cl.cuenta, &estado, &fecha)?;
            let saldo_dec = comp.saldo_actual.cantidad();
            let saldo_str = formatear_monto(saldo_dec, cl.cuenta.moneda.decimales);

            cuentas_vista.push(CuentaPatrimonioVista {
                id: cl.cuenta.id.clone(),
                nombre: cl.cuenta.nombre.clone(),
                moneda: cl.cuenta.moneda.codigo.clone(),
                saldo: saldo_str,
            });

            saldos_por_cuenta.push((
                cl.cuenta.moneda.codigo.clone(),
                cl.cuenta.moneda.decimales,
                saldo_dec,
            ));
        }

        let subtotales_vista = calcular_subtotales(&saldos_por_cuenta);
        let subtotales_dec: Vec<(String, Decimal)> = subtotales_vista
            .iter()
            .map(|s| {
                let d = Decimal::from_str(&s.total).unwrap_or(Decimal::ZERO);
                (s.moneda.clone(), d)
            })
            .collect();

        let cons = dominio::consolidar_subtotales(
            &estado.moneda_base,
            &fecha,
            &subtotales_dec,
            &estado.tasas,
            &estado.monedas,
        );

        let decimales_base = estado
            .monedas
            .iter()
            .find(|m| m.codigo.eq_ignore_ascii_case(&estado.moneda_base))
            .map(|m| m.decimales)
            .unwrap_or(2);

        let total_str = cons.total.map(|t| formatear_monto(t, decimales_base));

        let tasas_usadas: Vec<TasaUsadaVista> = cons
            .tasas_usadas
            .into_iter()
            .map(|t| {
                let decs = estado
                    .monedas
                    .iter()
                    .find(|m| m.codigo.eq_ignore_ascii_case(&t.moneda_destino))
                    .map(|m| m.decimales)
                    .unwrap_or(2);
                TasaUsadaVista {
                    origen: t.moneda_origen,
                    destino: t.moneda_destino,
                    valor: formatear_monto(t.valor, decs),
                    fecha: t.fecha,
                    fuente: t.fuente,
                    es_inversa: t.es_inversa,
                }
            })
            .collect();

        let faltantes: Vec<ParFaltanteVista> = cons
            .faltantes
            .into_iter()
            .map(|f| ParFaltanteVista {
                origen: f.moneda_origen,
                destino: f.moneda_destino,
            })
            .collect();

        Ok(PatrimonioVista {
            moneda_base: estado.moneda_base,
            fecha_consulta: fecha,
            cuentas: cuentas_vista,
            subtotales: subtotales_vista,
            total: total_str,
            tasas_usadas,
            faltantes,
        })
    }
}

/// ExtensiÃ³n auxiliar para obtener un [`Monto`] validado desde un evento de movimiento.
trait MontoDesdeEvento {
    fn from_evento_interno(ev: &MovimientoRegistrado, moneda: &Moneda) -> Monto;
}

impl MontoDesdeEvento for Monto {
    fn from_evento_interno(ev: &MovimientoRegistrado, moneda: &Moneda) -> Monto {
        let (cuenta_base, _) = Cuenta::crear_con_monedas(
            "tmp".to_string(),
            "Tmp",
            TipoCuenta::Efectivo,
            &moneda.codigo,
            &ev.monto.to_string(),
            &[],
            std::slice::from_ref(moneda),
        )
        .expect("monto de evento previamente validado");
        cuenta_base.saldo().clone()
    }
}

/// Calcula la composiciÃ³n y el saldo derivado de una cuenta usando la funciÃ³n pura del dominio.
fn calcular_composicion_de_cuenta(
    cuenta_ev: &CuentaCreada,
    estado: &EstadoLibro,
) -> Result<dominio::ComposicionSaldo, ErrorAplicacion> {
    calcular_composicion_a_fecha(cuenta_ev, estado, "9999-12-31")
}

/// Calcula la composición y el saldo de una cuenta hasta una fecha de corte dada.
fn calcular_composicion_a_fecha(
    cuenta_ev: &CuentaCreada,
    estado: &EstadoLibro,
    fecha_corte: &str,
) -> Result<dominio::ComposicionSaldo, ErrorAplicacion> {
    let (cuenta_dom, _) = Cuenta::crear_con_monedas(
        cuenta_ev.id.clone(),
        &cuenta_ev.nombre,
        cuenta_ev.tipo,
        &cuenta_ev.moneda.codigo,
        &cuenta_ev.saldo_inicial.to_string(),
        &[],
        std::slice::from_ref(&cuenta_ev.moneda),
    )
    .map_err(|e| ErrorAplicacion::Interno(format!("Cuenta corrupta en proyección: {e:?}")))?;

    let movs_cuenta: Vec<Movimiento> = estado
        .movimientos
        .iter()
        .filter(|m| {
            m.movimiento.id_cuenta == cuenta_ev.id && m.movimiento.fecha.as_str() <= fecha_corte
        })
        .map(|m| m.movimiento.clone())
        .collect();

    let ids_movs_cuenta: HashSet<&str> = movs_cuenta.iter().map(|m| m.id.as_str()).collect();

    let ajustes_cuenta: Vec<Ajuste> = estado
        .ajustes
        .iter()
        .filter(|a| {
            ids_movs_cuenta.contains(a.ajuste.id_movimiento_original.as_str())
                && a.fecha.as_str() <= fecha_corte
        })
        .map(|a| a.ajuste.clone())
        .collect();

    calcular_saldo_y_composicion(&cuenta_dom, &movs_cuenta, &ajustes_cuenta)
        .map_err(|e| ErrorAplicacion::Interno(format!("Error al calcular saldo: {e:?}")))
}

fn resolver_id_categoria(entrada: &str, categorias: &[Categoria]) -> String {
    let limpio = id_categoria_canonico(entrada.trim());
    if limpio.is_empty() {
        return String::new();
    }
    if let Some(c) = categorias.iter().find(|c| c.id == limpio) {
        return c.id.clone();
    }
    let lower = limpio.to_lowercase();
    if let Some(c) = categorias
        .iter()
        .find(|c| c.nombre.trim().to_lowercase() == lower)
    {
        return c.id.clone();
    }
    match lower.as_str() {
        "otros ingresos" | "ingreso:otros_ingresos" => "ingreso:otros".to_string(),
        "alimentacion" => "gasto:alimentacion".to_string(),
        "educacion" => "gasto:educacion".to_string(),
        _ => limpio.to_string(),
    }
}

/// Obtiene el resumen de una categorÃ­a por su identificador.
fn buscar_resumen_categoria(id_cat: &str, categorias: &[Categoria]) -> CategoriaResumen {
    if let Some(c) = categorias.iter().find(|c| c.id == id_cat) {
        CategoriaResumen {
            id: c.id.clone(),
            nombre: c.nombre.clone(),
        }
    } else {
        CategoriaResumen {
            id: id_cat.to_string(),
            nombre: id_cat.to_string(),
        }
    }
}

/// Construye la vista de un movimiento original registrado, calculando su valor vigente
/// a partir de su monto original y el efecto acumulado de sus ajustes.
fn armar_movimiento_original_listado(
    ev: &MovimientoRegistrado,
    categoria: CategoriaResumen,
    ajustes_mov: &[Ajuste],
) -> MovimientoListado {
    let decs = ev.moneda.decimales;
    let monto_str = formatear_monto(ev.monto, decs);
    let monto_con_signo = match ev.tipo {
        TipoMovimiento::Ingreso => format!("+{monto_str}"),
        TipoMovimiento::Gasto => format!("-{monto_str}"),
    };

    let suma_efectos: Decimal = ajustes_mov.iter().map(|a| a.efecto).sum();
    let vigente_dec = match ev.tipo {
        TipoMovimiento::Gasto => ev.monto - suma_efectos,
        TipoMovimiento::Ingreso => ev.monto + suma_efectos,
    };
    let monto_vigente_str = formatear_monto(vigente_dec, decs);

    let ajustes: Vec<String> = ajustes_mov.iter().map(|a| a.id_ajuste.clone()).collect();
    let estado = if ajustes.is_empty() {
        "vigente".to_string()
    } else {
        "corregido".to_string()
    };

    MovimientoListado {
        id: ev.id_movimiento.clone(),
        cuenta_id: ev.id_cuenta.clone(),
        tipo: codec::tipo_movimiento_a_texto(&ev.tipo).to_string(),
        monto: monto_str,
        monto_con_signo,
        monto_vigente: Some(monto_vigente_str),
        fecha: ev.fecha.clone(),
        categoria,
        nota: ev.nota.clone().unwrap_or_default(),
        estado,
        corrige_a: None,
        ajustes,
    }
}

/// Construye la vista de un movimiento de ajuste que referencia a un movimiento original.
fn armar_ajuste_listado(
    ev: &MovimientoCorregido,
    original: &Movimiento,
    categoria: CategoriaResumen,
    fecha: &str,
) -> MovimientoListado {
    let decs = ev.moneda.decimales;
    let monto_abs = formatear_monto(ev.efecto.abs(), decs);
    let monto_con_signo = if ev.efecto.is_sign_negative() && !ev.efecto.is_zero() {
        format!("-{monto_abs}")
    } else {
        format!("+{monto_abs}")
    };

    MovimientoListado {
        id: ev.id_ajuste.clone(),
        cuenta_id: original.id_cuenta.clone(),
        tipo: codec::tipo_movimiento_a_texto(&original.tipo).to_string(),
        monto: monto_abs,
        monto_con_signo,
        monto_vigente: None,
        fecha: fecha.to_string(),
        categoria,
        nota: String::new(),
        estado: "ajuste".to_string(),
        corrige_a: Some(original.id.clone()),
        ajustes: Vec::new(),
    }
}

/// Normaliza un monto escrito con coma decimal para que pueda ser parseado por `Decimal`.
fn normalizar_separador_decimal(texto: &str) -> String {
    let limpio = texto.trim();
    if limpio.contains(',') && !limpio.contains('.') {
        limpio.replace(',', ".")
    } else {
        limpio.to_string()
    }
}

/// Formatea una cantidad decimal con el nÃºmero exacto de decimales definidos para su moneda.
fn formatear_monto(cantidad: Decimal, decimales: u32) -> String {
    format!("{:.prec$}", cantidad, prec = decimales as usize)
}

/// Calcula la suma exacta de saldos actuales por cada moneda presente, conservando el orden de apariciÃ³n.
fn calcular_subtotales(saldos: &[(String, u32, Decimal)]) -> Vec<Subtotal> {
    let mut monedas_orden: Vec<(String, u32)> = Vec::new();
    let mut sumas: HashMap<String, Decimal> = HashMap::new();

    for (cod, decs, saldo) in saldos {
        if !sumas.contains_key(cod) {
            monedas_orden.push((cod.clone(), *decs));
        }
        let actual = sumas
            .entry(cod.clone())
            .or_insert_with(|| Decimal::new(0, 0));
        *actual += *saldo;
    }

    let mut subtotales = Vec::new();
    for (m, decimales) in monedas_orden {
        subtotales.push(Subtotal {
            moneda: m.clone(),
            total: formatear_monto(sumas[&m], decimales),
        });
    }
    subtotales
}

/// Convierte una moneda del dominio al modelo de lectura de la interfaz.
fn moneda_a_dto(m: Moneda) -> MonedaDto {
    MonedaDto {
        codigo: m.codigo,
        nombre: m.nombre,
        simbolo: m.simbolo,
        decimales: m.decimales as u8,
        de_fabrica: m.de_fabrica,
    }
}

/// Transforma la lista de errores de dominio al tipo [`ErrorValidacion`] de la capa de aplicaciÃ³n.
fn convertir_errores(errores: Vec<ErrorDominio>) -> Vec<ErrorValidacion> {
    errores
        .into_iter()
        .map(|e| ErrorValidacion {
            campo: Some(Campo::a_texto(&e.campo).to_string()),
            mensaje: e.mensaje,
        })
        .collect()
}
