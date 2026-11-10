import { invoke } from '@tauri-apps/api/core';
import type { DatosInicioSesion, DatosRegistro } from '../types/sesion';

/*
 * Capa de comandos: es el único lugar del frontend que conoce los nombres de los
 * comandos de Rust y la forma exacta de sus argumentos y respuestas.
 *
 * Convenciones del backend (ver src-tauri/src/dto.rs):
 *  - Los comandos reciben UN argumento llamado `datos` (salvo `detalle_cuenta` y
 *    `eliminar_tasa`, que reciben `cuenta_id` y `tasa_id`).
 *  - Montos, tasas y decimales viajan como TEXTO con punto decimal ("50.00").
 *  - Fechas viajan como "AAAA-MM-DD".
 *  - Los errores de validación llegan como `{ errores: [{ campo, mensaje }] }`.
 */

// ---------- Errores ----------

export interface ErrorCampo {
  campo: string | null;
  mensaje: string;
}

export interface ErroresDto {
  errores: ErrorCampo[];
}

export function esErroresDto(err: unknown): err is ErroresDto {
  return (
    typeof err === 'object' &&
    err !== null &&
    'errores' in err &&
    Array.isArray((err as { errores: unknown }).errores)
  );
}

/** Errores por campo si el backend los envió; si no, un único error general. */
export function extraerErrores(err: unknown, mensajePorDefecto: string): ErrorCampo[] {
  if (esErroresDto(err)) return err.errores;
  return [{ campo: null, mensaje: typeof err === 'string' && err ? err : mensajePorDefecto }];
}

/** Convierte cualquier error de invoke en un texto legible (para avisos simples). */
export function mensajeDeError(err: unknown, mensajePorDefecto: string): string {
  return extraerErrores(err, mensajePorDefecto)
    .map((e) => e.mensaje)
    .join(' ');
}

// ---------- DTOs: monedas, cuentas, categorías, movimientos ----------

export interface MonedaDto {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
  de_fabrica: boolean;
}

export interface CuentaDto {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_actual: string;
}

export interface SubtotalDto {
  moneda: string;
  total: string;
}

export interface ListaCuentasDto {
  cuentas: CuentaDto[];
  subtotales: SubtotalDto[];
}

export interface CategoriaDto {
  id: string;
  nombre: string;
  tipo: string;
  de_fabrica: boolean;
}

export interface MovimientoDto {
  id: string;
  cuenta_id: string;
  tipo: string;
  monto: string;
  monto_con_signo: string;
  monto_vigente: string | null;
  fecha: string;
  categoria_id: string;
  categoria_nombre: string;
  nota: string;
  /** "vigente" | "corregido" | "ajuste" */
  estado: string;
  corrige_a: string | null;
  ajustes: string[];
  monto_correcto: string | null;
}

export interface ComposicionDto {
  saldo_inicial: string;
  total_ingresos: string;
  total_gastos: string;
  total_ajustes: string;
}

export interface DetalleCuentaDto {
  cuenta: CuentaDto;
  composicion: ComposicionDto;
  movimientos: MovimientoDto[];
}

export interface PrevisualizacionDto {
  saldo_resultante: string;
  deja_saldo_negativo: boolean;
}

// ---------- Payloads de entrada ----------

export interface NuevaCuentaPayload {
  nombre: string;
  /** "efectivo" | "banco" | "billetera_digital" */
  tipo: string;
  moneda: string;
  saldo_inicial: string;
}

export interface NuevaCategoriaPayload {
  nombre: string;
  /** "gasto" | "ingreso" */
  tipo: string;
}

export interface NuevoMovimientoPayload {
  cuenta_id: string;
  /** "gasto" | "ingreso" */
  tipo: string;
  monto: string;
  fecha: string;
  categoria_id: string;
  nota: string;
}

export interface CorreccionPayload {
  movimiento_id: string;
  monto_correcto: string;
}

// ---------- DTOs: tasas, moneda base y patrimonio ----------

export interface RegistrarTasaPayload {
  moneda_origen: string;
  moneda_destino: string;
  valor: string;
  fecha: string;
  fuente: string;
}

