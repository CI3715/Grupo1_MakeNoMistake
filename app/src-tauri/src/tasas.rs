//! Comandos de tasas de cambio, moneda base, monedas propias y patrimonio (Entrega 4).
//!
//! PROVISIONAL: responden con **datos fijos** para que la interfaz pueda probarse
//! contra los comandos reales antes de que existan los casos de uso. Los datos viven
//! en memoria mientras la aplicación está abierta: no se guardan ni tocan SQLite.
//! Las validaciones imitan, con mensajes por campo en español, lo que hará el
//! dominio, y se reemplazan al conectar los casos de uso.
//!
//! Datos fijos (ejemplo 2.3 del enunciado):
//! - Cuentas: Efectivo bolívares 88.700,00 VES, Banco en dólares 200,00 USD y
//!   Billetera USDT 50,00 USDT.
//! - Tasas: 1 USD = 887,00 VES (20/10, BCV) y 1 USDT = 1.000,00 VES (21/10, casa de
//!   cambio). La moneda base inicial es USD, así que al empezar falta la tasa
//!   USDT→USD: al registrarla el total pasa a 350,00 USD; con base VES es 316.100,00.
//! - «Hoy» es 2026-10-27 para que las fechas futuras se rechacen de forma estable.
//! - Todas las monedas de los datos fijos usan 2 decimales.

use std::sync::Mutex;

use tauri::State;

use crate::EstadoCasosDeUso;
use crate::dto::{
    CambioMonedaBaseDto, ConsultaPatrimonioDto, ErrorCampo, ErroresDto, HistorialParDto,
    MonedaBaseDto, MonedaDto, NuevaMonedaDto, NuevaTasaDto, ParFaltanteDto, PatrimonioDto,
    SaldoCuentaDto, SubtotalMonedaDto, TasaDto, TasaUsadaDto,
};

const HOY_FIJO: &str = "2026-10-27";

/// (código, nombre, símbolo, decimales) de las monedas de fábrica.
const FABRICA: [(&str, &str, &str, u8); 4] = [
    ("VES", "Bolívar", "Bs.", 2),
    ("USD", "Dólar estadounidense", "$", 2),
    ("EUR", "Euro", "€", 2),
    ("USDT", "Tether (dólar digital)", "USDT", 2),
];

/// (id, nombre, moneda, saldo en centavos) de las cuentas fijas.
const CUENTAS: [(&str, &str, &str, i128); 3] = [
    ("cuenta-fija-ves", "Efectivo bolívares", "VES", 8_870_000),
    ("cuenta-fija-usd", "Banco en dólares", "USD", 20_000),
    ("cuenta-fija-usdt", "Billetera USDT", "USDT", 5_000),
];

// ---------- Estado en memoria de los datos fijos ----------

struct DatosFijos {
    base: String,
    tasas: Vec<TasaDto>,
    /// (código, nombre, símbolo, decimales) de las monedas creadas por la persona.
    propias: Vec<(String, String, String, u8)>,
    contador: u32,
}

fn tasa_fija(
    id: &str,
    origen: &str,
    destino: &str,
    valor: &str,
    fecha: &str,
    fuente: &str,
) -> TasaDto {
    TasaDto {
        id: id.to_string(),
        moneda_origen: origen.to_string(),
        moneda_destino: destino.to_string(),
        valor: valor.to_string(),
        fecha: fecha.to_string(),
        fuente: fuente.to_string(),
    }
}

impl DatosFijos {
    fn inicial() -> Self {
        DatosFijos {
            base: "USD".to_string(),
            tasas: vec![
                tasa_fija("tasa-fija-1", "USD", "VES", "887.00", "2026-10-20", "BCV"),
                tasa_fija(
                    "tasa-fija-2",
                    "USDT",
                    "VES",
                    "1000.00",
                    "2026-10-21",
                    "casa de cambio",
                ),
            ],
            propias: Vec::new(),
            contador: 2,
        }
    }

    fn existe_moneda(&self, codigo: &str) -> bool {
        FABRICA.iter().any(|m| m.0 == codigo) || self.propias.iter().any(|m| m.0 == codigo)
    }
}

