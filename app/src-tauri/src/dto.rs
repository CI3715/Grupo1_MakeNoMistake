use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct MonedaDto {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u8,
}

#[derive(Serialize)]
pub struct CuentaDto {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub moneda: String,
    pub saldo_actual: String,
}

#[derive(Serialize)]
pub struct SubtotalDto {
    pub moneda: String,
    pub total: String,
}

#[derive(Serialize)]
pub struct ListaCuentasDto {
    pub cuentas: Vec<CuentaDto>,
    pub subtotales: Vec<SubtotalDto>,
}

#[derive(Deserialize)]
pub struct NuevaCuentaDto {
    pub nombre: String,
    pub tipo: String,
    pub moneda: String,
    pub saldo_inicial: String,
}

#[derive(Serialize)]
pub struct ErrorCampo {
    pub campo: Option<String>,
    pub mensaje: String,
}

#[derive(Serialize)]
pub struct ErroresDto {
    pub errores: Vec<ErrorCampo>,
}

impl ErroresDto {
    /// Respuesta de los comandos de la Entrega 3 que aún no están conectados.
    #[allow(dead_code)] // los comandos ya no la usan
    #[allow(dead_code)] // los comandos ya no la usan
    pub fn no_implementado() -> Self {
        ErroresDto {
            errores: vec![ErrorCampo {
                campo: None,
                mensaje: "Esta función todavía no está disponible.".to_string(),
            }],
        }
    }
}

// ---------- Entrega 3: categorías, movimientos y detalle de cuenta ----------
//
// Convenciones (las mismas de la Entrega 2):
// - Todos los montos viajan como texto con punto decimal ("50.00"), nunca como
//   número; la interfaz solo les da formato. Un monto negativo lleva "-" y los
//   positivos no llevan "+".
// - Las fechas viajan como texto AAAA-MM-DD. «Hoy» es la fecha local del dispositivo.
// - Nombres en snake_case. Los errores usan `ErroresDto` con el campo que falla
//   (`cuenta`, `tipo`, `monto`, `fecha`, `categoria`, `nota`, `nombre`).
// - Los campos de entrada tienen valor por defecto para que un dato ausente se
//   rechace con un mensaje por campo y no con un error técnico.

/// Categoría de gasto o de ingreso. `tipo`: "gasto" o "ingreso".
#[derive(Serialize)]
pub struct CategoriaDto {
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub de_fabrica: bool,
}

/// Datos para crear una categoría propia. `tipo`: "gasto" o "ingreso".
#[allow(dead_code)] // se leerán al conectar los casos de uso
#[derive(Deserialize)]
pub struct NuevaCategoriaDto {
    #[serde(default)]
    pub nombre: String,
    #[serde(default)]
    pub tipo: String,
}

/// Datos de un gasto o un ingreso nuevo (también para previsualizarlo).
/// `tipo`: "gasto" o "ingreso". `monto`: texto positivo ("50.00"); la moneda es
/// siempre la de la cuenta. `fecha`: AAAA-MM-DD. `nota`: opcional ("" si no hay).
#[allow(dead_code)] // se leerán al conectar los casos de uso
#[derive(Deserialize)]
pub struct NuevoMovimientoDto {
    #[serde(default)]
    pub cuenta_id: String,
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub monto: String,
    #[serde(default)]
    pub fecha: String,
    #[serde(default)]
    pub categoria_id: String,
    #[serde(default)]
    pub nota: String,
}

/// Corrección de un movimiento: el monto correcto (0 lo anula por completo).
#[allow(dead_code)] // se leerán al conectar los casos de uso
#[derive(Deserialize)]
pub struct CorreccionDto {
    #[serde(default)]
    pub movimiento_id: String,
    #[serde(default)]
    pub monto_correcto: String,
}