export interface TasaDto {
  id: string;
  moneda_origen: string;
  moneda_destino: string;
  valor: string;
  fecha: string;
  fuente: string;
}

export interface HistorialParDto {
  moneda_origen: string;
  moneda_destino: string;
  tasas: TasaDto[];
}

export interface SaldoCuentaDto {
  id: string;
  nombre: string;
  moneda: string;
  saldo: string;
  en_moneda_base: string | null;
}

export interface SubtotalMonedaDto {
  moneda: string;
  total: string;
}

export interface TasaUsadaDto {
  moneda_origen: string;
  moneda_destino: string;
  valor: string;
  fecha: string;
  fuente: string;
  inversa: boolean;
}

export interface ParFaltanteDto {
  moneda_origen: string;
  moneda_destino: string;
}

export interface PatrimonioDto {
  moneda_base: string;
  fecha_consulta: string;
  cuentas: SaldoCuentaDto[];
  subtotales: SubtotalMonedaDto[];
  total: string | null;
  tasas_usadas: TasaUsadaDto[];
  faltantes: ParFaltanteDto[];
}

export interface NuevaMonedaPayload {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: string;
}

// ---------- Comandos: monedas, cuentas, categorías y movimientos ----------

export async function listarMonedasCommand(): Promise<MonedaDto[]> {
  return await invoke<MonedaDto[]>('listar_monedas');
}

export async function listarCuentasCommand(): Promise<ListaCuentasDto> {
  return await invoke<ListaCuentasDto>('listar_cuentas');
}

export async function crearCuentaCommand(datos: NuevaCuentaPayload): Promise<CuentaDto> {
  return await invoke<CuentaDto>('crear_cuenta', { datos });
}

export async function detalleCuentaCommand(cuentaId: string): Promise<DetalleCuentaDto> {
  return await invoke<DetalleCuentaDto>('detalle_cuenta', { cuenta_id: cuentaId });
}

export async function listarCategoriasCommand(): Promise<CategoriaDto[]> {
  return await invoke<CategoriaDto[]>('listar_categorias');
}

export async function crearCategoriaCommand(datos: NuevaCategoriaPayload): Promise<CategoriaDto> {
  return await invoke<CategoriaDto>('crear_categoria', { datos });
}

export async function registrarMovimientoCommand(
  datos: NuevoMovimientoPayload
): Promise<MovimientoDto> {
  return await invoke<MovimientoDto>('registrar_movimiento', { datos });
}

export async function previsualizarMovimientoCommand(
  datos: NuevoMovimientoPayload
): Promise<PrevisualizacionDto> {
  return await invoke<PrevisualizacionDto>('previsualizar_movimiento', { datos });
}

export async function corregirMovimientoCommand(datos: CorreccionPayload): Promise<MovimientoDto> {
  return await invoke<MovimientoDto>('corregir_movimiento', { datos });
}

// ---------- Comandos: tasas, moneda base y patrimonio ----------

export async function registrarTasaCommand(datos: RegistrarTasaPayload): Promise<TasaDto> {
  return await invoke<TasaDto>('registrar_tasa', { datos });
}

export async function listarTasasCommand(): Promise<HistorialParDto[]> {
  return await invoke<HistorialParDto[]>('listar_tasas');
}

export async function eliminarTasaCommand(tasaId: string): Promise<void> {
  return await invoke<void>('eliminar_tasa', { tasa_id: tasaId });
}

export async function cambiarMonedaBaseCommand(moneda: string): Promise<{ moneda: string }> {
  return await invoke<{ moneda: string }>('cambiar_moneda_base', { datos: { moneda } });
}

export async function crearMonedaCommand(datos: NuevaMonedaPayload): Promise<MonedaDto> {
  return await invoke<MonedaDto>('crear_moneda', { datos });
}

/** `fecha` vacía significa «hoy» (lo resuelve el backend). */
export async function consultarPatrimonioCommand(fecha: string): Promise<PatrimonioDto> {
  return await invoke<PatrimonioDto>('consultar_patrimonio', { datos: { fecha } });
}

// ---------- Comandos: registro e inicio de sesión (Entrega 5) ----------
// El token nunca llega a la interfaz: se queda en Rust y en el almacén de secretos.

