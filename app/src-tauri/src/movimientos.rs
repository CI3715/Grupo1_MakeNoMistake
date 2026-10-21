//! Comandos de categorías, movimientos y detalle de cuenta (Entrega 3).
//!
//! PROVISIONAL: responden con **datos fijos** para que la interfaz pueda probarse
//! contra los comandos reales antes de que existan los casos de uso. No guardan
//! nada ni consultan la base de datos. Las validaciones de aquí solo imitan, con
//! mensajes por campo en español, lo que hará el dominio; se reemplazan cuando se
//! conecten los casos de uso (miércoles).
//!
//! Datos fijos:
//! - `detalle_cuenta` devuelve siempre el ejemplo 2.2 del enunciado («Billetera
//!   dólares», USD): 1.000,00 + 800,00 − 500,00 + 450,00 = 1.750,00, con el gasto
//!   corregido y su ajuste. Usa el `cuenta_id` que reciba.
//! - Movimientos corregibles: `mov-fijo-gasto` (vigente 50.00) y `mov-fijo-ingreso`
//!   (vigente 800.00). `mov-fijo-ajuste` es un ajuste y no se corrige.
//! - `previsualizar_movimiento` parte de un saldo fijo de 1750.00.
//! - Todas las monedas de los datos fijos tienen 2 decimales.
//! - Sin regla de fecha futura todavía (necesita el reloj de los casos de uso).

use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::{
    CategoriaDto, ComposicionDto, CorreccionDto, CuentaDto, DetalleCuentaDto, ErrorCampo,
    ErroresDto, MovimientoDto, NuevaCategoriaDto, NuevoMovimientoDto, PrevisualizacionDto,
};

/// (id, nombre, tipo) de las categorías de fábrica.
const CATEGORIAS: [(&str, &str, &str); 11] = [
    ("gasto:alimentacion", "Alimentación", "gasto"),
    ("gasto:transporte", "Transporte", "gasto"),
    ("gasto:servicios", "Servicios", "gasto"),
    ("gasto:salud", "Salud", "gasto"),
    ("gasto:educacion", "Educación", "gasto"),
    ("gasto:entretenimiento", "Entretenimiento", "gasto"),
    ("gasto:otros", "Otros gastos", "gasto"),
    ("ingreso:salario", "Salario", "ingreso"),
    ("ingreso:ventas", "Ventas", "ingreso"),
    ("ingreso:regalos", "Regalos", "ingreso"),
    ("ingreso:otros", "Otros ingresos", "ingreso"),
];

/// Saldo fijo (en centavos) desde el que se previsualiza: 1750.00.
const SALDO_FIJO: i64 = 175_000;

const AJUSTE_FIJO_ID: &str = "mov-fijo-ajuste";

/// (id, tipo, id de categoría, nombre de categoría, valor vigente en centavos).
const ORIGINALES: [(&str, &str, &str, &str, i64); 2] = [
    (
        "mov-fijo-gasto",
        "gasto",
        "gasto:alimentacion",
        "Alimentación",
        5_000,
    ),
    (
        "mov-fijo-ingreso",
        "ingreso",
        "ingreso:salario",
        "Salario",
        80_000,
    ),
];

// ---------- Ayudas ----------

fn error(campo: Option<&str>, mensaje: impl Into<String>) -> ErrorCampo {
    ErrorCampo {
        campo: campo.map(String::from),
        mensaje: mensaje.into(),
    }
}

fn un_error(campo: Option<&str>, mensaje: impl Into<String>) -> ErroresDto {
    ErroresDto {
        errores: vec![error(campo, mensaje)],
    }
}

fn tipo_valido(tipo: &str) -> Option<&'static str> {
    match tipo.trim() {
        "gasto" => Some("gasto"),
        "ingreso" => Some("ingreso"),
        _ => None,
    }
}

fn buscar_categoria(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    CATEGORIAS.iter().find(|c| c.0 == id).copied()
}