struct EstadoFijo(Mutex<Option<DatosFijos>>);

impl EstadoFijo {
    const fn nuevo() -> Self {
        EstadoFijo(Mutex::new(None))
    }

    fn con<R>(&self, f: impl FnOnce(&mut DatosFijos) -> R) -> R {
        let mut guardia = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(guardia.get_or_insert_with(DatosFijos::inicial))
    }
}

static ESTADO_FIJO: EstadoFijo = EstadoFijo::nuevo();

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

/// Lee un decimal positivo escrito como texto y lo devuelve como entero multiplicado
/// por 10^decimales, con aritmética entera (sin flotantes). Acepta ceros finales.
fn a_entero(
    texto: &str,
    decimales: usize,
    nombre: &str,
    permitir_cero: bool,
) -> Result<i128, String> {
    let t = texto.trim();
    if t.is_empty() {
        return Err(format!("{nombre} es obligatorio."));
    }
    if t.starts_with('-') {
        return Err(format!("{nombre} debe ser mayor que cero."));
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
    let fraccion = fraccion.unwrap_or("").trim_end_matches('0');
    if fraccion.len() > decimales {
        return Err(format!("{nombre} tiene más de {decimales} decimales."));
    }
    let entero = entero.trim_start_matches('0');
    if entero.len() > 15 {
        return Err(format!("{nombre} es demasiado grande."));
    }
    let parte_entera: i128 = if entero.is_empty() {
        0
    } else {
        entero.parse().map_err(|_| invalido())?
    };
    let relleno = format!("{fraccion:0<decimales$}");
    let parte_fraccion: i128 = if relleno.is_empty() {
        0
    } else {
        relleno.parse().map_err(|_| invalido())?
    };
    let valor = parte_entera * 10_i128.pow(decimales as u32) + parte_fraccion;
    if valor == 0 && !permitir_cero {
        Err(format!("{nombre} debe ser mayor que cero."))
    } else {
        Ok(valor)
    }
}

/// Texto con punto decimal a partir de un entero escalado: (5000, 2, 2) -> "50.00".
/// Recorta ceros finales pero deja al menos `minimo` decimales.
fn texto_decimal(valor: i128, decimales: usize, minimo: usize) -> String {
    let signo = if valor < 0 { "-" } else { "" };
    let absoluto = valor.unsigned_abs();
    let escala = 10_u128.pow(decimales as u32);
    let entero = absoluto / escala;
    let fraccion = format!("{:0>decimales$}", absoluto % escala);
    let recortada = fraccion.trim_end_matches('0');
    let fraccion = if recortada.len() < minimo {
        &fraccion[..minimo]
    } else {
        recortada
    };
    format!("{signo}{entero}.{fraccion}")
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

/// Comprueba el formato AAAA-MM-DD y que el día exista.
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

// ---------- Tasas ----------

struct TasaValida {
    origen: String,
    destino: String,
    valor: i128,
    fecha: String,
    fuente: String,
}

fn validar_tasa(d: &DatosFijos, e: &NuevaTasaDto) -> Result<TasaValida, ErroresDto> {
    let mut errores = Vec::new();
    let origen = e.moneda_origen.trim();
    let destino = e.moneda_destino.trim();

    if origen.is_empty() {
        errores.push(error(Some("moneda_origen"), "Elige la moneda de origen."));
    } else if !d.existe_moneda(origen) {
        errores.push(error(
            Some("moneda_origen"),
            "La moneda de origen no existe.",
        ));
    }
    if destino.is_empty() {
        errores.push(error(Some("moneda_destino"), "Elige la moneda de destino."));
    } else if !d.existe_moneda(destino) {
        errores.push(error(
            Some("moneda_destino"),
            "La moneda de destino no existe.",
        ));
    } else if origen == destino {
        errores.push(error(
            Some("moneda_destino"),
            "La moneda de destino debe ser distinta de la de origen.",
        ));
    }
    let valor = match a_entero(&e.valor, 8, "El valor", false) {
        Ok(v) => Some(v),
        Err(m) => {
            errores.push(error(Some("valor"), m));
            None
        }
    };
    let fecha = e.fecha.trim();
    match validar_fecha(fecha) {
        Err(m) => errores.push(error(Some("fecha"), m)),
        Ok(()) if fecha > HOY_FIJO => {
            errores.push(error(Some("fecha"), "La fecha no puede ser futura."));
        }
        Ok(()) => {}
    }
    let fuente = e.fuente.trim();
    if fuente.is_empty() {
        errores.push(error(Some("fuente"), "Indica la fuente de la tasa."));
    }

    match valor {
        Some(valor) if errores.is_empty() => Ok(TasaValida {
            origen: origen.to_string(),
            destino: destino.to_string(),
            valor,
            fecha: fecha.to_string(),
            fuente: fuente.to_string(),
        }),
        _ => Err(ErroresDto { errores }),
    }
}

fn registrar_tasa_fija(estado: &EstadoFijo, e: &NuevaTasaDto) -> Result<TasaDto, ErroresDto> {
    estado.con(|d| -> Result<TasaDto, ErroresDto> {
        let v = validar_tasa(d, e)?;
        d.contador += 1;
        let tasa = TasaDto {
            id: format!("tasa-fija-{}", d.contador),
            moneda_origen: v.origen,
            moneda_destino: v.destino,
            valor: texto_decimal(v.valor, 8, 2),
            fecha: v.fecha,
            fuente: v.fuente,
        };
        d.tasas.push(tasa.clone());
        Ok(tasa)
    })
}

fn eliminar_tasa_fija(estado: &EstadoFijo, id: &str) -> Result<(), ErroresDto> {
    let id = id.trim();
    if id.is_empty() {
        return Err(un_error(None, "Elige la tasa que quieres eliminar."));
    }
    estado.con(|d| match d.tasas.iter().position(|t| t.id == id) {
        Some(posicion) => {
            d.tasas.remove(posicion);
            Ok(())
        }
        None => Err(un_error(None, "No encontramos esa tasa.")),
    })
}

/// Historial por par, de la tasa más reciente a la más antigua (fecha y luego orden
/// de registro).
fn listar_tasas_fijas(estado: &EstadoFijo) -> Vec<HistorialParDto> {
    estado.con(|d| {
        let mut pares: Vec<(String, String)> = Vec::new();
        for t in &d.tasas {
            let par = (t.moneda_origen.clone(), t.moneda_destino.clone());
            if !pares.contains(&par) {
                pares.push(par);
            }
        }
        pares
            .into_iter()
            .map(|(origen, destino)| {
                let mut tasas: Vec<(usize, &TasaDto)> = d
                    .tasas
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.moneda_origen == origen && t.moneda_destino == destino)
                    .collect();
                tasas.sort_by(|a, b| (&b.1.fecha, b.0).cmp(&(&a.1.fecha, a.0)));
                HistorialParDto {
                    moneda_origen: origen,
                    moneda_destino: destino,
                    tasas: tasas.into_iter().map(|(_, t)| t.clone()).collect(),
                }
            })
            .collect()
    })
}

// ---------- Moneda base y monedas propias ----------

fn cambiar_base_fija(
    estado: &EstadoFijo,
    e: &CambioMonedaBaseDto,
) -> Result<MonedaBaseDto, ErroresDto> {
    let moneda = e.moneda.trim();
    if moneda.is_empty() {
        return Err(un_error(Some("moneda"), "Elige la moneda base."));
    }
    estado.con(|d| {
        if !d.existe_moneda(moneda) {
            return Err(un_error(Some("moneda"), "La moneda elegida no existe."));
        }
        d.base = moneda.to_string();
        Ok(MonedaBaseDto {
            moneda: moneda.to_string(),
        })
    })
}

fn crear_moneda_fija(estado: &EstadoFijo, e: &NuevaMonedaDto) -> Result<MonedaDto, ErroresDto> {
    let mut errores = Vec::new();
    let codigo = e.codigo.trim();
    let nombre = e.nombre.trim();
    let simbolo = e.simbolo.trim();
    let decimales_texto = e.decimales.trim();

    let codigo_ok =
        (3..=5).contains(&codigo.len()) && codigo.chars().all(|c| c.is_ascii_uppercase());
    if !codigo_ok {
        errores.push(error(
            Some("codigo"),
            "El código debe tener de 3 a 5 letras mayúsculas.",
        ));
    }
    if nombre.is_empty() {
        errores.push(error(Some("nombre"), "Escribe el nombre de la moneda."));
    }
    if simbolo.is_empty() {
        errores.push(error(Some("simbolo"), "Escribe el símbolo de la moneda."));
    }
    let decimales = match decimales_texto.parse::<u8>() {
        Ok(n) if n <= 8 => Some(n),
        _ if decimales_texto.is_empty() => {
            errores.push(error(
                Some("decimales"),
                "Indica cuántos decimales usa la moneda.",
            ));
            None
        }
        _ => {
            errores.push(error(
                Some("decimales"),
                "Los decimales deben ser un número entero de 0 a 8.",
            ));
            None
        }
    };

    estado.con(|d| {
        if codigo_ok && d.existe_moneda(codigo) {
            errores.push(error(
                Some("codigo"),
                "Ya existe una moneda con ese código.",
            ));
        }
        match decimales {
            Some(decimales) if errores.is_empty() => {
                d.propias.push((
                    codigo.to_string(),
                    nombre.to_string(),
                    simbolo.to_string(),
                    decimales,
                ));
                Ok(MonedaDto {
                    codigo: codigo.to_string(),
                    nombre: nombre.to_string(),
                    simbolo: simbolo.to_string(),
                    decimales,
                })
            }
            _ => Err(ErroresDto {
                errores: std::mem::take(&mut errores),
            }),
        }
    })
}

// ---------- Patrimonio ----------

/// Tasa más reciente del par con fecha igual o anterior a `fecha`; en la misma fecha
/// gana la registrada más recientemente.
fn buscar_tasa<'a>(
    tasas: &'a [TasaDto],
    origen: &str,
    destino: &str,
    fecha: &str,
) -> Option<&'a TasaDto> {
    tasas
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.moneda_origen == origen && t.moneda_destino == destino && t.fecha.as_str() <= fecha
        })
        .max_by(|a, b| (&a.1.fecha, a.0).cmp(&(&b.1.fecha, b.0)))
        .map(|(_, t)| t)
}