export type EstadoSesion = 'sin_sesion' | 'vigente' | 'vencida';

export interface SesionDto {
  estado: EstadoSesion;
  nombre_visible: string | null;
  vence_en: string | null;
}

export interface UsuarioDto {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  correo: string;
  fecha_alta: string;
}

export async function registrarUsuarioCommand(datos: DatosRegistro): Promise<UsuarioDto> {
  return await invoke<UsuarioDto>('registrar_usuario', { datos });
}

export async function iniciarSesionCommand(datos: DatosInicioSesion): Promise<SesionDto> {
  return await invoke<SesionDto>('iniciar_sesion', { datos });
}

export async function obtenerPerfilCommand(): Promise<UsuarioDto> {
  return await invoke<UsuarioDto>('obtener_perfil');
}

export async function cerrarSesionCommand(): Promise<void> {
  await invoke<void>('cerrar_sesion');
}

export async function estadoSesionCommand(): Promise<SesionDto> {
  return await invoke<SesionDto>('estado_sesion');
}

// ---------- Comandos: amistades, grupos y gastos compartidos (Entrega 6) ----------
// Montos y porcentajes viajan como TEXTO con punto decimal ("100.00"). A las personas
// se las identifica por su nombre de usuario; nunca viaja el correo ni nada del libro
// personal (cuentas, movimientos, saldos, tasas). Los errores llegan como ErroresDto.

export type RespuestaInvitacion = 'aceptar' | 'rechazar';
export type EstadoInvitacion = 'pendiente' | 'aceptada' | 'rechazada';
export type RolGrupo = 'administrador' | 'miembro';
export type ReglaDivision = 'partes_iguales' | 'porcentajes' | 'montos_fijos';

// --- Personas y amistades ---

export interface BuscarUsuarioPayload {
  usuario: string;
}

export interface UsuarioPublicoDto {
  nombre_usuario: string;
  nombre_visible: string;
}

export interface InvitarAmigoPayload {
  usuario: string;
}

export interface ResponderAmistadPayload {
  amistad_id: string;
  respuesta: RespuestaInvitacion;
}

export interface AmigoDto {
  amistad_id: string;
  usuario: UsuarioPublicoDto;
}

export interface InvitacionAmistadDto {
  id: string;
  /** La otra persona: a quien se invitó (enviadas) o quien invitó (recibidas). */
  usuario: UsuarioPublicoDto;
  estado: EstadoInvitacion;
}

export interface ListaAmistadesDto {
  amigos: AmigoDto[];
  enviadas: InvitacionAmistadDto[];
  recibidas: InvitacionAmistadDto[];
}

// --- Grupos y miembros ---

export interface NuevoGrupoPayload {
  nombre: string;
  /** Opcional ("" si no hay). */
  descripcion: string;
  /** Código de 3 letras. */
  moneda: string;
  /** Fecha o actividad asociada, por ejemplo "Viaje a Mérida, marzo". */
  actividad: string;
}

export interface GrupoResumenDto {
  id: string;
  nombre: string;
  descripcion: string;
  moneda: string;
  actividad: string;
  rol: RolGrupo;
  cantidad_miembros: number;
}

export interface InvitacionGrupoDto {
  grupo_id: string;
  nombre: string;
  moneda: string;
  actividad: string;
  invitado_por: UsuarioPublicoDto;
}

export interface ListaGruposDto {
  grupos: GrupoResumenDto[];
  invitaciones: InvitacionGrupoDto[];
}

export interface InvitarAlGrupoPayload {
  grupo_id: string;
  usuario: string;
}

export interface InvitadoDto {
  usuario: string;
  nombre_visible: string;
  estado: EstadoInvitacion;
}

export interface ResponderInvitacionGrupoPayload {
  grupo_id: string;
  respuesta: RespuestaInvitacion;
}

// --- Gastos compartidos y división ---

export interface ParticipantePayload {
  usuario: string;
  /** Porcentaje ("50") o monto ("40.00") según la regla; "" en partes iguales. */
  valor: string;
}