/// Texto con punto decimal y dos decimales a partir de centavos: 5000 -> "50.00".
fn texto_monto(centavos: i64) -> String {
    let signo = if centavos < 0 { "-" } else { "" };
    let absoluto = centavos.unsigned_abs();
    format!("{signo}{}.{:02}", absoluto / 100, absoluto % 100)
}

/// Lee un monto escrito como texto ("50", "50.5", "50.50") y lo devuelve en
/// centavos, con aritmética entera (sin flotantes). Acepta ceros finales
/// ("10.500" vale 10.50) y rechaza más de 2 decimales significativos.
fn analizar_monto(texto: &str, permitir_cero: bool, nombre: &str) -> Result<i64, String> {
    let t = texto.trim();
    if t.is_empty() {
        return Err(format!("{nombre} es obligatorio."));
    }
    if t.starts_with('-') {
        return Err(if permitir_cero {
            format!("{nombre} no puede ser negativo.")
        } else {
            format!("{nombre} debe ser mayor que cero.")
        });
    }
    let invalido = || format!("{nombre} no es un número válido.");
    let solo_digitos = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let (entero, fraccion) = match t.split_once('.') {
        Some((e, f)) => (e, Some(f)),
        None => (t, None),
    };
    if !solo_digitos(entero) || fraccion.is_some_and(|f| !solo_digitos(f)) {
        return Err(invalido());
    }
    let fraccion_significativa = fraccion.unwrap_or("").trim_end_matches('0');
    if fraccion_significativa.len() > 2 {
        return Err(format!(
            "{nombre} tiene más decimales de los que admite la moneda."
        ));
    }
    let entero = entero.trim_start_matches('0');
    if entero.len() > 12 {
        return Err(format!("{nombre} es demasiado grande."));
    }
    let parte_entera: i64 = if entero.is_empty() {
        0
    } else {
        entero.parse().map_err(|_| invalido())?
    };
    let parte_decimal: i64 = format!("{fraccion_significativa:0<2}")
        .parse()
        .map_err(|_| invalido())?;
    let centavos = parte_entera * 100 + parte_decimal;
    if centavos == 0 && !permitir_cero {
        Err(format!("{nombre} debe ser mayor que cero."))
    } else {
        Ok(centavos)
    }
}

fn dias_del_mes(anio: u32, mes: u32) -> u32 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (anio.is_multiple_of(4) && !anio.is_multiple_of(100)) || anio.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
    }
}

/// Comprueba el formato AAAA-MM-DD y que el día exista. No compara con «hoy».
fn validar_fecha(fecha: &str) -> Result<(), String> {
    let f = fecha.trim();
    if f.is_empty() {
        return Err("Indica la fecha.".to_string());
    }
    let invalida = || "La fecha no es válida. Usa el formato AAAA-MM-DD.".to_string();
    let b = f.as_bytes();
    let formato_ok = f.is_ascii()
        && f.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && f.chars()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !formato_ok {
        return Err(invalida());
    }
    let anio: u32 = f[0..4].parse().map_err(|_| invalida())?;
    let mes: u32 = f[5..7].parse().map_err(|_| invalida())?;
    let dia: u32 = f[8..10].parse().map_err(|_| invalida())?;
    if !(1..=12).contains(&mes) || !(1..=dias_del_mes(anio, mes)).contains(&dia) {
        return Err(invalida());
    }
    Ok(())
}

fn validar_categoria(id: &str, tipo: Option<&str>) -> Result<(&'static str, &'static str), String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("Elige una categoría.".to_string());
    }
    let Some((categoria_id, nombre, categoria_tipo)) = buscar_categoria(id) else {
        return Err("La categoría elegida no existe.".to_string());
    };
    if let Some(t) = tipo
        && t != categoria_tipo
    {
        return Err(format!(
            "Esa categoría es de {categoria_tipo}; para un {t} elige una categoría de {t}."
        ));
    }
    Ok((categoria_id, nombre))
}

struct MovimientoValido {
    tipo: &'static str,
    centavos: i64,
    /// (id, nombre)
    categoria: (&'static str, &'static str),
}

