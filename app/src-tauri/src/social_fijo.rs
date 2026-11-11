//! Datos fijos de amistades, grupos y gastos compartidos (Entrega 6).
//!
//! VERSIÓN SIN SERVIDOR: un simulador en memoria para construir y probar la interfaz.
//! Parte de datos de ejemplo y reacciona a lo que se hace (invitar, aceptar, crear
//! grupos, registrar gastos con la división real del dominio, ver balances). Al
//! cerrar la aplicación todo vuelve al estado inicial. Los comandos reales lo
//! reemplazan.
//!
//! La persona con sesión es «ana». Estado inicial:
//!   Amigos:       beto, carla, luis. Invitación recibida pendiente: dario.
//!   Grupos:       «Viaje a Mérida» (g1, USD; ana administra; con beto y carla y dos
//!                 gastos: balances +36,66 · +11,67 · −48,33) y «Cena del viernes»
//!                 (g3, USD; la administra beto, ana es miembro: no puede invitar).
//!   Invitación a un grupo pendiente: «Cumpleaños de Luis» (g2), de beto.
//!   Personas que existen: ana, beto, carla, dario, luis. Cualquier otra: no existe.
//!
//! Nombres de usuario (o ids) de prueba que disparan cada caso, en cualquier campo
//! de texto que identifique a alguien, un grupo o una amistad:
//!   sin-conexion     servidor apagado
//!   sesion-vencida   token vencido (401)
//!   sin-sesion       no se ha iniciado sesión
//! Además: «ana» como invitado da «No puedes invitarte a ti mismo.»; invitar a quien
//! ya es amigo da 409; «dario» no es amigo (403 al invitarlo a un grupo); ver el
//! grupo «g2» (solo invitado) o «g99» (no existe) da el mismo 404.

use std::sync::Mutex;

use dominio::division::{self, Participante};

use crate::dto::{ErrorCampo, ErroresDto};
use crate::dto_social::{
    AmigoDto, BalanceDto, BuscarUsuarioDto, GastoGrupoDto, GrupoDetalleDto, GrupoResumenDto,
    InvitacionAmistadDto, InvitacionGrupoDto, InvitadoDto, InvitarAlGrupoDto, InvitarAmigoDto,
    ListaAmistadesDto, ListaGruposDto, MiembroDto, NuevoGastoGrupoDto, NuevoGrupoDto, ParteDto,
    ResponderAmistadDto, ResponderInvitacionGrupoDto, UsuarioPublicoDto,
};

const YO: &str = "ana";
const NO_ENCONTRADO: &str = "No hay nadie con ese nombre de usuario.";

// ---------- Estado ----------

struct GrupoFijo {
    resumen: GrupoResumenDto,
    miembros: Vec<MiembroDto>,
    invitados: Vec<InvitadoDto>,
    gastos: Vec<GastoGrupoDto>,
}

pub struct Estado {
    amigos: Vec<AmigoDto>,
    enviadas: Vec<InvitacionAmistadDto>,
    recibidas: Vec<InvitacionAmistadDto>,
    grupos: Vec<GrupoFijo>,
    invitaciones: Vec<InvitacionGrupoDto>,
    contador: u32,
}

static ESTADO: Mutex<Option<Estado>> = Mutex::new(None);

/// Ejecuta `f` sobre el estado compartido (lo crea en el primer uso).
pub fn con_estado<T>(f: impl FnOnce(&mut Estado) -> T) -> T {
    let mut guardia = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
    f(guardia.get_or_insert_with(Estado::inicial))
}

fn visible(usuario: &str) -> Option<&'static str> {
    match usuario {
        "ana" => Some("Ana Pérez"),
        "beto" => Some("Beto Gómez"),
        "carla" => Some("Carla Ruiz"),
        "dario" => Some("Darío Mena"),
        "luis" => Some("Luis Soto"),
        _ => None,
    }
}

fn publico(usuario: &str) -> UsuarioPublicoDto {
    UsuarioPublicoDto {
        nombre_usuario: usuario.to_string(),
        nombre_visible: visible(usuario).unwrap_or(usuario).to_string(),
    }
}

fn amigo(id: &str, usuario: &str) -> AmigoDto {
    AmigoDto {
        amistad_id: id.to_string(),
        usuario: publico(usuario),
    }
}

fn parte(usuario: &str, monto: &str, redondeo: bool) -> ParteDto {
    ParteDto {
        usuario: usuario.to_string(),
        monto: monto.to_string(),
        incluye_redondeo: redondeo,
    }
}