export interface NuevoGastoGrupoPayload {
  grupo_id: string;
  concepto: string;
  monto: string;
  /** AAAA-MM-DD */
  fecha: string;
  /** Nombre de usuario de un miembro del grupo. */
  pagador: string;
  regla: ReglaDivision;
  participantes: ParticipantePayload[];
}

export interface ParteDto {
  usuario: string;
  monto: string;
  /** Verdadero si recibió céntimos del residuo («incluye 0,01 de redondeo»). */
  incluye_redondeo: boolean;
}

export interface GastoGrupoDto {
  id: string;
  concepto: string;
  monto: string;
  fecha: string;
  pagador: string;
  regla: ReglaDivision;
  partes: ParteDto[];
}

export interface PrevisualizarDivisionPayload {
  monto: string;
  regla: ReglaDivision;
  participantes: ParticipantePayload[];
}

export interface PrevisualizacionDivisionDto {
  /** Vacío si no cuadra. */
  partes: ParteDto[];
  /** Suma de los céntimos de residuo repartidos ("0.01"). */
  residuo: string;
  cuadra: boolean;
  /** Mensaje exacto si no cuadra: «Los porcentajes suman 95; faltan 5.» */
  diferencia: string | null;
}

// --- Detalle del grupo ---

export interface MiembroDto {
  usuario: string;
  nombre_visible: string;
  rol: RolGrupo;
}

export interface BalanceDto {
  usuario: string;
  pago: string;
  le_corresponde: string;
  /** Positivo: le deben; negativo ("-48.33"): debe. La suma de todos es "0.00". */
  balance: string;
}

export interface GrupoDetalleDto {
  grupo: GrupoResumenDto;
  miembros: MiembroDto[];
  gastos: GastoGrupoDto[];
  balances: BalanceDto[];
}

// --- Llamadas a los comandos ---

export async function buscarUsuarioCommand(datos: BuscarUsuarioPayload): Promise<UsuarioPublicoDto> {
  return await invoke<UsuarioPublicoDto>('buscar_usuario', { datos });
}

export async function invitarAmigoCommand(datos: InvitarAmigoPayload): Promise<InvitacionAmistadDto> {
  return await invoke<InvitacionAmistadDto>('invitar_amigo', { datos });
}

export async function responderAmistadCommand(
  datos: ResponderAmistadPayload
): Promise<InvitacionAmistadDto> {
  return await invoke<InvitacionAmistadDto>('responder_amistad', { datos });
}

export async function listarAmistadesCommand(): Promise<ListaAmistadesDto> {
  return await invoke<ListaAmistadesDto>('listar_amistades');
}

export async function eliminarAmistadCommand(amistadId: string): Promise<void> {
  return await invoke<void>('eliminar_amistad', { amistad_id: amistadId });
}

export async function crearGrupoCommand(datos: NuevoGrupoPayload): Promise<GrupoResumenDto> {
  return await invoke<GrupoResumenDto>('crear_grupo', { datos });
}

export async function listarGruposCommand(): Promise<ListaGruposDto> {
  return await invoke<ListaGruposDto>('listar_grupos');
}

export async function invitarAlGrupoCommand(datos: InvitarAlGrupoPayload): Promise<InvitadoDto> {
  return await invoke<InvitadoDto>('invitar_al_grupo', { datos });
}

export async function responderInvitacionGrupoCommand(
  datos: ResponderInvitacionGrupoPayload
): Promise<GrupoResumenDto> {
  return await invoke<GrupoResumenDto>('responder_invitacion_grupo', { datos });
}

export async function registrarGastoGrupoCommand(
  datos: NuevoGastoGrupoPayload
): Promise<GastoGrupoDto> {
  return await invoke<GastoGrupoDto>('registrar_gasto_grupo', { datos });
}

export async function verGrupoCommand(grupoId: string): Promise<GrupoDetalleDto> {
  return await invoke<GrupoDetalleDto>('ver_grupo', { grupo_id: grupoId });
}

/** Cálculo local con decimales exactos; no usa la red. */
export async function previsualizarDivisionCommand(
  datos: PrevisualizarDivisionPayload
): Promise<PrevisualizacionDivisionDto> {
  return await invoke<PrevisualizacionDivisionDto>('previsualizar_division', { datos });
}
