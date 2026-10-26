//! Casos de uso de la aplicación: gestión de cuentas, categorías, registro y
//! corrección de movimientos, previsualización de saldo y consulta de detalles.

use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use dominio::{
    Ajuste, Campo, Categoria, Cuenta, CuentaCreada, ErrorDominio, EstadoMovimiento, Moneda, Monto,
    Movimiento, MovimientoCorregido, MovimientoRegistrado, TipoCuenta, TipoMovimiento,
    calcular_saldo_y_composicion, categorias_de_fabrica, corregir_movimiento, crear_categoria,
    registrar_movimiento, saldo_resultante,
};

use crate::categorias::id_categoria_canonico;
use crate::codec;
use crate::errores::{ErrorAplicacion, ErrorValidacion};
use crate::puertos::{
    DispositivoOrigen, EventoRegistrado, GeneradorIdentificadores, Reloj, RepositorioEventos,
};
use crate::vistas::{
    CategoriaListada, CategoriaResumen, ComposicionVista, CuentaListada, DetalleCuentaVista,
    ListaCuentas, MonedaDto, MovimientoListado, PrevisualizacionMovimiento, Subtotal,
};

/// Identificador de tipo para los eventos de creación de cuenta.
pub const TIPO_CUENTA_CREADA: &str = "CuentaCreada";
/// Identificador de tipo para los eventos de registro de movimiento.
pub const TIPO_MOVIMIENTO_REGISTRADO: &str = "MovimientoRegistrado";
/// Identificador de tipo para los eventos de corrección de movimiento (ajuste).
pub const TIPO_MOVIMIENTO_CORREGIDO: &str = "MovimientoCorregido";
/// Identificador de tipo para los eventos de creación de categoría personalizada.
pub const TIPO_CATEGORIA_CREADA: &str = "CategoriaCreada";

/// Datos de entrada requeridos para registrar una nueva cuenta.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaCuenta {
    /// Nombre de la cuenta (obligatorio y único sin distinguir mayúsculas).
    pub nombre: String,
    /// Tipo de cuenta (`"efectivo"`, `"banco"` o `"billetera_digital"`).
    pub tipo: String,
    /// Código de la moneda del catálogo (`"VES"`, `"USD"`, `"EUR"`, `"USDT"`).
    pub moneda: String,
    /// Saldo inicial expresado como cadena decimal (por ejemplo, `"1250.50"`).
    pub saldo_inicial: String,
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
    /// Identificador o nombre de la categoría asociada.
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

/// Datos de entrada requeridos para crear una nueva categoría de usuario.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct NuevaCategoria {
    /// Nombre de la categoría (único dentro de su mismo tipo sin distinguir mayúsculas).
    pub nombre: String,
    /// Tipo de movimiento al que pertenece (`"ingreso"` o `"gasto"`).
    pub tipo: String,
}

/// Registro interno de un movimiento original proyectado junto con su orden de inserción.
#[derive(Debug, Clone)]
struct MovimientoProyectado {
    orden: usize,
    movimiento: Movimiento,
}

/// Registro interno de un ajuste proyectado junto con su fecha y orden de inserción.
#[derive(Debug, Clone)]
struct AjusteProyectado {
    orden: usize,
    fecha: String,
    ajuste: Ajuste,
}

/// Estado completo proyectado en memoria a partir de la secuencia de eventos del libro.
struct EstadoLibro {
    cuentas: Vec<CuentaCreada>,
    categorias: Vec<Categoria>,
    movimientos: Vec<MovimientoProyectado>,
    ajustes: Vec<AjusteProyectado>,
}