/// Un movimiento del libro, listo para mostrar.
///
/// - `tipo`: "gasto" o "ingreso" (el ajuste hereda el del original).
/// - `monto`: valor absoluto registrado ("50.00"). `monto_con_signo`: efecto sobre
///   el saldo ("-50.00" para un gasto, "800.00" para un ingreso; en un ajuste, el
///   efecto del ajuste, por ejemplo "450.00").
/// - `estado`: "vigente", "corregido" (tiene al menos un ajuste) o "ajuste".
/// - `monto_vigente`: valor que hoy cuenta del movimiento (solo en gastos e
///   ingresos; `None` en un ajuste).
/// - `corrige_a`: identificador del original (solo en ajustes).
/// - `ajustes`: identificadores de los ajustes que lo corrigen (solo en originales).
/// - `monto_correcto`: monto que la persona indicó al corregir (solo en ajustes).
#[derive(Serialize)]
pub struct MovimientoDto {
    pub id: String,
    pub cuenta_id: String,
    pub tipo: String,
    pub monto: String,
    pub monto_con_signo: String,
    pub monto_vigente: Option<String>,
    pub fecha: String,
    pub categoria_id: String,
    pub categoria_nombre: String,
    pub nota: String,
    pub estado: String,
    pub corrige_a: Option<String>,
    pub ajustes: Vec<String>,
    pub monto_correcto: Option<String>,
}

/// Cómo se compone el saldo: saldo actual = saldo inicial + ingresos − gastos ± ajustes.
/// `total_ajustes` puede ser negativo ("-20.00").
#[derive(Serialize)]
pub struct ComposicionDto {
    pub saldo_inicial: String,
    pub total_ingresos: String,
    pub total_gastos: String,
    pub total_ajustes: String,
}

/// Detalle de una cuenta: saldo actual, composición y movimientos del más
/// reciente al más antiguo (por fecha y, si empatan, por momento de registro).
#[derive(Serialize)]
pub struct DetalleCuentaDto {
    pub cuenta: CuentaDto,
    pub composicion: ComposicionDto,
    pub movimientos: Vec<MovimientoDto>,
}

/// Resultado de previsualizar un movimiento sin guardarlo: el saldo que quedaría
/// y si es negativo (la interfaz advierte antes de confirmar).
#[derive(Serialize)]
pub struct PrevisualizacionDto {
    pub saldo_resultante: String,
    pub deja_saldo_negativo: bool,
}

// ---------- Conversiones desde la capa de aplicación ----------

impl From<aplicacion::MonedaDto> for MonedaDto {
    fn from(m: aplicacion::MonedaDto) -> Self {
        MonedaDto {
            codigo: m.codigo,
            nombre: m.nombre,
            simbolo: m.simbolo,
            decimales: m.decimales,
        }
    }
}

impl From<aplicacion::CuentaListada> for CuentaDto {
    fn from(c: aplicacion::CuentaListada) -> Self {
        CuentaDto {
            id: c.id,
            nombre: c.nombre,
            tipo: c.tipo,
            moneda: c.moneda,
            saldo_actual: c.saldo_actual,
        }
    }
}

impl From<aplicacion::ListaCuentas> for ListaCuentasDto {
    fn from(l: aplicacion::ListaCuentas) -> Self {
        ListaCuentasDto {
            cuentas: l.cuentas.into_iter().map(CuentaDto::from).collect(),
            subtotales: l
                .subtotales
                .into_iter()
                .map(|s| SubtotalDto {
                    moneda: s.moneda,
                    total: s.total,
                })
                .collect(),
        }
    }
}

impl From<NuevaCuentaDto> for aplicacion::NuevaCuenta {
    fn from(d: NuevaCuentaDto) -> Self {
        aplicacion::NuevaCuenta {
            nombre: d.nombre,
            tipo: d.tipo,
            moneda: d.moneda,
            saldo_inicial: d.saldo_inicial,
        }
    }
}

/// Errores de validación (por campo) o un único mensaje general para fallos
/// internos o de almacenamiento. El detalle técnico nunca llega a la pantalla.
impl From<aplicacion::ErrorAplicacion> for ErroresDto {
    fn from(e: aplicacion::ErrorAplicacion) -> Self {
        let errores = match e {
            aplicacion::ErrorAplicacion::Validacion(lista) => lista
                .into_iter()
                .map(|v| ErrorCampo {
                    campo: v.campo,
                    mensaje: v.mensaje,
                })
                .collect(),
            otro => vec![ErrorCampo {
                campo: None,
                mensaje: otro.to_string(),
            }],
        };
        ErroresDto { errores }
    }
}

