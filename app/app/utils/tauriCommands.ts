import { invoke } from '@tauri-apps/api/core';

// --- DTOs Cuentas, Movimientos y Categorías ---
export interface CuentaDto {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_actual: string | number;
}

export interface CategoriaDto {
  id: string;
  nombre: string;
  tipo: string;
  de_fabrica: boolean;
}

export interface MovimientoDto {
  id: string;
  fecha: string;
  tipo: string;
  monto: number;
  categoria: string;
  descripcion: string;
  en_orden: boolean;
  monto_corregido?: number | null;
  categoria_corregida?: string | null;
}

export interface DetalleCuentaDto {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_actual: number;
  movimientos: MovimientoDto[];
}

export interface ErrorCampo {
  campo: string;
  mensaje: string;
}

export interface ErroresDto {
  errores: ErrorCampo[];
}

// --- DTOs Monedas, Tasas y Patrimonio ---
export interface MonedaDto {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
}

export interface RegistrarTasaPayload {
  moneda_origen: string;
  moneda_destino: string;
  valor: number | string;
  fecha: string;
  fuente: string;
}

export interface TasaHistorialDto {
  id: string;
  par?: string;
  moneda_origen: string;
  moneda_destino: string;
  valor: number | string;
  fecha: string;
  fuente: string;
}

export interface HistorialParDto {
  par: string;
  tasas: TasaHistorialDto[];
}

export interface CuentaPatrimonioDto {
  id: string;
  nombre: string;
  moneda: string;
  saldo_nativo: number;
  en_moneda_base: number | null;
}

export interface TasaUsadaDto {
  par: string;
  valor: number;
  fecha: string;
  fuente: string;
  es_inversa: boolean;
}

export type TasaFaltanteDto = string | { moneda_origen: string; moneda_destino: string };

export interface RespuestaPatrimonioDto {
  total: number | null;
  moneda_base: string;
  cuentas: CuentaPatrimonioDto[];
  subtotales: Record<string, number>;
  tasas_usadas: TasaUsadaDto[];
  faltantes: TasaFaltanteDto[];
}

export interface CrearMonedaPayload {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number | string;
}

// --- Comandos Tauri para Cuentas, Categorías y Movimientos ---

export async function listarCuentasCommand(): Promise<CuentaDto[]> {
  return await invoke<CuentaDto[]>('listar_cuentas');
}

export async function detalleCuentaCommand(cuentaId: string): Promise<DetalleCuentaDto> {
  return await invoke<DetalleCuentaDto>('detalle_cuenta', { cuenta_id: cuentaId });
}

export async function crearCuentaCommand(payload: {
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_inicial: number;
}): Promise<CuentaDto> {
  return await invoke<CuentaDto>('crear_cuenta', { payload });
}

export async function listarCategoriasCommand(): Promise<CategoriaDto[]> {
  return await invoke<CategoriaDto[]>('listar_categorias');
}

export async function crearCategoriaCommand(payload: {
  nombre: string;
  tipo: string;
}): Promise<CategoriaDto> {
  return await invoke<CategoriaDto>('crear_categoria', { payload });
}

export async function registrarMovimientoCommand(payload: {
  cuenta_id: string;
  fecha: string;
  tipo: string;
  monto: number;
  categoria: string;
  descripcion?: string;
}): Promise<void> {
  return await invoke<void>('registrar_movimiento', { payload });
}

export async function previsualizarMovimientoCommand(payload: any): Promise<any> {
  return await invoke<any>('previsualizar_movimiento', { payload });
}

// --- Comandos Tauri para Monedas, Tasas y Patrimonio ---

export async function listarMonedasCommand(): Promise<MonedaDto[]> {
  return await invoke<MonedaDto[]>('listar_monedas');
}

export async function registrarTasaCommand(payload: RegistrarTasaPayload): Promise<void> {
  return await invoke<void>('registrar_tasa', {
    datos: {
      ...payload,
      valor: String(payload.valor).replace(',', '.'), // Fuerza conversión a string para Serde
    },
  });
}

export async function listarTasasCommand(): Promise<HistorialParDto[]> {
  return await invoke<HistorialParDto[]>('listar_tasas');
}

export async function eliminarTasaCommand(tasaId: string): Promise<void> {
  return await invoke<void>('eliminar_tasa', { tasa_id: tasaId });
}

export async function consultarPatrimonioCommand(fecha: string): Promise<RespuestaPatrimonioDto> {
  return await invoke<RespuestaPatrimonioDto>('consultar_patrimonio', {
    datos: { fecha: fecha.trim().split('T')[0] },
  });
}

export async function cambiarMonedaBaseCommand(moneda: string): Promise<void> {
  return await invoke<void>('cambiar_moneda_base', { datos: { moneda } });
}

export async function crearMonedaCommand(payload: CrearMonedaPayload): Promise<MonedaDto> {
  return await invoke<MonedaDto>('crear_moneda', {
    datos: {
      ...payload,
      decimales: String(payload.decimales), // Convierte decimales a string para Serde
    },
  });
}