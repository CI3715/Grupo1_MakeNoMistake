export type TipoMoneda = 'USD' | 'VES' | 'EUR' | 'USDT';
export type TipoMovimiento = 'ingreso' | 'gasto';
export type TipoCuenta = 'efectivo' | 'banco' | 'billetera_digital' | 'tarjeta';

export interface Moneda {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
}

export interface Cuenta {
  id: string;
  nombre: string;
  tipo: TipoCuenta;
  moneda: string;
  saldo_actual: string;
}

export interface Subtotal {
  moneda: string;
  total: string;
}

export interface ListaCuentas {
  cuentas: Cuenta[];
  subtotales: Subtotal[];
}

export interface NuevaCuenta {
  nombre: string;
  tipo: TipoCuenta | string;
  moneda: string;
  saldo_inicial: string;
}

export interface ErrorCampo {
  campo: string | null;
  mensaje: string;
}

export interface Categoria {
  id: string;
  nombre: string;
  tipo: TipoMovimiento;
  de_fabrica?: boolean;
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
  /** Viene de un gasto de grupo: no se corrige desde el libro, se corrige en el grupo. */
  es_gasto_de_grupo?: boolean;
}

export interface ComposicionSaldo {
  saldo_inicial: string;
  total_ingresos: string;
  total_gastos: string;
  total_ajustes: string;
}

export interface ComposicionSaldoData extends ComposicionSaldo {
  saldo_actual: string;
}

export interface DetalleCuentaData {
  cuenta: Cuenta;
  composicion: ComposicionSaldo;
  movimientos: Movimiento[];
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