fn grupo(
    id: &str,
    nombre: &str,
    moneda: &str,
    actividad: &str,
    rol_de_ana: &str,
    miembros: &[(&str, &str)],
) -> GrupoFijo {
    GrupoFijo {
        resumen: GrupoResumenDto {
            id: id.to_string(),
            nombre: nombre.to_string(),
            descripcion: String::new(),
            moneda: moneda.to_string(),
            actividad: actividad.to_string(),
            rol: rol_de_ana.to_string(),
            cantidad_miembros: 0,
        },
        miembros: miembros
            .iter()
            .map(|(u, rol)| MiembroDto {
                usuario: (*u).to_string(),
                nombre_visible: visible(u).unwrap_or(u).to_string(),
                rol: (*rol).to_string(),
            })
            .collect(),
        invitados: Vec::new(),
        gastos: Vec::new(),
    }
}

impl Estado {
    pub fn inicial() -> Estado {
        let mut viaje = grupo(
            "g1",
            "Viaje a Mérida",
            "USD",
            "marzo",
            "administrador",
            &[
                ("ana", "administrador"),
                ("beto", "miembro"),
                ("carla", "miembro"),
            ],
        );
        // Más reciente primero. Balances: ana +36,66 · beto +11,67 · carla −48,33.
        viaje.gastos = vec![
            GastoGrupoDto {
                id: "x2".to_string(),
                concepto: "Entradas al museo".to_string(),
                monto: "60.00".to_string(),
                fecha: "2026-11-11".to_string(),
                pagador: "beto".to_string(),
                regla: "porcentajes".to_string(),
                partes: vec![
                    parte("ana", "30.00", false),
                    parte("beto", "15.00", false),
                    parte("carla", "15.00", false),
                ],
            },
            GastoGrupoDto {
                id: "x1".to_string(),
                concepto: "Cena de bienvenida".to_string(),
                monto: "100.00".to_string(),
                fecha: "2026-11-10".to_string(),
                pagador: "ana".to_string(),
                regla: "partes_iguales".to_string(),
                partes: vec![
                    parte("ana", "33.34", true),
                    parte("beto", "33.33", false),
                    parte("carla", "33.33", false),
                ],
            },
        ];
        let cena = grupo(
            "g3",
            "Cena del viernes",
            "USD",
            "viernes",
            "miembro",
            &[("beto", "administrador"), ("ana", "miembro")],
        );
        Estado {
            amigos: vec![
                amigo("a1", "beto"),
                amigo("a2", "carla"),
                amigo("a3", "luis"),
            ],
            enviadas: Vec::new(),
            recibidas: vec![InvitacionAmistadDto {
                id: "i1".to_string(),
                usuario: publico("dario"),
                estado: "pendiente".to_string(),
            }],
            grupos: vec![viaje, cena],
            invitaciones: vec![InvitacionGrupoDto {
                grupo_id: "g2".to_string(),
                nombre: "Cumpleaños de Luis".to_string(),
                moneda: "USD".to_string(),
                actividad: "sábado".to_string(),
                invitado_por: publico("beto"),
            }],
            contador: 10,
        }
    }
}

// ---------- Errores ----------

fn campo(campo: &str, mensaje: &str) -> ErrorCampo {
    ErrorCampo {
        campo: Some(campo.to_string()),
        mensaje: mensaje.to_string(),
    }
}

fn falla(c: &str, mensaje: &str) -> ErroresDto {
    ErroresDto {
        errores: vec![campo(c, mensaje)],
    }
}

fn general(mensaje: &str) -> ErroresDto {
    ErroresDto {
        errores: vec![ErrorCampo {
            campo: None,
            mensaje: mensaje.to_string(),
        }],
    }
}

fn grupo_no_encontrado() -> ErroresDto {
    general("Grupo no encontrado.")
}

fn normal(texto: &str) -> String {
    texto.trim().to_lowercase()
}

/// Casos de prueba que se disparan con un texto especial.
fn disparo(texto: &str) -> Option<ErroresDto> {
    match normal(texto).as_str() {
        "sin-conexion" => Some(general(
            "No hay conexión con el servidor. Puedes seguir usando la aplicación sin iniciar sesión.",
        )),
        "sesion-vencida" => Some(general("Tu sesión venció, inicia sesión de nuevo.")),
        "sin-sesion" => Some(general("No has iniciado sesión.")),
        _ => None,
    }
}

// ---------- Aritmética de los balances (en céntimos) ----------

