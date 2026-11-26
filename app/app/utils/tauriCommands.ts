import { invoke } from '@tauri-apps/api/core';
import type { DatosInicioSesion, DatosRegistro } from '../types/sesion';
import type {
  ActualizarCredencialesPayload,
  ConfigurarDestinoPayload,
  DestinoSincronizacionDto,
  EstadoSincronizacionLibroDto,
  InformeSincronizacionLibroDto,
  ResumenImportacionDto,
} from '../types/copia';
import * as simulacion from './simulacionIntercambio';

/*
 * Capa de comandos: es el único lugar del frontend que conoce los nombres de los
 * comandos de Rust y la forma exacta de sus argumentos y respuestas.
 *
 * Convenciones del backend (ver src-tauri/src/dto.rs):
 *  - Los comandos reciben UN argumento llamado `datos` (salvo `detalle_cuenta` y
 *    `eliminar_tasa`, que reciben `cuenta_id` y `tasa_id` porque en Rust llevan
 *    `rename_all = "snake_case"`).
 *  - Los comandos de social.rs sin `datos` (`ver_grupo`, `eliminar_amistad`) NO llevan
 *    ese atributo: Tauri espera el argumento en camelCase (`grupoId`, `amistadId`).
 *  - Montos, tasas y decimales viajan como TEXTO con punto decimal ("50.00").
 *  - Fechas viajan como "AAAA-MM-DD".
 *  - Los errores de validación llegan como `{ errores: [{ campo, mensaje }] }`.
 */

// ---------- Errores ----------

export interface ErrorCampo {
  campo: string | null;
  mensaje: string;
  /**
   * Código estable del error (p. ej. «frase_incorrecta»), opcional. Si el núcleo lo envía, la
   * interfaz lo usa; si no, reconoce el tipo por el texto (ver `utils/erroresCopia.ts`).
   */
  codigo?: string | null;
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
  /** true si viene de un gasto de grupo: no se corrige desde el libro */
  gasto_de_grupo: boolean;
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
  /**
   * Id del usuario (Entrega 7). Opcional: si el núcleo aún no lo envía, la interfaz usa un
   * contador de cambios de libro como clave y funciona igual.
   */
  usuario_id?: string | null;
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
  return await invoke<void>('eliminar_amistad', { amistadId });
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
  return await invoke<GrupoDetalleDto>('ver_grupo', { grupoId });
}

/** Cálculo local con decimales exactos; no usa la red. */
export async function previsualizarDivisionCommand(
  datos: PrevisualizarDivisionPayload
): Promise<PrevisualizacionDivisionDto> {
  return await invoke<PrevisualizacionDivisionDto>('previsualizar_division', { datos });
}

// ---------------------------------------------------------------------------
// Entrega 7: sincronización, cuenta del grupo y cola sin conexión
// ---------------------------------------------------------------------------

export interface ParteGrupoPendienteDto {
  evento_id: string;
  grupo_id: string;
  grupo_nombre: string;
  concepto: string;
  monto: string;
  moneda: string;
  fecha: string;
}

export interface GastoEnColaDto {
  clave: string;
  grupo_id: string;
  grupo_nombre: string;
  concepto: string;
  monto: string;
  moneda: string;
  fecha: string;
  estado: 'pendiente' | 'rechazado';
  motivo: string | null;
}

export interface EstadoSincronizacionDto {
  ultima_sincronizacion: string | null;
  sincronizando: boolean;
  partes_pendientes: ParteGrupoPendienteDto[];
  cola: GastoEnColaDto[];
}

export interface CuentaParaGrupoDto {
  id: string;
  nombre: string;
  moneda: string;
}

export interface CuentasParaGrupoDto {
  grupo_id: string;
  cuenta_elegida: string | null;
  cuentas: CuentaParaGrupoDto[];
}

export async function sincronizar(): Promise<EstadoSincronizacionDto> {
  return await invoke<EstadoSincronizacionDto>('sincronizar');
}

export async function estadoSincronizacion(): Promise<EstadoSincronizacionDto> {
  return await invoke<EstadoSincronizacionDto>('estado_sincronizacion');
}

export async function cuentasParaGrupo(
  grupo_id: string,
  moneda: string,
): Promise<CuentasParaGrupoDto> {
  return await invoke<CuentasParaGrupoDto>('cuentas_para_grupo', { datos: { grupo_id, moneda } });
}

export async function elegirCuentaGrupo(
  grupo_id: string,
  cuenta_id: string,
  moneda: string,
): Promise<CuentasParaGrupoDto> {
  return await invoke<CuentasParaGrupoDto>('elegir_cuenta_grupo', {
    datos: { grupo_id, cuenta_id, moneda },
  });
}

