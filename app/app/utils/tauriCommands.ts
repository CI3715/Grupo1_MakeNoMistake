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