fn centimos(texto: &str) -> i128 {
    let (negativo, sin_signo) = match texto.strip_prefix('-') {
        Some(resto) => (true, resto),
        None => (false, texto),
    };
    let (entero, frac) = sin_signo.split_once('.').unwrap_or((sin_signo, "0"));
    let valor = entero.parse::<i128>().unwrap_or(0) * 100 + frac.parse::<i128>().unwrap_or(0);
    if negativo { -valor } else { valor }
}

fn con_signo(c: i128) -> String {
    let signo = if c < 0 { "-" } else { "" };
    let a = c.abs();
    format!("{signo}{}.{:02}", a / 100, a % 100)
}

fn resumen(g: &GrupoFijo) -> GrupoResumenDto {
    let mut r = g.resumen.clone();
    r.cantidad_miembros = u32::try_from(g.miembros.len()).unwrap_or(u32::MAX);
    r
}

fn grupo_de_ana<'a>(estado: &'a mut Estado, id: &str) -> Option<&'a mut GrupoFijo> {
    estado
        .grupos
        .iter_mut()
        .find(|g| g.resumen.id == id && g.miembros.iter().any(|m| m.usuario == YO))
}

// ---------- Amistades ----------

pub fn buscar(datos: &BuscarUsuarioDto) -> Result<UsuarioPublicoDto, ErroresDto> {
    let u = normal(&datos.usuario);
    if u.is_empty() {
        return Err(falla("usuario", "Escribe un nombre de usuario."));
    }
    if let Some(e) = disparo(&u) {
        return Err(e);
    }
    if visible(&u).is_some() {
        Ok(publico(&u))
    } else {
        Err(falla("usuario", NO_ENCONTRADO))
    }
}

pub fn invitar_amigo(
    estado: &mut Estado,
    datos: &InvitarAmigoDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    let u = normal(&datos.usuario);
    if u.is_empty() {
        return Err(falla("usuario", "Escribe un nombre de usuario."));
    }
    if let Some(e) = disparo(&u) {
        return Err(e);
    }
    if u == YO {
        return Err(falla("usuario", "No puedes invitarte a ti mismo."));
    }
    if visible(&u).is_none() {
        return Err(falla("usuario", NO_ENCONTRADO));
    }
    if estado.amigos.iter().any(|a| a.usuario.nombre_usuario == u) {
        return Err(falla("usuario", "Ya son amigos."));
    }
    let pendiente = estado
        .enviadas
        .iter()
        .chain(estado.recibidas.iter())
        .any(|i| i.usuario.nombre_usuario == u);
    if pendiente {
        return Err(falla(
            "usuario",
            "Ya hay una invitación pendiente entre ustedes.",
        ));
    }
    estado.contador += 1;
    let invitacion = InvitacionAmistadDto {
        id: format!("i{}", estado.contador),
        usuario: publico(&u),
        estado: "pendiente".to_string(),
    };
    estado.enviadas.push(invitacion.clone());
    Ok(invitacion)
}

pub fn responder_amistad(
    estado: &mut Estado,
    datos: &ResponderAmistadDto,
) -> Result<InvitacionAmistadDto, ErroresDto> {
    if let Some(e) = disparo(&datos.amistad_id) {
        return Err(e);
    }
    let respuesta = datos.respuesta.trim();
    if respuesta != "aceptar" && respuesta != "rechazar" {
        return Err(falla("respuesta", "Elige aceptar o rechazar."));
    }
    if estado.enviadas.iter().any(|i| i.id == datos.amistad_id) {
        return Err(general(
            "Solo quien recibe la invitación puede responderla.",
        ));
    }
    let Some(pos) = estado
        .recibidas
        .iter()
        .position(|i| i.id == datos.amistad_id)
    else {
        return Err(general("Esa invitación no existe o ya fue respondida."));
    };
    let mut invitacion = estado.recibidas.remove(pos);
    if respuesta == "aceptar" {
        invitacion.estado = "aceptada".to_string();
        estado.amigos.push(AmigoDto {
            amistad_id: invitacion.id.clone(),
            usuario: invitacion.usuario.clone(),
        });
    } else {
        invitacion.estado = "rechazada".to_string();
    }
    Ok(invitacion)
}

pub fn listar_amistades(estado: &Estado) -> ListaAmistadesDto {
    ListaAmistadesDto {
        amigos: estado.amigos.clone(),
        enviadas: estado.enviadas.clone(),
        recibidas: estado.recibidas.clone(),
    }
}

