import { invoke } from '@tauri-apps/api/core';

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