fn validar_movimiento(d: &NuevoMovimientoDto) -> Result<MovimientoValido, ErroresDto> {
    let mut errores = Vec::new();

    if d.cuenta_id.trim().is_empty() {
        errores.push(error(Some("cuenta"), "Elige una cuenta."));
    }
    let tipo = tipo_valido(&d.tipo);
    if tipo.is_none() {
        errores.push(error(Some("tipo"), "Elige si es un gasto o un ingreso."));
    }
    let centavos = match analizar_monto(&d.monto, false, "El monto") {
        Ok(c) => Some(c),
        Err(m) => {
            errores.push(error(Some("monto"), m));
            None
        }
    };
    if let Err(m) = validar_fecha(&d.fecha) {
        errores.push(error(Some("fecha"), m));
    }
    let categoria = validar_categoria(&d.categoria_id, tipo);
    if let Err(m) = &categoria {
        errores.push(error(Some("categoria"), m.clone()));
    }

    match (tipo, centavos, categoria) {
        (Some(tipo), Some(centavos), Ok(categoria)) if errores.is_empty() => Ok(MovimientoValido {
            tipo,
            centavos,
            categoria,
        }),
        _ => Err(ErroresDto { errores }),
    }
}

// ---------- Lógica con datos fijos (se prueba sin Tauri) ----------

fn categorias_fijas() -> Vec<CategoriaDto> {
    CATEGORIAS
        .iter()
        .map(|(id, nombre, tipo)| CategoriaDto {
            id: (*id).to_string(),
            nombre: (*nombre).to_string(),
            tipo: (*tipo).to_string(),
            de_fabrica: true,
        })
        .collect()
}

fn crear_categoria_fija(d: &NuevaCategoriaDto) -> Result<CategoriaDto, ErroresDto> {
    let mut errores = Vec::new();
    let nombre = d.nombre.trim();
    let tipo = tipo_valido(&d.tipo);

    if nombre.is_empty() {
        errores.push(error(Some("nombre"), "Escribe el nombre de la categoría."));
    }
    if tipo.is_none() {
        errores.push(error(Some("tipo"), "Elige si es de gasto o de ingreso."));
    }
    if !nombre.is_empty()
        && let Some(t) = tipo
    {
        let repetido = CATEGORIAS
            .iter()
            .any(|(_, n, ct)| *ct == t && n.to_lowercase() == nombre.to_lowercase());
        if repetido {
            errores.push(error(
                Some("nombre"),
                format!("Ya existe una categoría de {t} con ese nombre."),
            ));
        }
    }

    match tipo {
        Some(t) if errores.is_empty() => Ok(CategoriaDto {
            id: format!("{t}:propia-{}", nombre.to_lowercase().replace(' ', "-")),
            nombre: nombre.to_string(),
            tipo: t.to_string(),
            de_fabrica: false,
        }),
        _ => Err(ErroresDto { errores }),
    }
}

fn registrar_fijo(d: &NuevoMovimientoDto) -> Result<MovimientoDto, ErroresDto> {
    let v = validar_movimiento(d)?;
    let con_signo = if v.tipo == "gasto" {
        -v.centavos
    } else {
        v.centavos
    };
    Ok(MovimientoDto {
        id: "mov-fijo-nuevo".to_string(),
        cuenta_id: d.cuenta_id.trim().to_string(),
        tipo: v.tipo.to_string(),
        monto: texto_monto(v.centavos),
        monto_con_signo: texto_monto(con_signo),
        monto_vigente: Some(texto_monto(v.centavos)),
        fecha: d.fecha.trim().to_string(),
        categoria_id: v.categoria.0.to_string(),
        categoria_nombre: v.categoria.1.to_string(),
        nota: d.nota.trim().to_string(),
        estado: "vigente".to_string(),
        corrige_a: None,
        ajustes: Vec::new(),
        monto_correcto: None,
    })
}