export async function listarPartesPendientes(): Promise<ParteGrupoPendienteDto[]> {
  return await invoke<ParteGrupoPendienteDto[]>('listar_partes_pendientes');
}

export async function reiniciarCursor(): Promise<EstadoSincronizacionDto> {
  return await invoke<EstadoSincronizacionDto>('reiniciar_cursor');
}

export interface ResultadoGastoGrupoDto {
  estado: 'registrado' | 'pendiente';
  clave: string;
  gasto: GastoGrupoDto | null;
}

export async function registrarGastoGrupoConCola(
  datos: NuevoGastoGrupoPayload,
  grupoNombre: string,
  moneda: string,
): Promise<ResultadoGastoGrupoDto> {
  return await invoke<ResultadoGastoGrupoDto>('registrar_gasto_grupo_con_cola', {
    datos,
    grupo_nombre: grupoNombre,
    moneda,
  });
}


// ---------- Comandos: copia de seguridad y sincronización del libro (Entrega 8) ----------
// Todo ocurre entre el dispositivo y un archivo o destino que elige la persona: ninguno de
// estos comandos habla con el servidor de Cuentas Claras (RNF-01, INV-16).
//
// Los comandos de esta sección (como `exportar_copia`) llevan `rename_all = "snake_case"`:
// reciben sus argumentos sueltos (`ruta`, `frase`) y los que traen un cuerpo, en `datos`.
//
// SIMULACIÓN: con `NEXT_PUBLIC_SIMULAR_INTERCAMBIO=1` (solo desarrollo) estas funciones no
// llaman a Rust: responden `utils/simulacionIntercambio.ts`, que permite probar la interfaz
// con los cinco errores de la sección 2.7 mientras el cifrado, la importación y la
// sincronización se integran. Con la bandera apagada se usan los comandos reales.

/** `true` solo si se arrancó con `NEXT_PUBLIC_SIMULAR_INTERCAMBIO=1`. Next lo fija al compilar. */
export const SIMULAR_INTERCAMBIO = process.env.NEXT_PUBLIC_SIMULAR_INTERCAMBIO === '1';

/**
 * Escribe la copia en `ruta`. Sin `frase` queda en texto legible; con `frase` va cifrada
 * (mínimo 12 caracteres). No modifica el libro (RF-INT-01).
 */
export async function exportarCopiaCommand(ruta: string, frase: string | null): Promise<void> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.exportarCopia(ruta, frase);
  return await invoke<void>('exportar_copia', { ruta, frase });
}

/**
 * Descifra y valida todo el archivo y lo fusiona con el libro en una sola transacción:
 * importar dos veces el mismo archivo no cambia nada la segunda vez (RF-INT-02, ESC-04).
 * `frase` es `null` si el archivo está en texto legible.
 */
export async function importarCopiaCommand(
  ruta: string,
  frase: string | null
): Promise<ResumenImportacionDto> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.importarCopia(ruta, frase);
  return await invoke<ResumenImportacionDto>('importar_copia', { ruta, frase });
}

/** Destino configurado (si lo hay) y fecha de la última sincronización exitosa. Sin red. */
export async function estadoSincronizacionLibroCommand(): Promise<EstadoSincronizacionLibroDto> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.estadoSincronizacionLibro();
  return await invoke<EstadoSincronizacionLibroDto>('estado_sincronizacion_libro');
}

/**
 * Guarda el destino. La frase de paso y las credenciales de Git van al almacén de secretos
 * del sistema: nunca al libro, al destino ni a la exportación (RF-INT-05).
 */
export async function configurarDestinoCommand(
  datos: ConfigurarDestinoPayload
): Promise<DestinoSincronizacionDto> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.configurarDestino(datos);
  return await invoke<DestinoSincronizacionDto>('configurar_destino_sincronizacion', { datos });
}

/** Reemplaza usuario y token del repositorio Git configurado. */
export async function actualizarCredencialesDestinoCommand(
  datos: ActualizarCredencialesPayload
): Promise<void> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.actualizarCredenciales(datos);
  return await invoke<void>('actualizar_credenciales_destino', { datos });
}

/** Quita el destino y borra del almacén de secretos su frase y credenciales. */
export async function quitarDestinoCommand(): Promise<void> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.quitarDestino();
  return await invoke<void>('quitar_destino_sincronizacion');
}

/** Trae, fusiona, envía y publica (RF-INT-03). Informa enviados, recibidos y rechazados. */
export async function sincronizarLibroCommand(): Promise<InformeSincronizacionLibroDto> {
  if (SIMULAR_INTERCAMBIO) return await simulacion.sincronizarLibro();
  return await invoke<InformeSincronizacionLibroDto>('sincronizar_libro');
}