/// Eliminar una amistad no toca los grupos ni los gastos (RF-SOC-06).
pub fn eliminar_amistad(estado: &mut Estado, amistad_id: &str) -> Result<(), ErroresDto> {
    if let Some(e) = disparo(amistad_id) {
        return Err(e);
    }
    let Some(pos) = estado
        .amigos
        .iter()
        .position(|a| a.amistad_id == amistad_id)
    else {
        return Err(general("Esa amistad no existe."));
    };
    estado.amigos.remove(pos);
    Ok(())
}

// ---------- Grupos ----------

pub fn crear_grupo(
    estado: &mut Estado,
    datos: &NuevoGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    if let Some(e) = disparo(&datos.nombre) {
        return Err(e);
    }
    let mut errores = Vec::new();
    let nombre = datos.nombre.trim();
    if nombre.is_empty() {
        errores.push(campo("nombre", "El nombre del grupo es obligatorio."));
    }
    let moneda = datos.moneda.trim().to_uppercase();
    if moneda.len() != 3 || !moneda.chars().all(|c| c.is_ascii_alphabetic()) {
        errores.push(campo("moneda", "La moneda debe ser un código de 3 letras."));
    }
    if !errores.is_empty() {
        return Err(ErroresDto { errores });
    }
    estado.contador += 1;
    let id = format!("g{}", estado.contador);
    let mut nuevo = grupo(
        &id,
        nombre,
        &moneda,
        datos.actividad.trim(),
        "administrador",
        &[(YO, "administrador")],
    );
    nuevo.resumen.descripcion = datos.descripcion.trim().to_string();
    let respuesta = resumen(&nuevo);
    estado.grupos.push(nuevo);
    Ok(respuesta)
}

pub fn listar_grupos(estado: &Estado) -> ListaGruposDto {
    ListaGruposDto {
        grupos: estado
            .grupos
            .iter()
            .filter(|g| g.miembros.iter().any(|m| m.usuario == YO))
            .map(resumen)
            .collect(),
        invitaciones: estado.invitaciones.clone(),
    }
}

pub fn invitar_al_grupo(
    estado: &mut Estado,
    datos: &InvitarAlGrupoDto,
) -> Result<InvitadoDto, ErroresDto> {
    let u = normal(&datos.usuario);
    if let Some(e) = disparo(&u).or_else(|| disparo(&datos.grupo_id)) {
        return Err(e);
    }
    let amigos: Vec<String> = estado
        .amigos
        .iter()
        .map(|a| a.usuario.nombre_usuario.clone())
        .collect();
    let Some(g) = grupo_de_ana(estado, &datos.grupo_id) else {
        return Err(grupo_no_encontrado());
    };
    if g.resumen.rol != "administrador" {
        return Err(general("Solo el administrador puede invitar al grupo."));
    }
    if u.is_empty() {
        return Err(falla("usuario", "Escribe un nombre de usuario."));
    }
    if visible(&u).is_none() {
        return Err(falla("usuario", NO_ENCONTRADO));
    }
    if !amigos.contains(&u) {
        return Err(falla("usuario", "Solo puedes invitar a tus amigos."));
    }
    let ya_esta = g.miembros.iter().any(|m| m.usuario == u)
        || g.invitados
            .iter()
            .any(|i| i.usuario == u && i.estado == "pendiente");
    if ya_esta {
        return Err(falla(
            "usuario",
            "Esa persona ya es miembro del grupo o ya está invitada.",
        ));
    }
    let invitado = InvitadoDto {
        usuario: u.clone(),
        nombre_visible: visible(&u).unwrap_or(&u).to_string(),
        estado: "pendiente".to_string(),
    };
    g.invitados.push(invitado.clone());
    Ok(invitado)
}

pub fn responder_invitacion_grupo(
    estado: &mut Estado,
    datos: &ResponderInvitacionGrupoDto,
) -> Result<GrupoResumenDto, ErroresDto> {
    if let Some(e) = disparo(&datos.grupo_id) {
        return Err(e);
    }
    let respuesta = datos.respuesta.trim();
    if respuesta != "aceptar" && respuesta != "rechazar" {
        return Err(falla("respuesta", "Elige aceptar o rechazar."));
    }
    let Some(pos) = estado
        .invitaciones
        .iter()
        .position(|i| i.grupo_id == datos.grupo_id)
    else {
        if grupo_de_ana(estado, &datos.grupo_id).is_some() {
            return Err(general("Esa invitación ya fue respondida."));
        }
        return Err(grupo_no_encontrado());
    };
    let invitacion = estado.invitaciones.remove(pos);
    let administrador = invitacion.invitado_por.nombre_usuario.clone();
    let nuevo = grupo(
        &invitacion.grupo_id,
        &invitacion.nombre,
        &invitacion.moneda,
        &invitacion.actividad,
        "miembro",
        &[(administrador.as_str(), "administrador"), (YO, "miembro")],
    );
    let respuesta_dto = resumen(&nuevo);
    if respuesta == "aceptar" {
        estado.grupos.push(nuevo);
    }
    Ok(respuesta_dto)
}