fn previsualizar_fijo(d: &NuevoMovimientoDto) -> Result<PrevisualizacionDto, ErroresDto> {
    let v = validar_movimiento(d)?;
    let resultante = if v.tipo == "gasto" {
        SALDO_FIJO - v.centavos
    } else {
        SALDO_FIJO + v.centavos
    };
    Ok(PrevisualizacionDto {
        saldo_resultante: texto_monto(resultante),
        deja_saldo_negativo: resultante < 0,
    })
}

fn corregir_fijo(d: &CorreccionDto) -> Result<MovimientoDto, ErroresDto> {
    let id = d.movimiento_id.trim();
    if id.is_empty() {
        return Err(un_error(None, "Elige el movimiento que quieres corregir."));
    }
    if id == AJUSTE_FIJO_ID {
        return Err(un_error(
            None,
            "Los ajustes no se corrigen: corrige el movimiento original.",
        ));
    }
    let Some(&(_, tipo, categoria_id, categoria_nombre, vigente)) =
        ORIGINALES.iter().find(|o| o.0 == id)
    else {
        return Err(un_error(None, "No encontramos ese movimiento."));
    };
    let correcto = analizar_monto(&d.monto_correcto, true, "El monto correcto")
        .map_err(|m| un_error(Some("monto"), m))?;
    if correcto == vigente {
        return Err(un_error(
            Some("monto"),
            "El monto correcto es igual al valor vigente: no hay nada que ajustar.",
        ));
    }
    // Efecto del ajuste sobre el saldo: positivo si el saldo sube.
    let efecto = if tipo == "gasto" {
        vigente - correcto
    } else {
        correcto - vigente
    };
    Ok(MovimientoDto {
        id: "mov-fijo-ajuste-nuevo".to_string(),
        cuenta_id: "cuenta-fija".to_string(),
        tipo: tipo.to_string(),
        monto: texto_monto(efecto.abs()),
        monto_con_signo: texto_monto(efecto),
        monto_vigente: None,
        fecha: "2026-10-19".to_string(),
        categoria_id: categoria_id.to_string(),
        categoria_nombre: categoria_nombre.to_string(),
        nota: String::new(),
        estado: "ajuste".to_string(),
        corrige_a: Some(id.to_string()),
        ajustes: Vec::new(),
        monto_correcto: Some(texto_monto(correcto)),
    })
}

/// Ejemplo 2.2 del enunciado, del movimiento más reciente al más antiguo.
fn detalle_fijo(cuenta_id: &str) -> Result<DetalleCuentaDto, ErroresDto> {
    let id = cuenta_id.trim();
    if id.is_empty() {
        return Err(un_error(None, "Elige una cuenta."));
    }
    let ajuste = MovimientoDto {
        id: AJUSTE_FIJO_ID.to_string(),
        cuenta_id: id.to_string(),
        tipo: "gasto".to_string(),
        monto: "450.00".to_string(),
        monto_con_signo: "450.00".to_string(),
        monto_vigente: None,
        fecha: "2026-10-14".to_string(),
        categoria_id: "gasto:alimentacion".to_string(),
        categoria_nombre: "Alimentación".to_string(),
        nota: "Corrección: eran 50, no 500".to_string(),
        estado: "ajuste".to_string(),
        corrige_a: Some("mov-fijo-gasto".to_string()),
        ajustes: Vec::new(),
        monto_correcto: Some("50.00".to_string()),
    };
    let gasto = MovimientoDto {
        id: "mov-fijo-gasto".to_string(),
        cuenta_id: id.to_string(),
        tipo: "gasto".to_string(),
        monto: "500.00".to_string(),
        monto_con_signo: "-500.00".to_string(),
        monto_vigente: Some("50.00".to_string()),
        fecha: "2026-10-12".to_string(),
        categoria_id: "gasto:alimentacion".to_string(),
        categoria_nombre: "Alimentación".to_string(),
        nota: "Mercado del sábado".to_string(),
        estado: "corregido".to_string(),
        corrige_a: None,
        ajustes: vec![AJUSTE_FIJO_ID.to_string()],
        monto_correcto: None,
    };
    let ingreso = MovimientoDto {
        id: "mov-fijo-ingreso".to_string(),
        cuenta_id: id.to_string(),
        tipo: "ingreso".to_string(),
        monto: "800.00".to_string(),
        monto_con_signo: "800.00".to_string(),
        monto_vigente: Some("800.00".to_string()),
        fecha: "2026-10-10".to_string(),
        categoria_id: "ingreso:salario".to_string(),
        categoria_nombre: "Salario".to_string(),
        nota: "Pago de octubre".to_string(),
        estado: "vigente".to_string(),
        corrige_a: None,
        ajustes: Vec::new(),
        monto_correcto: None,
    };
    Ok(DetalleCuentaDto {
        cuenta: CuentaDto {
            id: id.to_string(),
            nombre: "Billetera dólares".to_string(),
            tipo: "banco".to_string(),
            moneda: "USD".to_string(),
            saldo_actual: "1750.00".to_string(),
        },
        composicion: ComposicionDto {
            saldo_inicial: "1000.00".to_string(),
            total_ingresos: "800.00".to_string(),
            total_gastos: "500.00".to_string(),
            total_ajustes: "450.00".to_string(),
        },
        movimientos: vec![ajuste, gasto, ingreso],
    })
}

