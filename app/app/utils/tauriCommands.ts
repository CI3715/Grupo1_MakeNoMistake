import { invoke } from '@tauri-apps/api/core';

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
  monto_vigente?: string;
  fecha: string;
  categoria_id: string;
  categoria_nombre: string;
  nota: string;
  estado: string;
  corrige_a?: string;
  ajustes: string[];
  monto_correcto?: string;
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

export interface ErrorCampo {
  campo?: string;
  mensaje: string;
}

export interface ErroresDto {
  errores: ErrorCampo[];
}

export async function listarCuentasCommand(): Promise<ListaCuentasDto> {
  return await invoke<ListaCuentasDto>('listar_cuentas');
}

export async function crearCuentaCommand(payload: {
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_inicial: string;
}): Promise<CuentaDto> {
  return await invoke<CuentaDto>('crear_cuenta', { datos: payload });
}

export async function detalleCuentaCommand(cuentaId: string): Promise<DetalleCuentaDto> {
  return await invoke<DetalleCuentaDto>('detalle_cuenta', { cuenta_id: cuentaId });
}

export async function listarCategoriasCommand(): Promise<CategoriaDto[]> {
  return await invoke<CategoriaDto[]>('listar_categorias');
}

export async function crearCategoriaCommand(nombre: string, tipo: string): Promise<CategoriaDto> {
  return await invoke<CategoriaDto>('crear_categoria', { datos: { nombre, tipo } });
}

export async function registrarMovimientoCommand(payload: {
  cuenta_id: string;
  tipo: string;
  monto: string;
  fecha: string;
  categoria_id: string;
  nota: string;
}): Promise<MovimientoDto> {
  return await invoke<MovimientoDto>('registrar_movimiento', { datos: payload });
}

export interface PrevisualizacionDto {
  saldo_resultante: string;
  deja_saldo_negativo: boolean;
}

export async function previsualizarMovimientoCommand(payload: {
  cuenta_id: string;
  tipo: string;
  monto: string;
  fecha: string;
  categoria_id: string;
  nota: string;
}): Promise<PrevisualizacionDto> {
  return await invoke<PrevisualizacionDto>('previsualizar_movimiento', { datos: payload });
}

export interface MonedaDto {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
}

// ---------- Entrega 4: tasas de cambio, moneda base y patrimonio ----------

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

export interface MonedaBaseDto {
  moneda: string;
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

export async function registrarTasaCommand(payload: {
  moneda_origen: string;
  moneda_destino: string;
  valor: string;
  fecha: string;
  fuente: string;
}): Promise<TasaDto> {
  return await invoke<TasaDto>('registrar_tasa', { datos: payload });
}

export async function eliminarTasaCommand(tasaId: string): Promise<void> {
  await invoke<void>('eliminar_tasa', { tasa_id: tasaId });
}

export async function listarTasasCommand(): Promise<HistorialParDto[]> {
  return await invoke<HistorialParDto[]>('listar_tasas');
}

export async function cambiarMonedaBaseCommand(moneda: string): Promise<MonedaBaseDto> {
  return await invoke<MonedaBaseDto>('cambiar_moneda_base', { datos: { moneda } });
}

export async function crearMonedaCommand(payload: {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: string;
}): Promise<MonedaDto> {
  return await invoke<MonedaDto>('crear_moneda', { datos: payload });
}

export async function consultarPatrimonioCommand(fecha = ''): Promise<PatrimonioDto> {
  return await invoke<PatrimonioDto>('consultar_patrimonio', { datos: { fecha } });
}

export async function listarMonedasCommand(): Promise<MonedaDto[]> {
  return await invoke<MonedaDto[]>('listar_monedas');
}
