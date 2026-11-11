//! División exacta de un gasto compartido (Entrega 6, sección 2.4 del enunciado).
//!
//! Funciones puras, sin red ni base de datos. Toda la aritmética es entera sobre
//! céntimos: el texto se lee con `Decimal` (nunca coma flotante, RNF-08) y se
//! convierte a céntimos exactos. El procedimiento es determinista:
//!
//! 1. Se calcula la parte exacta de cada participante y se trunca a céntimos.
//! 2. Residuo = total menos la suma de las partes truncadas.
//! 3. El residuo se reparte de a un céntimo, en orden alfabético de nombre de
//!    usuario, hasta agotarlo.
//! 4. Cada parte lleva una marca `incluye_redondeo`.
//!
//! Este mismo procedimiento lo repite el servidor, que es quien guarda el resultado.

use std::collections::HashSet;
use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Monto máximo de un gasto: 9.999.999.999,99 (en céntimos).
const MAX_CENTIMOS: i128 = 999_999_999_999;
/// 100 % expresado en centésimas de porcentaje (dos decimales).
const CIEN_POR_CIENTO: i128 = 10_000;

/// Regla de división: "partes_iguales", "porcentajes" o "montos_fijos".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regla {
    PartesIguales,
    Porcentajes,
    MontosFijos,
}

impl Regla {
    pub fn desde_texto(texto: &str) -> Option<Regla> {
        match texto.trim() {
            "partes_iguales" => Some(Regla::PartesIguales),
            "porcentajes" => Some(Regla::Porcentajes),
            "montos_fijos" => Some(Regla::MontosFijos),
            _ => None,
        }
    }
}

/// Un participante. `valor`: el porcentaje ("50") o el monto ("40.00") según la
/// regla; se ignora en partes iguales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participante {
    pub usuario: String,
    pub valor: String,
}

/// Parte de un participante. `monto` va como texto con punto ("33.34").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParteDividida {
    pub usuario: String,
    pub monto: String,
    pub incluye_redondeo: bool,
}

/// Resultado: las partes en orden alfabético de usuario y el residuo repartido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Division {
    pub partes: Vec<ParteDividida>,
    pub residuo: String,
}

/// Error de la división. `campo`: "monto", "regla", "participantes", "porcentajes"
/// o "montos". `no_cuadra` es verdadero cuando los datos son válidos pero la suma
/// no coincide (el mensaje indica la diferencia exacta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDivision {
    pub campo: String,
    pub mensaje: String,
    pub no_cuadra: bool,
}

fn invalido(campo: &str, mensaje: &str) -> ErrorDivision {
    ErrorDivision {
        campo: campo.to_string(),
        mensaje: mensaje.to_string(),
        no_cuadra: false,
    }
}

fn no_cuadra(campo: &str, mensaje: String) -> ErrorDivision {
    ErrorDivision {
        campo: campo.to_string(),
        mensaje,
        no_cuadra: true,
    }
}

/// Lee un número positivo con hasta dos decimales y lo devuelve ×100 (entero exacto).
fn a_centesimas(texto: &str) -> Option<i128> {
    let d = Decimal::from_str(texto.trim()).ok()?.normalize();
    if d <= Decimal::ZERO || d.scale() > 2 {
        return None;
    }
    (d * Decimal::from(100)).to_i128()
}

/// Céntimos como texto con punto: 3334 -> "33.34".
fn con_punto(c: i128) -> String {
    format!("{}.{:02}", c / 100, c % 100)
}

/// Céntimos como texto con coma, para los mensajes: 200 -> "2,00".
fn con_coma(c: i128) -> String {
    format!("{},{:02}", c / 100, c % 100)
}

/// Porcentaje en centésimas como texto con coma, sin ceros de más: 9500 -> "95",
/// 3350 -> "33,5", 3333 -> "33,33".
fn porcentaje_con_coma(c: i128) -> String {
    let (entero, frac) = (c / 100, c % 100);
    if frac == 0 {
        entero.to_string()
    } else if frac % 10 == 0 {
        format!("{entero},{}", frac / 10)
    } else {
        format!("{entero},{frac:02}")
    }
}