fn redondear(numerador: i128, denominador: i128) -> i128 {
    if denominador == 0 {
        return 0;
    }
    (numerador * 2 + denominador) / (denominador * 2)
}

/// Convierte centavos con el valor de una tasa (entero por 10^8), o con su inversa.
fn convertir(centavos: i128, valor: i128, inversa: bool) -> i128 {
    const E8: i128 = 100_000_000;
    if inversa {
        redondear(centavos * E8, valor)
    } else {
        redondear(centavos * valor, E8)
    }
}

fn consultar_patrimonio_fijo(
    estado: &EstadoFijo,
    e: &ConsultaPatrimonioDto,
) -> Result<PatrimonioDto, ErroresDto> {
    let fecha = if e.fecha.trim().is_empty() {
        HOY_FIJO.to_string()
    } else {
        e.fecha.trim().to_string()
    };
    validar_fecha(&fecha).map_err(|m| un_error(Some("fecha"), m))?;
    if fecha.as_str() > HOY_FIJO {
        return Err(un_error(
            Some("fecha"),
            "La fecha de consulta no puede ser futura.",
        ));
    }

    Ok(estado.con(|d| {
        let mut cuentas = Vec::new();
        let mut subtotales: Vec<(String, i128)> = Vec::new();
        let mut usadas: Vec<TasaUsadaDto> = Vec::new();
        let mut faltantes: Vec<ParFaltanteDto> = Vec::new();
        let mut total: i128 = 0;

        for (id, nombre, moneda, centavos) in CUENTAS {
            match subtotales.iter_mut().find(|s| s.0 == moneda) {
                Some(s) => s.1 += centavos,
                None => subtotales.push((moneda.to_string(), centavos)),
            }

            let (en_base, uso) = if moneda == d.base {
                (Some(centavos), None)
            } else if let Some(t) = buscar_tasa(&d.tasas, moneda, &d.base, &fecha) {
                let valor = a_entero(&t.valor, 8, "", false).unwrap_or(0);
                (Some(convertir(centavos, valor, false)), Some((t, false)))
            } else if let Some(t) = buscar_tasa(&d.tasas, &d.base, moneda, &fecha) {
                let valor = a_entero(&t.valor, 8, "", false).unwrap_or(0);
                (Some(convertir(centavos, valor, true)), Some((t, true)))
            } else {
                (None, None)
            };

            match en_base {
                Some(c) => total += c,
                None => {
                    let ya = faltantes
                        .iter()
                        .any(|f| f.moneda_origen == moneda && f.moneda_destino == d.base);
                    if !ya {
                        faltantes.push(ParFaltanteDto {
                            moneda_origen: moneda.to_string(),
                            moneda_destino: d.base.clone(),
                        });
                    }
                }
            }
            if let Some((t, inversa)) = uso {
                let ya = usadas.iter().any(|u| {
                    u.moneda_origen == t.moneda_origen
                        && u.moneda_destino == t.moneda_destino
                        && u.fecha == t.fecha
                        && u.valor == t.valor
                });
                if !ya {
                    usadas.push(TasaUsadaDto {
                        moneda_origen: t.moneda_origen.clone(),
                        moneda_destino: t.moneda_destino.clone(),
                        valor: t.valor.clone(),
                        fecha: t.fecha.clone(),
                        fuente: t.fuente.clone(),
                        inversa,
                    });
                }
            }
            cuentas.push(SaldoCuentaDto {
                id: id.to_string(),
                nombre: nombre.to_string(),
                moneda: moneda.to_string(),
                saldo: texto_decimal(centavos, 2, 2),
                en_moneda_base: en_base.map(|c| texto_decimal(c, 2, 2)),
            });
        }

        PatrimonioDto {
            moneda_base: d.base.clone(),
            fecha_consulta: fecha.clone(),
            cuentas,
            subtotales: subtotales
                .into_iter()
                .map(|(moneda, c)| SubtotalMonedaDto {
                    moneda,
                    total: texto_decimal(c, 2, 2),
                })
                .collect(),
            total: if faltantes.is_empty() {
                Some(texto_decimal(total, 2, 2))
            } else {
                None
            },
            tasas_usadas: usadas,
            faltantes,
        }
    }))
}