// ---------- Comandos ----------

/// Registra un gasto o un ingreso en una cuenta y devuelve el movimiento.
#[tauri::command]
pub fn registrar_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<MovimientoDto, ErroresDto> {
    registrar_fijo(&datos)
}

/// Corrige un movimiento registrando un ajuste por la diferencia; devuelve el ajuste.
#[tauri::command]
pub fn corregir_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: CorreccionDto,
) -> Result<MovimientoDto, ErroresDto> {
    corregir_fijo(&datos)
}

/// Calcula el saldo que quedaría tras un movimiento, sin guardar nada.
#[tauri::command]
pub fn previsualizar_movimiento(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevoMovimientoDto,
) -> Result<PrevisualizacionDto, ErroresDto> {
    previsualizar_fijo(&datos)
}

/// Lista las categorías de fábrica y las creadas por la persona.
#[tauri::command]
pub fn listar_categorias(
    _estado: State<'_, EstadoCasosDeUso>,
) -> Result<Vec<CategoriaDto>, ErroresDto> {
    Ok(categorias_fijas())
}

/// Crea una categoría propia.
#[tauri::command]
pub fn crear_categoria(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaCategoriaDto,
) -> Result<CategoriaDto, ErroresDto> {
    crear_categoria_fija(&datos)
}