fn leer_valores(
    nombres: &[(String, &str)],
    campo: &str,
    que: &str,
) -> Result<Vec<i128>, Vec<ErrorDivision>> {
    let mut errores = Vec::new();
    let mut valores = Vec::new();
    for (usuario, texto) in nombres {
        match a_centesimas(texto) {
            Some(c) => valores.push(c),
            None => errores.push(invalido(
                campo,
                &format!("{que} de {usuario} debe ser mayor que 0 y tener hasta dos decimales."),
            )),
        }
    }
    if errores.is_empty() {
        Ok(valores)
    } else {
        Err(errores)
    }
}

fn faltan_o_sobran(esperado: i128, suma: i128) -> &'static str {
    if suma < esperado { "faltan" } else { "sobran" }
}

/// Divide `monto` entre los participantes según la regla.
///
/// Devuelve las partes en orden alfabético de nombre de usuario (normalizado a
/// minúsculas). Si algo no es válido devuelve todos los errores a la vez; si los
/// datos son válidos pero porcentajes o montos no cuadran, devuelve un único error
/// con `no_cuadra` y la diferencia exacta («Los porcentajes suman 95; faltan 5.»).
pub fn dividir(
    monto: &str,
    regla: &str,
    participantes: &[Participante],
) -> Result<Division, Vec<ErrorDivision>> {
    let mut errores = Vec::new();

    let total = match a_centesimas(monto) {
        Some(c) if c <= MAX_CENTIMOS => Some(c),
        Some(_) => {
            errores.push(invalido("monto", "El monto es demasiado grande."));
            None
        }
        None => {
            errores.push(invalido(
                "monto",
                "El monto debe ser mayor que 0 y tener hasta dos decimales.",
            ));
            None
        }
    };

    let regla = Regla::desde_texto(regla);
    if regla.is_none() {
        errores.push(invalido("regla", "Elige una regla de división válida."));
    }

    if participantes.is_empty() {
        errores.push(invalido("participantes", "Elige al menos un participante."));
    }
    let mut nombres: Vec<(String, &str)> = Vec::new();
    let mut vistos = HashSet::new();
    for p in participantes {
        let usuario = p.usuario.trim().to_lowercase();
        if usuario.is_empty() {
            errores.push(invalido(
                "participantes",
                "Falta el nombre de usuario de un participante.",
            ));
        } else if !vistos.insert(usuario.clone()) {
            errores.push(invalido(
                "participantes",
                &format!("{usuario} está repetido entre los participantes."),
            ));
        } else {
            nombres.push((usuario, p.valor.as_str()));
        }
    }

    let (Some(total), Some(regla)) = (total, regla) else {
        return Err(errores);
    };
    if !errores.is_empty() {
        return Err(errores);
    }

    nombres.sort_by(|a, b| a.0.cmp(&b.0));
    let cantidad = i128::try_from(nombres.len()).unwrap_or(i128::MAX);

    // Parte exacta truncada a céntimos de cada participante.
    let truncadas: Vec<i128> = match regla {
        Regla::PartesIguales => vec![total / cantidad; nombres.len()],
        Regla::Porcentajes => {
            let centesimas = leer_valores(&nombres, "porcentajes", "El porcentaje")?;
            let suma: i128 = centesimas.iter().sum();
            if suma != CIEN_POR_CIENTO {
                return Err(vec![no_cuadra(
                    "porcentajes",
                    format!(
                        "Los porcentajes suman {}; {} {}.",
                        porcentaje_con_coma(suma),
                        faltan_o_sobran(CIEN_POR_CIENTO, suma),
                        porcentaje_con_coma((CIEN_POR_CIENTO - suma).abs()),
                    ),
                )]);
            }
            centesimas
                .iter()
                .map(|c| total * c / CIEN_POR_CIENTO)
                .collect()
        }
        Regla::MontosFijos => {
            let montos = leer_valores(&nombres, "montos", "El monto")?;
            let suma: i128 = montos.iter().sum();
            if suma != total {
                return Err(vec![no_cuadra(
                    "montos",
                    format!(
                        "Los montos suman {}; {} {}.",
                        con_coma(suma),
                        faltan_o_sobran(total, suma),
                        con_coma((total - suma).abs()),
                    ),
                )]);
            }
            montos
        }
    };

    // Residuo: de a un céntimo, en orden alfabético, hasta agotarlo.
    let residuo = total - truncadas.iter().sum::<i128>();
    let mut por_repartir = residuo;
    let partes = nombres
        .iter()
        .zip(truncadas)
        .map(|((usuario, _), base)| {
            let recibe = por_repartir > 0;
            if recibe {
                por_repartir -= 1;
            }
            ParteDividida {
                usuario: usuario.clone(),
                monto: con_punto(base + i128::from(recibe)),
                incluye_redondeo: recibe,
            }
        })
        .collect();

    Ok(Division {
        partes,
        residuo: con_punto(residuo),
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn gente(valores: &[(&str, &str)]) -> Vec<Participante> {
        valores
            .iter()
            .map(|(u, v)| Participante {
                usuario: (*u).to_string(),
                valor: (*v).to_string(),
            })
            .collect()
    }

    fn p(usuario: &str, monto: &str, redondeo: bool) -> (String, String, bool) {
        (usuario.to_string(), monto.to_string(), redondeo)
    }

    fn partes(d: &Division) -> Vec<(String, String, bool)> {
        d.partes
            .iter()
            .map(|p| (p.usuario.clone(), p.monto.clone(), p.incluye_redondeo))
            .collect()
    }

    fn en_centimos(texto: &str) -> i128 {
        let (entero, frac) = texto.split_once('.').expect("texto con punto");
        entero.parse::<i128>().unwrap() * 100 + frac.parse::<i128>().unwrap()
    }

    fn mensajes(e: &[ErrorDivision]) -> Vec<(&str, &str)> {
        e.iter()
            .map(|x| (x.campo.as_str(), x.mensaje.as_str()))
            .collect()
    }

    // ----- Los tres ejemplos del enunciado (sección 2.4) -----

    #[test]
    fn cien_en_partes_iguales_da_33_34_33_33_33_33() {
        let g = gente(&[("ana", ""), ("beto", ""), ("carla", "")]);
        let d = dividir("100.00", "partes_iguales", &g).unwrap();
        assert_eq!(
            partes(&d),
            [
                p("ana", "33.34", true),
                p("beto", "33.33", false),
                p("carla", "33.33", false),
            ]
        );
        assert_eq!(d.residuo, "0.01");
    }

    #[test]
    fn diez_por_porcentajes_da_3_34_3_33_3_33() {
        let g = gente(&[("ana", "33.33"), ("beto", "33.33"), ("carla", "33.34")]);
        let d = dividir("10.00", "porcentajes", &g).unwrap();
        assert_eq!(
            partes(&d),
            [
                p("ana", "3.34", true),
                p("beto", "3.33", false),
                p("carla", "3.33", false),
            ]
        );
        assert_eq!(d.residuo, "0.01");
    }

    #[test]
    fn montos_fijos_que_suman_el_total_no_tienen_redondeo() {
        let g = gente(&[("ana", "50.00"), ("beto", "40.00"), ("carla", "30.00")]);
        let d = dividir("120.00", "montos_fijos", &g).unwrap();
        assert_eq!(
            partes(&d),
            [
                p("ana", "50.00", false),
                p("beto", "40.00", false),
                p("carla", "30.00", false),
            ]
        );
        assert_eq!(d.residuo, "0.00");
    }

    #[test]
    fn el_segundo_ejemplo_de_balances_usa_50_25_25() {
        // beto paga 60,00 por porcentajes (ana 50 %, beto 25 %, carla 25 %).
        let g = gente(&[("ana", "50"), ("beto", "25"), ("carla", "25")]);
        let d = dividir("60.00", "porcentajes", &g).unwrap();
        let montos: Vec<_> = d.partes.iter().map(|p| p.monto.as_str()).collect();
        assert_eq!(montos, ["30.00", "15.00", "15.00"]);
        assert_eq!(d.residuo, "0.00");
    }

    // ----- Diferencia exacta -----

    #[test]
    fn porcentajes_que_suman_95_faltan_5() {
        let g = gente(&[("ana", "50"), ("beto", "25"), ("carla", "20")]);
        let e = dividir("100.00", "porcentajes", &g).unwrap_err();
        assert_eq!(
            mensajes(&e),
            [("porcentajes", "Los porcentajes suman 95; faltan 5.")]
        );
        assert!(e[0].no_cuadra);
    }

    #[test]
    fn porcentajes_que_se_pasan_sobran() {
        let g = gente(&[("ana", "60"), ("beto", "45")]);
        let e = dividir("100.00", "porcentajes", &g).unwrap_err();
        assert_eq!(e[0].mensaje, "Los porcentajes suman 105; sobran 5.");
    }

    #[test]
    fn la_diferencia_de_porcentajes_con_decimales_usa_coma() {
        let g = gente(&[("ana", "50.5"), ("beto", "30.25")]);
        let e = dividir("100.00", "porcentajes", &g).unwrap_err();
        assert_eq!(e[0].mensaje, "Los porcentajes suman 80,75; faltan 19,25.");
    }

    #[test]
    fn montos_que_suman_118_de_120_faltan_2() {
        let g = gente(&[("ana", "58.00"), ("beto", "60.00")]);
        let e = dividir("120.00", "montos_fijos", &g).unwrap_err();
        assert_eq!(
            mensajes(&e),
            [("montos", "Los montos suman 118,00; faltan 2,00.")]
        );
        assert!(e[0].no_cuadra);
    }

    #[test]
    fn montos_que_se_pasan_sobran() {
        let g = gente(&[("ana", "70.00"), ("beto", "52.50")]);
        let e = dividir("120.00", "montos_fijos", &g).unwrap_err();
        assert_eq!(e[0].mensaje, "Los montos suman 122,50; sobran 2,50.");
    }

    // ----- Casos pedidos: residuo, un solo participante, montos grandes -----

    #[test]
    fn el_residuo_de_varios_centimos_va_a_los_primeros_en_orden_alfabetico() {
        // 100,00 entre 7: 14,28 cada uno y sobran 0,04.
        let g = gente(&[
            ("g", ""),
            ("a", ""),
            ("f", ""),
            ("b", ""),
            ("e", ""),
            ("c", ""),
            ("d", ""),
        ]);
        let d = dividir("100.00", "partes_iguales", &g).unwrap();
        let resumen: Vec<_> = d
            .partes
            .iter()
            .map(|p| (p.usuario.as_str(), p.monto.as_str(), p.incluye_redondeo))
            .collect();
        assert_eq!(
            resumen,
            [
                ("a", "14.29", true),
                ("b", "14.29", true),
                ("c", "14.29", true),
                ("d", "14.29", true),
                ("e", "14.28", false),
                ("f", "14.28", false),
                ("g", "14.28", false),
            ]
        );
        assert_eq!(d.residuo, "0.04");
    }

    #[test]
    fn un_solo_participante_se_queda_con_todo() {
        let g = gente(&[("ana", "")]);
        let d = dividir("100.00", "partes_iguales", &g).unwrap();
        assert_eq!(partes(&d), [p("ana", "100.00", false)]);
        let g = gente(&[("ana", "100")]);
        let d = dividir("0.01", "porcentajes", &g).unwrap();
        assert_eq!(partes(&d), [p("ana", "0.01", false)]);
    }

    #[test]
    fn un_centimo_entre_tres_se_lo_lleva_el_primero() {
        let g = gente(&[("carla", ""), ("ana", ""), ("beto", "")]);
        let d = dividir("0.01", "partes_iguales", &g).unwrap();
        assert_eq!(
            partes(&d),
            [
                p("ana", "0.01", true),
                p("beto", "0.00", false),
                p("carla", "0.00", false),
            ]
        );
    }

    #[test]
    fn montos_grandes_suman_exactamente_el_total() {
        let g = gente(&[("ana", ""), ("beto", ""), ("carla", "")]);
        let d = dividir("9999999999.99", "partes_iguales", &g).unwrap();
        let suma: i128 = d.partes.iter().map(|p| en_centimos(&p.monto)).sum();
        assert_eq!(suma, 999_999_999_999);
        let g = gente(&[("ana", "33.33"), ("beto", "33.33"), ("carla", "33.34")]);
        let d = dividir("9999999999.99", "porcentajes", &g).unwrap();
        let suma: i128 = d.partes.iter().map(|p| en_centimos(&p.monto)).sum();
        assert_eq!(suma, 999_999_999_999);
    }

    #[test]
    fn el_resultado_no_depende_del_orden_de_entrada() {
        let a = gente(&[("ana", ""), ("beto", ""), ("carla", "")]);
        let b = gente(&[("carla", ""), ("ana", ""), ("beto", "")]);
        assert_eq!(
            dividir("100.00", "partes_iguales", &a),
            dividir("100.00", "partes_iguales", &b)
        );
    }

    #[test]
    fn los_nombres_se_normalizan_a_minusculas() {
        let g = gente(&[(" Ana ", ""), ("BETO", "")]);
        let d = dividir("10.00", "partes_iguales", &g).unwrap();
        assert_eq!(d.partes[0].usuario, "ana");
        assert_eq!(d.partes[1].usuario, "beto");
    }

    // ----- Datos inválidos -----

    #[test]
    fn montos_invalidos_se_rechazan_en_el_campo_monto() {
        let g = gente(&[("ana", "")]);
        for malo in ["", "0", "-5", "10.001", "abc"] {
            let e = dividir(malo, "partes_iguales", &g).unwrap_err();
            assert_eq!(e[0].campo, "monto", "monto {malo:?}");
            assert!(!e[0].no_cuadra);
        }
        let e = dividir("10000000000.00", "partes_iguales", &g).unwrap_err();
        assert_eq!(e[0].mensaje, "El monto es demasiado grande.");
    }

    #[test]
    fn un_monto_con_ceros_de_mas_es_valido() {
        let g = gente(&[("ana", "")]);
        assert!(dividir("100", "partes_iguales", &g).is_ok());
        assert!(dividir("100.10", "partes_iguales", &g).is_ok());
        assert!(dividir("100.100", "partes_iguales", &g).is_ok());
    }

    #[test]
    fn faltan_participantes_o_estan_repetidos() {
        let e = dividir("10.00", "partes_iguales", &[]).unwrap_err();
        assert_eq!(
            mensajes(&e),
            [("participantes", "Elige al menos un participante.")]
        );
        let g = gente(&[("ana", ""), ("ANA", "")]);
        let e = dividir("10.00", "partes_iguales", &g).unwrap_err();
        assert_eq!(e[0].campo, "participantes");
    }

    #[test]
    fn la_regla_desconocida_se_rechaza() {
        let g = gente(&[("ana", "")]);
        let e = dividir("10.00", "al_azar", &g).unwrap_err();
        assert_eq!(
            mensajes(&e),
            [("regla", "Elige una regla de división válida.")]
        );
    }

    #[test]
    fn varios_errores_llegan_juntos() {
        let e = dividir("0", "al_azar", &[]).unwrap_err();
        let campos: Vec<_> = e.iter().map(|x| x.campo.as_str()).collect();
        assert_eq!(campos, ["monto", "regla", "participantes"]);
    }

    #[test]
    fn un_porcentaje_o_monto_ilegible_nombra_a_la_persona() {
        let g = gente(&[("ana", "50"), ("beto", "mucho")]);
        let e = dividir("10.00", "porcentajes", &g).unwrap_err();
        assert_eq!(e[0].campo, "porcentajes");
        assert!(e[0].mensaje.contains("beto"));
        assert!(!e[0].no_cuadra);
        let g = gente(&[("ana", "5.001"), ("beto", "0")]);
        let e = dividir("10.00", "montos_fijos", &g).unwrap_err();
        assert_eq!(e.len(), 2);
    }

    // ----- Pruebas de propiedades (generador propio, sin dependencias) -----

    struct Azar(u64);

    impl Azar {
        fn siguiente(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
        fn entre(&mut self, minimo: u64, maximo: u64) -> u64 {
            minimo + self.siguiente() % (maximo - minimo + 1)
        }
    }

    fn nombres(n: u64) -> Vec<String> {
        (0..n).map(|i| format!("u{i:02}")).collect()
    }

    /// Reparte `total` en `n` enteros positivos que suman exactamente `total`.
    fn reparto_positivo(azar: &mut Azar, total: u64, n: u64) -> Vec<u64> {
        let pesos: Vec<u64> = (0..n).map(|_| azar.entre(1, 1000)).collect();
        let suma: u64 = pesos.iter().sum();
        let resto = total - n;
        let mut partes: Vec<u64> = pesos
            .iter()
            .take(pesos.len() - 1)
            .map(|p| 1 + (u128::from(resto) * u128::from(*p) / u128::from(suma)) as u64)
            .collect();
        partes.push(total - partes.iter().sum::<u64>());
        partes
    }

    fn comprobar(d: &Division, total: i128, cantidad: i128) {
        let suma: i128 = d.partes.iter().map(|p| en_centimos(&p.monto)).sum();
        assert_eq!(suma, total, "las partes deben sumar el total");
        let residuo = en_centimos(&d.residuo);
        assert!(
            residuo < cantidad,
            "el residuo es menor que los participantes"
        );
        let con_marca = d.partes.iter().filter(|p| p.incluye_redondeo).count();
        assert_eq!(i128::try_from(con_marca).unwrap(), residuo);
    }

    #[test]
    fn propiedad_las_partes_suman_el_total_en_partes_iguales() {
        let mut azar = Azar(7);
        for _ in 0..2000 {
            let n = azar.entre(1, 12);
            let total = azar.entre(1, 999_999_999_999);
            let g: Vec<Participante> = nombres(n)
                .into_iter()
                .map(|u| Participante {
                    usuario: u,
                    valor: String::new(),
                })
                .collect();
            let d = dividir(&con_punto(i128::from(total)), "partes_iguales", &g).unwrap();
            comprobar(&d, i128::from(total), i128::from(n));
        }
    }

    #[test]
    fn propiedad_las_partes_suman_el_total_en_porcentajes() {
        let mut azar = Azar(11);
        for _ in 0..2000 {
            let n = azar.entre(1, 12);
            let total = azar.entre(1, 999_999_999_999);
            let centesimas = reparto_positivo(&mut azar, 10_000, n);
            let g: Vec<Participante> = nombres(n)
                .into_iter()
                .zip(centesimas)
                .map(|(u, c)| Participante {
                    usuario: u,
                    valor: format!("{}.{:02}", c / 100, c % 100),
                })
                .collect();
            let d = dividir(&con_punto(i128::from(total)), "porcentajes", &g).unwrap();
            comprobar(&d, i128::from(total), i128::from(n));
        }
    }

    #[test]
    fn propiedad_los_montos_fijos_que_cuadran_se_aceptan_tal_cual() {
        let mut azar = Azar(13);
        for _ in 0..2000 {
            let n = azar.entre(1, 12);
            let total = azar.entre(n, 999_999_999_999);
            let montos = reparto_positivo(&mut azar, total, n);
            let g: Vec<Participante> = nombres(n)
                .into_iter()
                .zip(&montos)
                .map(|(u, m)| Participante {
                    usuario: u,
                    valor: con_punto(i128::from(*m)),
                })
                .collect();
            let d = dividir(&con_punto(i128::from(total)), "montos_fijos", &g).unwrap();
            assert_eq!(d.residuo, "0.00");
            comprobar(&d, i128::from(total), i128::from(n));
            for (parte, esperado) in d.partes.iter().zip(&montos) {
                assert_eq!(en_centimos(&parte.monto), i128::from(*esperado));
            }
        }
    }
}