// ---------- Entrega 3: conversiones desde y hacia la capa de aplicación ----------

impl From<aplicacion::CategoriaListada> for CategoriaDto {
    fn from(c: aplicacion::CategoriaListada) -> Self {
        CategoriaDto {
            id: c.id,
            nombre: c.nombre,
            tipo: c.tipo,
            de_fabrica: c.de_fabrica,
        }
    }
}

/// El núcleo devuelve el efecto con «+» explícito; el acuerdo del equipo dice que
/// los positivos viajan sin «+», así que solo se quita ese carácter (no se calcula nada).
impl From<aplicacion::MovimientoListado> for MovimientoDto {
    fn from(m: aplicacion::MovimientoListado) -> Self {
        // Mientras el núcleo no entregue el valor vigente, solo se conoce con
        // certeza cuando el movimiento no tiene ajustes: es su propio monto.
        let monto_vigente = if m.estado == "vigente" {
            Some(m.monto.clone())
        } else {
            None
        };
        MovimientoDto {
            id: m.id,
            cuenta_id: m.cuenta_id,
            tipo: m.tipo,
            monto: m.monto,
            monto_con_signo: m.monto_con_signo.trim_start_matches('+').to_string(),
            monto_vigente,
            fecha: m.fecha,
            categoria_id: m.categoria.id,
            categoria_nombre: m.categoria.nombre,
            nota: m.nota,
            estado: m.estado,
            corrige_a: m.corrige_a,
            ajustes: m.ajustes,
            monto_correcto: None,
        }
    }
}

impl From<aplicacion::ComposicionVista> for ComposicionDto {
    fn from(c: aplicacion::ComposicionVista) -> Self {
        ComposicionDto {
            saldo_inicial: c.saldo_inicial,
            total_ingresos: c.total_ingresos,
            total_gastos: c.total_gastos,
            total_ajustes: c.total_ajustes,
        }
    }
}

impl From<aplicacion::DetalleCuentaVista> for DetalleCuentaDto {
    fn from(d: aplicacion::DetalleCuentaVista) -> Self {
        DetalleCuentaDto {
            cuenta: CuentaDto::from(d.cuenta),
            composicion: ComposicionDto::from(d.composicion),
            movimientos: d.movimientos.into_iter().map(MovimientoDto::from).collect(),
        }
    }
}

impl From<aplicacion::PrevisualizacionMovimiento> for PrevisualizacionDto {
    fn from(p: aplicacion::PrevisualizacionMovimiento) -> Self {
        PrevisualizacionDto {
            saldo_resultante: p.saldo_resultante,
            deja_saldo_negativo: p.deja_saldo_negativo,
        }
    }
}

impl From<NuevoMovimientoDto> for aplicacion::NuevoMovimiento {
    fn from(d: NuevoMovimientoDto) -> Self {
        aplicacion::NuevoMovimiento {
            cuenta_id: d.cuenta_id,
            tipo: d.tipo,
            monto: d.monto,
            fecha: d.fecha,
            categoria_id: d.categoria_id,
            nota: if d.nota.trim().is_empty() {
                None
            } else {
                Some(d.nota)
            },
        }
    }
}

impl From<NuevoMovimientoDto> for aplicacion::DatosPrevisualizarMovimiento {
    fn from(d: NuevoMovimientoDto) -> Self {
        aplicacion::DatosPrevisualizarMovimiento {
            cuenta_id: d.cuenta_id,
            tipo: d.tipo,
            monto: d.monto,
        }
    }
}

impl From<CorreccionDto> for aplicacion::DatosCorregirMovimiento {
    fn from(d: CorreccionDto) -> Self {
        aplicacion::DatosCorregirMovimiento {
            movimiento_id: d.movimiento_id,
            monto_correcto: d.monto_correcto,
        }
    }
}