/// Servicio de aplicación que agrupa los casos de uso e inyecta los puertos requeridos.
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

    /// Devuelve el catálogo de monedas soportadas por el sistema.
    pub fn listar_monedas(&self) -> Vec<MonedaDto> {
        Moneda::catalogo()
            .into_iter()
            .map(|m| MonedaDto {
                codigo: m.codigo,
                nombre: m.nombre,
                simbolo: m.simbolo,
                decimales: m.decimales as u8,
            })
            .collect()
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

    /// Valida los datos de entrada con una única invocación a `Cuenta::crear` (D-10)
    /// y, si son válidos, sella y persiste el evento `CuentaCreada`.
    pub fn crear_cuenta(&mut self, entrada: NuevaCuenta) -> Result<CuentaListada, ErrorAplicacion> {
        let proyectadas = self.repositorio.listar_cuentas()?;
        let nombres_existentes: Vec<String> =
            proyectadas.iter().map(|c| c.nombre.clone()).collect();

        let tipo_resultado = codec::texto_a_tipo(&entrada.tipo);
        let tipo_para_validar = tipo_resultado.clone().unwrap_or(TipoCuenta::Efectivo);
        let saldo_normalizado = normalizar_separador_decimal(&entrada.saldo_inicial);

        // Una sola llamada a Cuenta::crear (D-10).
        let resultado_dominio = Cuenta::crear(
            String::new(),
            &entrada.nombre,
            tipo_para_validar,
            &entrada.moneda,
            &saldo_normalizado,
            &nombres_existentes,
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
                            mensaje: "El tipo de cuenta no es válido.".to_string(),
                        },
                    );
                }
                Err(ErrorAplicacion::Validacion(errores_validacion))
            }
        }
    }

    /// Lista todas las categorías disponibles (las 11 de fábrica más las creadas por el usuario).
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

    /// Crea una categoría personalizada validando unicidad por tipo (sin distinguir mayúsculas)
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
                        mensaje: "El nombre de la categoría no puede estar vacío".to_string(),
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

        let res_dominio = registrar_movimiento(
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
                mensaje: "El monto no es un número válido".to_string(),
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
        let (_, mov_hipotetico) = registrar_movimiento(
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

    /// Obtiene el detalle completo de una cuenta: datos generales, composición del saldo
    /// y lista de movimientos y ajustes del más reciente al más antiguo.
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
    /// y reconstruye el estado de cuentas, categorías, movimientos y ajustes.
    fn proyectar_libro(&self) -> Result<EstadoLibro, ErrorAplicacion> {
        let eventos = self.repositorio.listar()?;
        let mut vistos = HashSet::new();
        let mut cuentas = Vec::new();
        let mut categorias = categorias_de_fabrica();
        let mut movimientos: Vec<MovimientoProyectado> = Vec::new();
        let mut ajustes: Vec<AjusteProyectado> = Vec::new();

        for (indice, e) in eventos.iter().enumerate() {
            if !vistos.insert(e.id.clone()) {
                continue;
            }
            match e.tipo.as_str() {
                TIPO_CUENTA_CREADA => {
                    cuentas.push(codec::deserializar(&e.datos)?);
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
                    let ev = codec::deserializar_evento_movimiento(&e.datos)?;
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
                    let ev = codec::deserializar_correccion(&e.datos)?;
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
                _ => continue,
            }
        }

        Ok(EstadoLibro {
            cuentas,
            categorias,
            movimientos,
            ajustes,
        })
    }
}

/// Extensión auxiliar para obtener un [`Monto`] validado desde un evento de movimiento.
trait MontoDesdeEvento {
    fn from_evento_interno(ev: &MovimientoRegistrado, moneda: &Moneda) -> Monto;
}

impl MontoDesdeEvento for Monto {
    fn from_evento_interno(ev: &MovimientoRegistrado, moneda: &Moneda) -> Monto {
        let (cuenta_base, _) = Cuenta::crear(
            "tmp".to_string(),
            "Tmp",
            TipoCuenta::Efectivo,
            &moneda.codigo,
            &ev.monto.to_string(),
            &[],
        )
        .expect("monto de evento previamente validado");
        cuenta_base.saldo().clone()
    }
}

/// Calcula la composición y el saldo derivado de una cuenta usando la función pura del dominio.
fn calcular_composicion_de_cuenta(
    cuenta_ev: &CuentaCreada,
    estado: &EstadoLibro,
) -> Result<dominio::ComposicionSaldo, ErrorAplicacion> {
    let (cuenta_dom, _) = Cuenta::crear(
        cuenta_ev.id.clone(),
        &cuenta_ev.nombre,
        cuenta_ev.tipo,
        &cuenta_ev.moneda.codigo,
        &cuenta_ev.saldo_inicial.to_string(),
        &[],
    )
    .map_err(|e| ErrorAplicacion::Interno(format!("Cuenta corrupta en proyección: {e:?}")))?;

    let movs_cuenta: Vec<Movimiento> = estado
        .movimientos
        .iter()
        .filter(|m| m.movimiento.id_cuenta == cuenta_ev.id)
        .map(|m| m.movimiento.clone())
        .collect();

    let ids_movs_cuenta: HashSet<&str> = movs_cuenta.iter().map(|m| m.id.as_str()).collect();

    let ajustes_cuenta: Vec<Ajuste> = estado
        .ajustes
        .iter()
        .filter(|a| ids_movs_cuenta.contains(a.ajuste.id_movimiento_original.as_str()))
        .map(|a| a.ajuste.clone())
        .collect();

    calcular_saldo_y_composicion(&cuenta_dom, &movs_cuenta, &ajustes_cuenta)
        .map_err(|e| ErrorAplicacion::Interno(format!("Error al calcular saldo: {e:?}")))
}

/// Resuelve un identificador o nombre de categoría (incluyendo nombres equivalentes del enunciado).
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

/// Obtiene el resumen de una categoría por su identificador.
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

/// Formatea una cantidad decimal con el número exacto de decimales definidos para su moneda.
fn formatear_monto(cantidad: Decimal, decimales: u32) -> String {
    format!("{:.prec$}", cantidad, prec = decimales as usize)
}

/// Calcula la suma exacta de saldos actuales por cada moneda presente, conservando el orden de aparición.
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

/// Transforma la lista de errores de dominio al tipo [`ErrorValidacion`] de la capa de aplicación.
fn convertir_errores(errores: Vec<ErrorDominio>) -> Vec<ErrorValidacion> {
    errores
        .into_iter()
        .map(|e| ErrorValidacion {
            campo: Some(Campo::a_texto(&e.campo).to_string()),
            mensaje: e.mensaje,
        })
        .collect()
}
