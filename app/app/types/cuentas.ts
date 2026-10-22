export type TipoMoneda = 'USD' | 'VES' | 'EUR' | 'USDT' | string;
export type TipoMovimiento = 'ingreso' | 'gasto';
export type TipoCuenta = 'efectivo' | 'banco' | 'billetera' | 'tarjeta';

export interface Categoria {
  id: string;
  nombre: string;
  tipo: TipoMovimiento;
}

export interface Movimiento {
  id: string;
  cuenta_id: string;
  tipo: TipoMovimiento;
  monto: string;
  monto_formateado: string;
  fecha: string;
  categoria_id: string;
  categoria_nombre: string;
  nota?: string;
  es_corregido?: boolean;
  es_ajuste?: boolean;
  movimiento_referenciado_id?: string;
}

export interface ComposicionSaldoData {
  saldo_inicial: string;
  total_ingresos: string;
  total_gastos: string;
  total_ajustes: string;
  saldo_actual: string;
}

export interface CuentaItem {
  id: string;
  nombre: string;
  tipo: string;
  moneda: TipoMoneda;
  saldo_actual: string;
}

export interface ValidacionMovimientoError {
  monto?: string;
  categoria?: string;
  fecha?: string;
  cuenta?: string;
  general?: string;
}