impl From<NuevaCategoriaDto> for aplicacion::NuevaCategoria {
    fn from(d: NuevaCategoriaDto) -> Self {
        aplicacion::NuevaCategoria {
            nombre: d.nombre,
            tipo: d.tipo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn claves(v: &Value) -> Vec<String> {
        let mut c: Vec<String> = v.as_object().expect("objeto").keys().cloned().collect();
        c.sort();
        c
    }

    fn cuenta() -> CuentaDto {
        CuentaDto {
            id: "c1".into(),
            nombre: "Billetera dólares".into(),
            tipo: "banco".into(),
            moneda: "USD".into(),
            saldo_actual: "1750.00".into(),
        }
    }

    fn movimiento() -> MovimientoDto {
        MovimientoDto {
            id: "m1".into(),
            cuenta_id: "c1".into(),
            tipo: "gasto".into(),
            monto: "500.00".into(),
            monto_con_signo: "-500.00".into(),
            monto_vigente: Some("50.00".into()),
            fecha: "2026-10-19".into(),
            categoria_id: "gasto:alimentacion".into(),
            categoria_nombre: "Alimentación".into(),
            nota: String::new(),
            estado: "corregido".into(),
            corrige_a: None,
            ajustes: vec!["m2".into()],
            monto_correcto: None,
        }
    }

    #[test]
    fn movimiento_tiene_los_campos_acordados() {
        let v = serde_json::to_value(movimiento()).unwrap();
        assert_eq!(
            claves(&v),
            vec![
                "ajustes",
                "categoria_id",
                "categoria_nombre",
                "corrige_a",
                "cuenta_id",
                "estado",
                "fecha",
                "id",
                "monto",
                "monto_con_signo",
                "monto_correcto",
                "monto_vigente",
                "nota",
                "tipo",
            ]
        );
        assert_eq!(v["corrige_a"], Value::Null);
        assert_eq!(v["monto_con_signo"], "-500.00");
    }

    #[test]
    fn detalle_de_cuenta_tiene_los_campos_acordados() {
        let detalle = DetalleCuentaDto {
            cuenta: cuenta(),
            composicion: ComposicionDto {
                saldo_inicial: "1000.00".into(),
                total_ingresos: "800.00".into(),
                total_gastos: "500.00".into(),
                total_ajustes: "450.00".into(),
            },
            movimientos: vec![movimiento()],
        };
        let v = serde_json::to_value(detalle).unwrap();
        assert_eq!(claves(&v), vec!["composicion", "cuenta", "movimientos"]);
        assert_eq!(
            claves(&v["composicion"]),
            vec![
                "saldo_inicial",
                "total_ajustes",
                "total_gastos",
                "total_ingresos"
            ]
        );
    }

    #[test]
    fn previsualizacion_tiene_los_campos_acordados() {
        let p = PrevisualizacionDto {
            saldo_resultante: "-50.00".into(),
            deja_saldo_negativo: true,
        };
        let v = serde_json::to_value(p).unwrap();
        assert_eq!(claves(&v), vec!["deja_saldo_negativo", "saldo_resultante"]);
    }

    #[test]
    fn categoria_tiene_los_campos_acordados() {
        let c = CategoriaDto {
            id: "gasto:alimentacion".into(),
            nombre: "Alimentación".into(),
            tipo: "gasto".into(),
            de_fabrica: true,
        };
        let v = serde_json::to_value(c).unwrap();
        assert_eq!(claves(&v), vec!["de_fabrica", "id", "nombre", "tipo"]);
    }

    #[test]
    fn los_datos_de_entrada_ausentes_no_fallan_al_leerse() {
        let m: NuevoMovimientoDto = serde_json::from_value(json!({})).unwrap();
        assert!(m.monto.is_empty() && m.nota.is_empty());
        let c: CorreccionDto = serde_json::from_value(json!({})).unwrap();
        assert!(c.movimiento_id.is_empty());
        let k: NuevaCategoriaDto = serde_json::from_value(json!({"nombre": "Mascotas"})).unwrap();
        assert_eq!(k.nombre, "Mascotas");
        assert!(k.tipo.is_empty());
    }

    #[test]
    fn el_error_no_implementado_va_en_espanol_y_sin_campo() {
        let e = ErroresDto::no_implementado();
        assert_eq!(e.errores.len(), 1);
        assert!(e.errores[0].campo.is_none());
        assert_eq!(
            e.errores[0].mensaje,
            "Esta función todavía no está disponible."
        );
    }
}