// ---------- Comandos ----------

/// Registra una tasa de cambio (1 origen = valor destino) y devuelve la tasa guardada.
#[tauri::command]
pub fn registrar_tasa(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaTasaDto,
) -> Result<TasaDto, ErroresDto> {
    registrar_tasa_fija(&ESTADO_FIJO, &datos)
}

/// Elimina una tasa registrada por error (en el libro real es un evento nuevo).
/// Se invoca con `invoke('eliminar_tasa', { tasa_id })`, en snake_case como el resto.
#[tauri::command(rename_all = "snake_case")]
pub fn eliminar_tasa(
    _estado: State<'_, EstadoCasosDeUso>,
    tasa_id: String,
) -> Result<(), ErroresDto> {
    eliminar_tasa_fija(&ESTADO_FIJO, &tasa_id)
}

/// Historial de cada par de monedas, con valor, fecha y fuente.
#[tauri::command]
pub fn listar_tasas(
    _estado: State<'_, EstadoCasosDeUso>,
) -> Result<Vec<HistorialParDto>, ErroresDto> {
    Ok(listar_tasas_fijas(&ESTADO_FIJO))
}

/// Cambia la moneda base en la que se presenta el total consolidado.
#[tauri::command]
pub fn cambiar_moneda_base(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: CambioMonedaBaseDto,
) -> Result<MonedaBaseDto, ErroresDto> {
    cambiar_base_fija(&ESTADO_FIJO, &datos)
}