// ---------- Gastos y detalle ----------

fn fecha_valida(f: &str) -> bool {
    f.len() == 10
        && f.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Registra un gasto calculando la división con el mismo código del dominio, como
/// hará el servidor. Los errores de validación y de porcentajes o montos que no
/// cuadran llegan por campo (en el servidor serían un 400).
pub fn registrar_gasto(
    estado: &mut Estado,
    datos: &NuevoGastoGrupoDto,
) -> Result<GastoGrupoDto, ErroresDto> {
    if let Some(e) = disparo(&datos.concepto).or_else(|| disparo(&datos.grupo_id)) {
        return Err(e);
    }
    let id = format!("x{}", estado.contador + 1);
    let Some(g) = grupo_de_ana(estado, &datos.grupo_id) else {
        return Err(grupo_no_encontrado());
    };

    let mut errores = Vec::new();
    if datos.concepto.trim().is_empty() {
        errores.push(campo("concepto", "El concepto es obligatorio."));
    }
    if !fecha_valida(datos.fecha.trim()) {
        errores.push(campo("fecha", "La fecha debe tener el formato AAAA-MM-DD."));
    }
    let pagador = normal(&datos.pagador);
    if !g.miembros.iter().any(|m| m.usuario == pagador) {
        errores.push(campo("pagador", "El pagador debe ser miembro del grupo."));
    }
    for p in &datos.participantes {
        let u = normal(&p.usuario);
        if !u.is_empty() && !g.miembros.iter().any(|m| m.usuario == u) {
            errores.push(campo(
                "participantes",
                &format!("{u} no es miembro del grupo."),
            ));
        }
    }

    let participantes: Vec<Participante> = datos
        .participantes
        .iter()
        .map(|p| Participante {
            usuario: p.usuario.clone(),
            valor: p.valor.clone(),
        })
        .collect();
    match division::dividir(&datos.monto, &datos.regla, &participantes) {
        Err(fallos) => {
            errores.extend(fallos.into_iter().map(|e| ErrorCampo {
                campo: Some(e.campo),
                mensaje: e.mensaje,
            }));
            Err(ErroresDto { errores })
        }
        Ok(_) if !errores.is_empty() => Err(ErroresDto { errores }),
        Ok(d) => {
            let monto = con_signo(d.partes.iter().map(|p| centimos(&p.monto)).sum());
            let gasto = GastoGrupoDto {
                id,
                concepto: datos.concepto.trim().to_string(),
                monto,
                fecha: datos.fecha.trim().to_string(),
                pagador,
                regla: datos.regla.trim().to_string(),
                partes: d
                    .partes
                    .into_iter()
                    .map(|p| ParteDto {
                        usuario: p.usuario,
                        monto: p.monto,
                        incluye_redondeo: p.incluye_redondeo,
                    })
                    .collect(),
            };
            g.gastos.insert(0, gasto.clone());
            estado.contador += 1;
            Ok(gasto)
        }
    }
}

pub fn ver_grupo(estado: &mut Estado, id: &str) -> Result<GrupoDetalleDto, ErroresDto> {
    if let Some(e) = disparo(id) {
        return Err(e);
    }
    let Some(g) = grupo_de_ana(estado, id) else {
        return Err(grupo_no_encontrado());
    };
    let balances = g
        .miembros
        .iter()
        .map(|m| {
            let pago: i128 = g
                .gastos
                .iter()
                .filter(|x| x.pagador == m.usuario)
                .map(|x| centimos(&x.monto))
                .sum();
            let le_corresponde: i128 = g
                .gastos
                .iter()
                .flat_map(|x| x.partes.iter())
                .filter(|p| p.usuario == m.usuario)
                .map(|p| centimos(&p.monto))
                .sum();
            BalanceDto {
                usuario: m.usuario.clone(),
                pago: con_signo(pago),
                le_corresponde: con_signo(le_corresponde),
                balance: con_signo(pago - le_corresponde),
            }
        })
        .collect();
    Ok(GrupoDetalleDto {
        grupo: resumen(g),
        miembros: g.miembros.clone(),
        gastos: g.gastos.clone(),
        balances,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dto_social::ParticipanteDto;

    fn exito<T>(r: Result<T, ErroresDto>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => panic!(
                "no debía fallar: {}",
                serde_json::to_string(&e).unwrap_or_default()
            ),
        }
    }

    type Fallos = Vec<(Option<String>, String)>;

    fn errores<T>(r: Result<T, ErroresDto>) -> Fallos {
        match r {
            Ok(_) => panic!("se esperaban errores"),
            Err(e) => e
                .errores
                .into_iter()
                .map(|c| (c.campo, c.mensaje))
                .collect(),
        }
    }

    fn gasto(
        grupo: &str,
        monto: &str,
        regla: &str,
        pagador: &str,
        gente: &[(&str, &str)],
    ) -> NuevoGastoGrupoDto {
        NuevoGastoGrupoDto {
            grupo_id: grupo.to_string(),
            concepto: "Compra".to_string(),
            monto: monto.to_string(),
            fecha: "2026-11-12".to_string(),
            pagador: pagador.to_string(),
            regla: regla.to_string(),
            participantes: gente
                .iter()
                .map(|(u, v)| ParticipanteDto {
                    usuario: (*u).to_string(),
                    valor: (*v).to_string(),
                })
                .collect(),
        }
    }

    fn invitar(u: &str) -> InvitarAmigoDto {
        InvitarAmigoDto {
            usuario: u.to_string(),
        }
    }

    fn al_grupo(g: &str, u: &str) -> InvitarAlGrupoDto {
        InvitarAlGrupoDto {
            grupo_id: g.to_string(),
            usuario: u.to_string(),
        }
    }

    fn suma_de_balances(d: &GrupoDetalleDto) -> i128 {
        d.balances.iter().map(|b| centimos(&b.balance)).sum()
    }

    #[test]
    fn los_balances_iniciales_son_los_del_enunciado_y_suman_cero() {
        let mut e = Estado::inicial();
        let d = exito(ver_grupo(&mut e, "g1"));
        let b: Vec<_> = d
            .balances
            .iter()
            .map(|b| (b.usuario.as_str(), b.balance.as_str()))
            .collect();
        assert_eq!(
            b,
            [("ana", "36.66"), ("beto", "11.67"), ("carla", "-48.33")]
        );
        assert_eq!(suma_de_balances(&d), 0);
        assert_eq!(d.gastos.len(), 2);
        assert_eq!(d.grupo.cantidad_miembros, 3);
    }

    #[test]
    fn la_busqueda_es_exacta_y_los_casos_de_prueba_se_disparan() {
        assert_eq!(
            exito(buscar(&BuscarUsuarioDto {
                usuario: " Beto ".into()
            }))
            .nombre_usuario,
            "beto"
        );
        for parcial in ["bet", "BE", "b"] {
            let e = errores(buscar(&BuscarUsuarioDto {
                usuario: parcial.into(),
            }));
            assert_eq!(e[0].0.as_deref(), Some("usuario"));
        }
        let e = errores(buscar(&BuscarUsuarioDto {
            usuario: "sin-conexion".into(),
        }));
        assert!(e[0].1.starts_with("No hay conexión con el servidor"));
        let e = errores(buscar(&BuscarUsuarioDto {
            usuario: "sesion-vencida".into(),
        }));
        assert_eq!(e[0].1, "Tu sesión venció, inicia sesión de nuevo.");
        let e = errores(buscar(&BuscarUsuarioDto { usuario: "".into() }));
        assert_eq!(e[0].0.as_deref(), Some("usuario"));
    }

    #[test]
    fn invitar_amigos_cubre_cada_caso() {
        let mut e = Estado::inicial();
        assert_eq!(
            errores(invitar_amigo(&mut e, &invitar("ana")))[0].1,
            "No puedes invitarte a ti mismo."
        );
        assert_eq!(
            errores(invitar_amigo(&mut e, &invitar("luis")))[0].1,
            "Ya son amigos."
        );
        assert_eq!(
            errores(invitar_amigo(&mut e, &invitar("dario")))[0].1,
            "Ya hay una invitación pendiente entre ustedes."
        );
        assert_eq!(
            errores(invitar_amigo(&mut e, &invitar("zoe")))[0].1,
            NO_ENCONTRADO
        );
        // No hay otra persona que invitar: se crea una enviada con «dario» ya respondida.
        let r = exito(responder_amistad(
            &mut e,
            &ResponderAmistadDto {
                amistad_id: "i1".into(),
                respuesta: "rechazar".into(),
            },
        ));
        assert_eq!(r.estado, "rechazada");
        let enviada = exito(invitar_amigo(&mut e, &invitar("dario")));
        assert_eq!(enviada.estado, "pendiente");
        assert_eq!(listar_amistades(&e).enviadas.len(), 1);
        // Repetir la invitación da 409.
        assert!(
            errores(invitar_amigo(&mut e, &invitar("DARIO")))[0]
                .1
                .contains("pendiente")
        );
    }

    #[test]
    fn aceptar_una_invitacion_crea_la_amistad_y_no_se_puede_responder_dos_veces() {
        let mut e = Estado::inicial();
        let datos = ResponderAmistadDto {
            amistad_id: "i1".into(),
            respuesta: "aceptar".into(),
        };
        let r = exito(responder_amistad(&mut e, &datos));
        assert_eq!(r.estado, "aceptada");
        let lista = listar_amistades(&e);
        assert!(
            lista
                .amigos
                .iter()
                .any(|a| a.usuario.nombre_usuario == "dario")
        );
        assert!(lista.recibidas.is_empty());
        let otra = errores(responder_amistad(&mut e, &datos));
        assert_eq!(otra[0].1, "Esa invitación no existe o ya fue respondida.");
        let mala = ResponderAmistadDto {
            amistad_id: "i1".into(),
            respuesta: "quizas".into(),
        };
        assert_eq!(
            errores(responder_amistad(&mut e, &mala))[0].0.as_deref(),
            Some("respuesta")
        );
    }

    #[test]
    fn eliminar_una_amistad_no_toca_los_grupos_ni_los_gastos() {
        let mut e = Estado::inicial();
        exito(eliminar_amistad(&mut e, "a2")); // carla
        assert!(
            !listar_amistades(&e)
                .amigos
                .iter()
                .any(|a| a.usuario.nombre_usuario == "carla")
        );
        let d = exito(ver_grupo(&mut e, "g1"));
        assert_eq!(d.miembros.len(), 3, "carla sigue en el grupo");
        assert_eq!(d.gastos.len(), 2);
        assert_eq!(suma_de_balances(&d), 0);
        assert_eq!(
            errores(eliminar_amistad(&mut e, "a2"))[0].1,
            "Esa amistad no existe."
        );
    }

    #[test]
    fn porcentajes_que_suman_95_se_rechazan_y_50_25_25_se_registra() {
        let mut e = Estado::inicial();
        let malo = gasto(
            "g1",
            "60.00",
            "porcentajes",
            "beto",
            &[("ana", "50"), ("beto", "25"), ("carla", "20")],
        );
        let f = errores(registrar_gasto(&mut e, &malo));
        assert!(f.iter().any(|(c, m)| c.as_deref() == Some("porcentajes")
            && m == "Los porcentajes suman 95; faltan 5."));
        assert_eq!(
            exito(ver_grupo(&mut e, "g1")).gastos.len(),
            2,
            "no se guardó nada"
        );

        let bueno = gasto(
            "g1",
            "60.00",
            "porcentajes",
            "beto",
            &[("ana", "50"), ("beto", "25"), ("carla", "25")],
        );
        let g = exito(registrar_gasto(&mut e, &bueno));
        let montos: Vec<_> = g.partes.iter().map(|p| p.monto.as_str()).collect();
        assert_eq!(montos, ["30.00", "15.00", "15.00"]);
        let d = exito(ver_grupo(&mut e, "g1"));
        assert_eq!(d.gastos.len(), 3);
        assert_eq!(d.gastos[0].id, g.id, "el más reciente va primero");
        assert_eq!(suma_de_balances(&d), 0);
    }

    #[test]
    fn partes_iguales_marca_el_residuo_y_los_balances_siguen_en_cero() {
        let mut e = Estado::inicial();
        let datos = gasto(
            "g1",
            "100.00",
            "partes_iguales",
            "carla",
            &[("ana", ""), ("beto", ""), ("carla", "")],
        );
        let g = exito(registrar_gasto(&mut e, &datos));
        assert_eq!(g.partes[0].monto, "33.34");
        assert!(g.partes[0].incluye_redondeo && !g.partes[1].incluye_redondeo);
        assert_eq!(suma_de_balances(&exito(ver_grupo(&mut e, "g1"))), 0);
    }

    #[test]
    fn los_datos_invalidos_de_un_gasto_llegan_por_campo() {
        let mut e = Estado::inicial();
        let mut datos = gasto(
            "g1",
            "100.00",
            "partes_iguales",
            "dario",
            &[("ana", ""), ("dario", "")],
        );
        datos.concepto = " ".into();
        datos.fecha = "12/11/2026".into();
        let f = errores(registrar_gasto(&mut e, &datos));
        let campos: Vec<_> = f.iter().filter_map(|(c, _)| c.as_deref()).collect();
        assert_eq!(campos, ["concepto", "fecha", "pagador", "participantes"]);
    }

    #[test]
    fn un_no_miembro_recibe_la_misma_respuesta_que_un_grupo_inexistente() {
        let mut e = Estado::inicial();
        let inexistente = errores(ver_grupo(&mut e, "g99"));
        assert_eq!(
            errores(ver_grupo(&mut e, "g2")),
            inexistente,
            "g2: solo invitada"
        );
        let g = gasto("g2", "10.00", "partes_iguales", "ana", &[("ana", "")]);
        let h = gasto("g99", "10.00", "partes_iguales", "ana", &[("ana", "")]);
        assert_eq!(
            errores(registrar_gasto(&mut e, &g)),
            errores(registrar_gasto(&mut e, &h))
        );
        assert_eq!(
            errores(invitar_al_grupo(&mut e, &al_grupo("g2", "luis"))),
            errores(invitar_al_grupo(&mut e, &al_grupo("g99", "luis")))
        );
    }

    #[test]
    fn invitar_al_grupo_cubre_403_y_409() {
        let mut e = Estado::inicial();
        assert_eq!(
            errores(invitar_al_grupo(&mut e, &al_grupo("g3", "luis")))[0].1,
            "Solo el administrador puede invitar al grupo."
        );
        assert_eq!(
            errores(invitar_al_grupo(&mut e, &al_grupo("g1", "dario")))[0].1,
            "Solo puedes invitar a tus amigos."
        );
        assert!(
            errores(invitar_al_grupo(&mut e, &al_grupo("g1", "beto")))[0]
                .1
                .contains("ya es miembro")
        );
        let ok = exito(invitar_al_grupo(&mut e, &al_grupo("g1", "luis")));
        assert_eq!(ok.estado, "pendiente");
        assert!(
            errores(invitar_al_grupo(&mut e, &al_grupo("g1", "luis")))[0]
                .1
                .contains("ya está invitada")
        );
        // El invitado todavía no es miembro: no aparece en el detalle.
        assert_eq!(exito(ver_grupo(&mut e, "g1")).miembros.len(), 3);
    }

    #[test]
    fn aceptar_una_invitacion_de_grupo_da_acceso_y_antes_no() {
        let mut e = Estado::inicial();
        assert!(ver_grupo(&mut e, "g2").is_err());
        let datos = ResponderInvitacionGrupoDto {
            grupo_id: "g2".into(),
            respuesta: "aceptar".into(),
        };
        let r = exito(responder_invitacion_grupo(&mut e, &datos));
        assert_eq!(r.rol, "miembro");
        assert!(listar_grupos(&e).invitaciones.is_empty());
        assert_eq!(exito(ver_grupo(&mut e, "g2")).miembros.len(), 2);
        assert_eq!(
            errores(responder_invitacion_grupo(&mut e, &datos))[0].1,
            "Esa invitación ya fue respondida."
        );
    }

    #[test]
    fn rechazar_una_invitacion_de_grupo_no_da_acceso() {
        let mut e = Estado::inicial();
        let datos = ResponderInvitacionGrupoDto {
            grupo_id: "g2".into(),
            respuesta: "rechazar".into(),
        };
        exito(responder_invitacion_grupo(&mut e, &datos));
        assert!(ver_grupo(&mut e, "g2").is_err());
        assert!(listar_grupos(&e).invitaciones.is_empty());
    }

    #[test]
    fn crear_un_grupo_valida_nombre_y_moneda() {
        let mut e = Estado::inicial();
        let malo = NuevoGrupoDto {
            nombre: "  ".into(),
            descripcion: String::new(),
            moneda: "DÓLAR".into(),
            actividad: String::new(),
        };
        let f = errores(crear_grupo(&mut e, &malo));
        let campos: Vec<_> = f.iter().filter_map(|(c, _)| c.as_deref()).collect();
        assert_eq!(campos, ["nombre", "moneda"]);
        let bueno = NuevoGrupoDto {
            nombre: "Casa".into(),
            descripcion: "Gastos".into(),
            moneda: "eur".into(),
            actividad: "diciembre".into(),
        };
        let g = exito(crear_grupo(&mut e, &bueno));
        assert_eq!(
            (g.moneda.as_str(), g.rol.as_str(), g.cantidad_miembros),
            ("EUR", "administrador", 1)
        );
        assert_eq!(listar_grupos(&e).grupos.len(), 3);
    }
}