/// Detalle de una cuenta: saldo, composición y movimientos. Se invoca con
/// `invoke('detalle_cuenta', { cuenta_id })`, en snake_case como el resto.
#[tauri::command(rename_all = "snake_case")]
pub fn detalle_cuenta(
    _estado: State<'_, EstadoCasosDeUso>,
    cuenta_id: String,
) -> Result<DetalleCuentaDto, ErroresDto> {
    detalle_fijo(&cuenta_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ErroresDto` no implementa `Debug`, así que `unwrap()` no sirve con él.
    trait EsperarBien<T> {
        fn bien(self) -> T;
    }

    impl<T> EsperarBien<T> for Result<T, ErroresDto> {
        fn bien(self) -> T {
            match self {
                Ok(v) => v,
                Err(e) => {
                    let mensajes: Vec<&str> =
                        e.errores.iter().map(|x| x.mensaje.as_str()).collect();
                    panic!("se esperaba Ok y hubo errores: {mensajes:?}")
                }
            }
        }
    }

    fn mov(tipo: &str, monto: &str, fecha: &str, categoria: &str) -> NuevoMovimientoDto {
        NuevoMovimientoDto {
            cuenta_id: "c1".into(),
            tipo: tipo.into(),
            monto: monto.into(),
            fecha: fecha.into(),
            categoria_id: categoria.into(),
            nota: String::new(),
        }
    }

    fn gasto(monto: &str) -> NuevoMovimientoDto {
        mov("gasto", monto, "2026-10-19", "gasto:alimentacion")
    }

    fn campos(e: &ErroresDto) -> Vec<Option<&str>> {
        e.errores.iter().map(|x| x.campo.as_deref()).collect()
    }

    fn mensaje_de(e: &ErroresDto, campo: &str) -> String {
        e.errores
            .iter()
            .find(|x| x.campo.as_deref() == Some(campo))
            .map(|x| x.mensaje.clone())
            .unwrap_or_default()
    }

    fn correccion(id: &str, correcto: &str) -> CorreccionDto {
        CorreccionDto {
            movimiento_id: id.into(),
            monto_correcto: correcto.into(),
        }
    }

    // ---- categorías ----

    #[test]
    fn hay_siete_categorias_de_gasto_y_cuatro_de_ingreso() {
        let c = categorias_fijas();
        assert_eq!(c.len(), 11);
        assert_eq!(c.iter().filter(|x| x.tipo == "gasto").count(), 7);
        assert_eq!(c.iter().filter(|x| x.tipo == "ingreso").count(), 4);
        assert!(c.iter().all(|x| x.de_fabrica));
    }

    #[test]
    fn crear_categoria_propia_valida() {
        let c = crear_categoria_fija(&NuevaCategoriaDto {
            nombre: "  Mascotas ".into(),
            tipo: "gasto".into(),
        })
        .bien();
        assert_eq!(c.nombre, "Mascotas");
        assert_eq!(c.tipo, "gasto");
        assert!(!c.de_fabrica);
    }

    #[test]
    fn categoria_repetida_en_el_mismo_tipo_se_rechaza_sin_distinguir_mayusculas() {
        let e = crear_categoria_fija(&NuevaCategoriaDto {
            nombre: "alimentación".into(),
            tipo: "gasto".into(),
        })
        .err()
        .unwrap();
        assert_eq!(campos(&e), vec![Some("nombre")]);
    }

    #[test]
    fn el_mismo_nombre_en_otro_tipo_si_se_permite() {
        let c = crear_categoria_fija(&NuevaCategoriaDto {
            nombre: "Alimentación".into(),
            tipo: "ingreso".into(),
        });
        assert!(c.is_ok());
    }

    #[test]
    fn categoria_sin_nombre_ni_tipo_da_dos_errores() {
        let e = crear_categoria_fija(&NuevaCategoriaDto {
            nombre: "   ".into(),
            tipo: String::new(),
        })
        .err()
        .unwrap();
        assert_eq!(campos(&e), vec![Some("nombre"), Some("tipo")]);
    }

    // ---- detalle ----

    #[test]
    fn el_detalle_reproduce_el_ejemplo_2_2() {
        let d = detalle_fijo("cuenta-1").bien();
        assert_eq!(d.cuenta.id, "cuenta-1");
        assert_eq!(d.cuenta.saldo_actual, "1750.00");
        assert_eq!(d.composicion.saldo_inicial, "1000.00");
        assert_eq!(d.composicion.total_ingresos, "800.00");
        assert_eq!(d.composicion.total_gastos, "500.00");
        assert_eq!(d.composicion.total_ajustes, "450.00");
        assert_eq!(d.movimientos.len(), 3);
        assert_eq!(d.movimientos[0].estado, "ajuste");
        assert_eq!(
            d.movimientos[0].corrige_a.as_deref(),
            Some("mov-fijo-gasto")
        );
        assert_eq!(d.movimientos[1].estado, "corregido");
        assert_eq!(d.movimientos[1].ajustes, vec![AJUSTE_FIJO_ID.to_string()]);
        assert_eq!(d.movimientos[2].estado, "vigente");
    }

    #[test]
    fn el_detalle_sin_cuenta_da_un_error_general() {
        let e = detalle_fijo("  ").err().unwrap();
        assert_eq!(campos(&e), vec![None]);
    }

    // ---- registrar ----

    #[test]
    fn un_gasto_valido_lleva_signo_negativo() {
        let m = registrar_fijo(&gasto("50")).bien();
        assert_eq!(m.monto, "50.00");
        assert_eq!(m.monto_con_signo, "-50.00");
        assert_eq!(m.estado, "vigente");
        assert_eq!(m.categoria_nombre, "Alimentación");
    }

    #[test]
    fn un_ingreso_valido_lleva_signo_positivo() {
        let m = registrar_fijo(&mov("ingreso", "800", "2026-10-19", "ingreso:salario")).bien();
        assert_eq!(m.monto_con_signo, "800.00");
    }

    #[test]
    fn ceros_finales_en_el_monto_son_validos() {
        let m = registrar_fijo(&gasto("10.500")).bien();
        assert_eq!(m.monto, "10.50");
    }

    #[test]
    fn monto_cero_negativo_vacio_no_numerico_y_con_decimales_de_mas() {
        let cero = registrar_fijo(&gasto("0")).err().unwrap();
        assert_eq!(
            mensaje_de(&cero, "monto"),
            "El monto debe ser mayor que cero."
        );

        let negativo = registrar_fijo(&gasto("-5")).err().unwrap();
        assert_eq!(
            mensaje_de(&negativo, "monto"),
            "El monto debe ser mayor que cero."
        );

        let vacio = registrar_fijo(&gasto("  ")).err().unwrap();
        assert_eq!(mensaje_de(&vacio, "monto"), "El monto es obligatorio.");

        let letras = registrar_fijo(&gasto("abc")).err().unwrap();
        assert_eq!(
            mensaje_de(&letras, "monto"),
            "El monto no es un número válido."
        );

        let decimales = registrar_fijo(&gasto("10.555")).err().unwrap();
        assert_eq!(
            mensaje_de(&decimales, "monto"),
            "El monto tiene más decimales de los que admite la moneda."
        );
    }

    #[test]
    fn categoria_del_tipo_equivocado_vacia_o_inexistente() {
        let otro_tipo = registrar_fijo(&mov("gasto", "5", "2026-10-19", "ingreso:salario"))
            .err()
            .unwrap();
        assert_eq!(campos(&otro_tipo), vec![Some("categoria")]);
        assert_eq!(
            mensaje_de(&otro_tipo, "categoria"),
            "Esa categoría es de ingreso; para un gasto elige una categoría de gasto."
        );

        let vacia = registrar_fijo(&mov("gasto", "5", "2026-10-19", ""))
            .err()
            .unwrap();
        assert_eq!(mensaje_de(&vacia, "categoria"), "Elige una categoría.");

        let inexistente = registrar_fijo(&mov("gasto", "5", "2026-10-19", "gasto:magia"))
            .err()
            .unwrap();
        assert_eq!(
            mensaje_de(&inexistente, "categoria"),
            "La categoría elegida no existe."
        );
    }

    #[test]
    fn fechas_invalidas_vacias_y_bisiestas() {
        let mala = registrar_fijo(&mov("gasto", "5", "2026-13-40", "gasto:salud"))
            .err()
            .unwrap();
        assert_eq!(campos(&mala), vec![Some("fecha")]);

        let vacia = registrar_fijo(&mov("gasto", "5", "", "gasto:salud"))
            .err()
            .unwrap();
        assert_eq!(mensaje_de(&vacia, "fecha"), "Indica la fecha.");

        assert!(registrar_fijo(&mov("gasto", "5", "2028-02-29", "gasto:salud")).is_ok());
        assert!(registrar_fijo(&mov("gasto", "5", "2027-02-29", "gasto:salud")).is_err());
        assert!(registrar_fijo(&mov("gasto", "5", "19/10/2026", "gasto:salud")).is_err());
    }

    #[test]
    fn varios_errores_llegan_juntos_y_por_campo() {
        let d = NuevoMovimientoDto {
            cuenta_id: String::new(),
            tipo: String::new(),
            monto: "0".into(),
            fecha: "mañana".into(),
            categoria_id: String::new(),
            nota: String::new(),
        };
        let e = registrar_fijo(&d).err().unwrap();
        assert_eq!(
            campos(&e),
            vec![
                Some("cuenta"),
                Some("tipo"),
                Some("monto"),
                Some("fecha"),
                Some("categoria")
            ]
        );
    }

    // ---- previsualizar ----

    #[test]
    fn un_gasto_mayor_que_el_saldo_deja_saldo_negativo() {
        let p = previsualizar_fijo(&gasto("2000")).bien();
        assert_eq!(p.saldo_resultante, "-250.00");
        assert!(p.deja_saldo_negativo);
    }

    #[test]
    fn un_gasto_menor_que_el_saldo_no_advierte() {
        let p = previsualizar_fijo(&gasto("50")).bien();
        assert_eq!(p.saldo_resultante, "1700.00");
        assert!(!p.deja_saldo_negativo);
    }

    #[test]
    fn un_ingreso_suma_al_saldo() {
        let p =
            previsualizar_fijo(&mov("ingreso", "250.50", "2026-10-19", "ingreso:ventas")).bien();
        assert_eq!(p.saldo_resultante, "2000.50");
        assert!(!p.deja_saldo_negativo);
    }

    #[test]
    fn previsualizar_valida_igual_que_registrar() {
        let e = previsualizar_fijo(&gasto("0")).err().unwrap();
        assert_eq!(campos(&e), vec![Some("monto")]);
    }

    // ---- corregir ----

    #[test]
    fn corregir_un_gasto_a_un_monto_menor_sube_el_saldo() {
        let a = corregir_fijo(&correccion("mov-fijo-gasto", "40")).bien();
        assert_eq!(a.estado, "ajuste");
        assert_eq!(a.corrige_a.as_deref(), Some("mov-fijo-gasto"));
        assert_eq!(a.monto_correcto.as_deref(), Some("40.00"));
        assert_eq!(a.monto_con_signo, "10.00");
        assert_eq!(a.monto, "10.00");
    }

    #[test]
    fn corregir_a_cero_anula_el_movimiento_por_completo() {
        let a = corregir_fijo(&correccion("mov-fijo-gasto", "0")).bien();
        assert_eq!(a.monto_con_signo, "50.00");
        let b = corregir_fijo(&correccion("mov-fijo-ingreso", "0")).bien();
        assert_eq!(b.monto_con_signo, "-800.00");
        assert_eq!(b.monto, "800.00");
    }

    #[test]
    fn corregir_un_ingreso_a_un_monto_menor_baja_el_saldo() {
        let a = corregir_fijo(&correccion("mov-fijo-ingreso", "700")).bien();
        assert_eq!(a.monto_con_signo, "-100.00");
    }

    #[test]
    fn corregir_con_el_mismo_valor_vigente_se_rechaza() {
        let e = corregir_fijo(&correccion("mov-fijo-gasto", "50.00"))
            .err()
            .unwrap();
        assert_eq!(campos(&e), vec![Some("monto")]);
    }

    #[test]
    fn corregir_con_monto_negativo_o_vacio_se_rechaza_por_el_campo_monto() {
        let negativo = corregir_fijo(&correccion("mov-fijo-gasto", "-1"))
            .err()
            .unwrap();
        assert_eq!(
            mensaje_de(&negativo, "monto"),
            "El monto correcto no puede ser negativo."
        );
        let vacio = corregir_fijo(&correccion("mov-fijo-gasto", ""))
            .err()
            .unwrap();
        assert_eq!(
            mensaje_de(&vacio, "monto"),
            "El monto correcto es obligatorio."
        );
    }

    #[test]
    fn un_ajuste_no_se_corrige_y_un_movimiento_inexistente_tampoco() {
        let ajuste = corregir_fijo(&correccion(AJUSTE_FIJO_ID, "10"))
            .err()
            .unwrap();
        assert_eq!(campos(&ajuste), vec![None]);
        assert_eq!(
            ajuste.errores[0].mensaje,
            "Los ajustes no se corrigen: corrige el movimiento original."
        );

        let inexistente = corregir_fijo(&correccion("no-existe", "10")).err().unwrap();
        assert_eq!(campos(&inexistente), vec![None]);

        let sin_id = corregir_fijo(&correccion("", "10")).err().unwrap();
        assert_eq!(campos(&sin_id), vec![None]);
    }
}