/// Crea una moneda propia (código de 3 a 5 mayúsculas, nombre, símbolo y decimales).
#[tauri::command]
pub fn crear_moneda(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: NuevaMonedaDto,
) -> Result<MonedaDto, ErroresDto> {
    crear_moneda_fija(&ESTADO_FIJO, &datos)
}

/// Patrimonio a una fecha ("" = hoy): saldos, subtotales por moneda y total
/// consolidado en la moneda base, o los pares de tasas que faltan.
#[tauri::command]
pub fn consultar_patrimonio(
    _estado: State<'_, EstadoCasosDeUso>,
    datos: ConsultaPatrimonioDto,
) -> Result<PatrimonioDto, ErroresDto> {
    consultar_patrimonio_fijo(&ESTADO_FIJO, &datos)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ErroresDto` no implementa `Debug`, así que `unwrap()` no sirve con él.
    fn bien<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => {
                let mensajes: Vec<&str> = e.errores.iter().map(|x| x.mensaje.as_str()).collect();
                panic!("se esperaba Ok y hubo errores: {mensajes:?}")
            }
        }
    }

    fn campos(e: &ErroresDto) -> Vec<Option<&str>> {
        e.errores.iter().map(|x| x.campo.as_deref()).collect()
    }

    fn nueva_tasa(
        origen: &str,
        destino: &str,
        valor: &str,
        fecha: &str,
        fuente: &str,
    ) -> NuevaTasaDto {
        NuevaTasaDto {
            moneda_origen: origen.into(),
            moneda_destino: destino.into(),
            valor: valor.into(),
            fecha: fecha.into(),
            fuente: fuente.into(),
        }
    }

    fn consulta(fecha: &str) -> ConsultaPatrimonioDto {
        ConsultaPatrimonioDto {
            fecha: fecha.into(),
        }
    }

    fn cambiar_base(e: &EstadoFijo, moneda: &str) {
        bien(cambiar_base_fija(
            e,
            &CambioMonedaBaseDto {
                moneda: moneda.into(),
            },
        ));
    }

    #[test]
    fn el_ejemplo_2_3_se_reproduce_de_punta_a_punta() {
        let e = EstadoFijo::nuevo();

        // Base USD (por defecto): falta la tasa USDT -> USD, así que no hay total.
        let p = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(p.moneda_base, "USD");
        assert_eq!(p.total, None);
        assert_eq!(p.faltantes.len(), 1);
        assert_eq!(p.faltantes[0].moneda_origen, "USDT");
        assert_eq!(p.faltantes[0].moneda_destino, "USD");
        assert_eq!(p.cuentas[0].en_moneda_base.as_deref(), Some("100.00"));
        assert_eq!(p.cuentas[2].en_moneda_base, None);
        assert_eq!(p.subtotales.len(), 3);

        // Al registrar 1 USDT = 1,00 USD el total es 350,00 USD.
        bien(registrar_tasa_fija(
            &e,
            &nueva_tasa("USDT", "USD", "1", "2026-10-23", "estimación propia"),
        ));
        let p = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(p.total.as_deref(), Some("350.00"));
        assert!(p.faltantes.is_empty());
        assert_eq!(p.tasas_usadas.len(), 2);
        assert!(p.tasas_usadas.iter().any(|t| t.inversa));

        // Con base VES el total es 316.100,00 y todas las tasas son directas.
        cambiar_base(&e, "VES");
        let p = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(p.total.as_deref(), Some("316100.00"));
        assert_eq!(p.cuentas[1].en_moneda_base.as_deref(), Some("177400.00"));
        assert_eq!(p.cuentas[2].en_moneda_base.as_deref(), Some("50000.00"));
        assert!(p.tasas_usadas.iter().all(|t| !t.inversa));
    }

    #[test]
    fn cambiar_la_base_no_cambia_los_saldos() {
        let e = EstadoFijo::nuevo();
        let antes: Vec<String> = bien(consultar_patrimonio_fijo(&e, &consulta("")))
            .cuentas
            .into_iter()
            .map(|c| c.saldo)
            .collect();
        cambiar_base(&e, "VES");
        let despues: Vec<String> = bien(consultar_patrimonio_fijo(&e, &consulta("")))
            .cuentas
            .into_iter()
            .map(|c| c.saldo)
            .collect();
        assert_eq!(antes, despues);
    }

    #[test]
    fn eliminar_una_tasa_necesaria_deja_de_consolidar_y_nombra_el_par() {
        let e = EstadoFijo::nuevo();
        cambiar_base(&e, "VES");
        assert!(
            bien(consultar_patrimonio_fijo(&e, &consulta("")))
                .total
                .is_some()
        );

        bien(eliminar_tasa_fija(&e, "tasa-fija-2"));
        let p = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(p.total, None);
        assert_eq!(p.faltantes[0].moneda_origen, "USDT");
        assert_eq!(p.faltantes[0].moneda_destino, "VES");

        assert!(eliminar_tasa_fija(&e, "tasa-fija-2").is_err());
        assert!(eliminar_tasa_fija(&e, "  ").is_err());
    }

    #[test]
    fn varias_tasas_del_mismo_par_y_el_patrimonio_usa_la_vigente_en_la_fecha() {
        let e = EstadoFijo::nuevo();
        bien(registrar_tasa_fija(
            &e,
            &nueva_tasa("USD", "VES", "900", "2026-10-25", "BCV"),
        ));
        cambiar_base(&e, "VES");

        let historial = listar_tasas_fijas(&e);
        let par = historial
            .iter()
            .find(|h| h.moneda_origen == "USD" && h.moneda_destino == "VES")
            .expect("par USD a VES");
        assert_eq!(par.tasas.len(), 2);
        assert_eq!(par.tasas[0].fecha, "2026-10-25");
        assert_eq!(par.tasas[0].valor, "900.00");

        let antes = bien(consultar_patrimonio_fijo(&e, &consulta("2026-10-22")));
        assert_eq!(
            antes.cuentas[1].en_moneda_base.as_deref(),
            Some("177400.00")
        );
        let hoy = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(hoy.cuentas[1].en_moneda_base.as_deref(), Some("180000.00"));
    }

    #[test]
    fn en_la_misma_fecha_gana_la_tasa_registrada_mas_recientemente() {
        let e = EstadoFijo::nuevo();
        bien(registrar_tasa_fija(
            &e,
            &nueva_tasa("USD", "VES", "950", "2026-10-20", "casa de cambio"),
        ));
        cambiar_base(&e, "VES");
        let p = bien(consultar_patrimonio_fijo(&e, &consulta("")));
        assert_eq!(p.cuentas[1].en_moneda_base.as_deref(), Some("190000.00"));
    }

    #[test]
    fn una_tasa_invalida_se_rechaza_por_campo() {
        let e = EstadoFijo::nuevo();
        let casos = [
            (
                nueva_tasa("USD", "USD", "1", "2026-10-20", "BCV"),
                "moneda_destino",
            ),
            (nueva_tasa("USD", "VES", "0", "2026-10-20", "BCV"), "valor"),
            (nueva_tasa("USD", "VES", "-5", "2026-10-20", "BCV"), "valor"),
            (
                nueva_tasa("USD", "VES", "1.123456789", "2026-10-20", "BCV"),
                "valor",
            ),
            (
                nueva_tasa("USD", "VES", "887", "2026-12-01", "BCV"),
                "fecha",
            ),
            (nueva_tasa("USD", "VES", "887", "", "BCV"), "fecha"),
            (
                nueva_tasa("USD", "VES", "887", "2026-10-20", "  "),
                "fuente",
            ),
            (
                nueva_tasa("XXX", "VES", "887", "2026-10-20", "BCV"),
                "moneda_origen",
            ),
        ];
        for (datos, campo) in casos {
            let err = registrar_tasa_fija(&e, &datos).err().expect("debía fallar");
            assert_eq!(campos(&err), vec![Some(campo)], "caso {campo}");
        }
        // Hasta 8 decimales es válido.
        assert!(
            registrar_tasa_fija(
                &e,
                &nueva_tasa("USD", "EUR", "0.12345678", "2026-10-20", "BCV")
            )
            .is_ok()
        );
    }

    #[test]
    fn una_moneda_propia_se_crea_y_se_puede_usar_en_una_tasa() {
        let e = EstadoFijo::nuevo();
        let m = bien(crear_moneda_fija(
            &e,
            &NuevaMonedaDto {
                codigo: "BTC".into(),
                nombre: "Bitcoin".into(),
                simbolo: "₿".into(),
                decimales: "8".into(),
            },
        ));
        assert_eq!(m.codigo, "BTC");
        assert_eq!(m.decimales, 8);
        assert!(
            registrar_tasa_fija(
                &e,
                &nueva_tasa("BTC", "USD", "65000", "2026-10-20", "estimación propia")
            )
            .is_ok()
        );
        cambiar_base(&e, "BTC");
    }

    #[test]
    fn una_moneda_invalida_o_repetida_se_rechaza_por_campo() {
        let e = EstadoFijo::nuevo();
        let datos = |codigo: &str, nombre: &str, simbolo: &str, decimales: &str| NuevaMonedaDto {
            codigo: codigo.into(),
            nombre: nombre.into(),
            simbolo: simbolo.into(),
            decimales: decimales.into(),
        };
        let casos = [
            (datos("usd1", "X", "x", "2"), vec![Some("codigo")]),
            (datos("AB", "X", "x", "2"), vec![Some("codigo")]),
            (datos("USD", "X", "x", "2"), vec![Some("codigo")]),
            (datos("ZZZ", "", "x", "2"), vec![Some("nombre")]),
            (datos("ZZZ", "X", "", "2"), vec![Some("simbolo")]),
            (datos("ZZZ", "X", "x", "9"), vec![Some("decimales")]),
            (datos("ZZZ", "X", "x", ""), vec![Some("decimales")]),
            (datos("ZZZ", "X", "x", "dos"), vec![Some("decimales")]),
            (
                datos("", "", "", ""),
                vec![
                    Some("codigo"),
                    Some("nombre"),
                    Some("simbolo"),
                    Some("decimales"),
                ],
            ),
        ];
        for (d, esperado) in casos {
            let err = crear_moneda_fija(&e, &d).err().expect("debía fallar");
            assert_eq!(campos(&err), esperado, "código {}", d.codigo);
        }
    }

    #[test]
    fn la_moneda_base_debe_existir() {
        let e = EstadoFijo::nuevo();
        let err = cambiar_base_fija(
            &e,
            &CambioMonedaBaseDto {
                moneda: "QQQ".into(),
            },
        )
        .err()
        .expect("debía fallar");
        assert_eq!(campos(&err), vec![Some("moneda")]);
        let vacia = cambiar_base_fija(
            &e,
            &CambioMonedaBaseDto {
                moneda: String::new(),
            },
        )
        .err()
        .expect("debía fallar");
        assert_eq!(campos(&vacia), vec![Some("moneda")]);
    }

    #[test]
    fn la_fecha_de_consulta_no_puede_ser_futura_ni_invalida() {
        let e = EstadoFijo::nuevo();
        for fecha in ["2030-01-01", "ayer", "2026-02-30"] {
            let err = consultar_patrimonio_fijo(&e, &consulta(fecha))
                .err()
                .expect("debía fallar");
            assert_eq!(campos(&err), vec![Some("fecha")], "fecha {fecha}");
        }
    }

    #[test]
    fn los_datos_tienen_los_campos_acordados() {
        let e = EstadoFijo::nuevo();
        let v = serde_json::to_value(bien(consultar_patrimonio_fijo(&e, &consulta("")))).unwrap();
        let mut claves: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        claves.sort_unstable();
        assert_eq!(
            claves,
            vec![
                "cuentas",
                "faltantes",
                "fecha_consulta",
                "moneda_base",
                "subtotales",
                "tasas_usadas",
                "total"
            ]
        );
        assert!(v["total"].is_null());
    }
}
