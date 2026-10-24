export type TipoCuenta = 'efectivo' | 'banco' | 'billetera_digital';
export type TipoMovimiento = 'gasto' | 'ingreso';

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
  tipo: TipoCuenta;
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
  tipo: string;
  monto: string;
  monto_con_signo: string;
  monto_vigente?: string;
  fecha: string;
  categoria_id: string;
  categoria_nombre: string;
  nota: string;
  estado: string; // 'normal' | 'corregido' | 'ajuste'
  corrige_a?: string;
  ajustes?: string[];
  monto_correcto?: string;
}

export interface ComposicionSaldo {
  saldo_inicial: string;
  total_ingresos: string;
  total_gastos: string;
  total_ajustes: string;
}

export interface DetalleCuentaData {
  cuenta: Cuenta;
  composicion: ComposicionSaldo;
  movimientos: Movimiento[];
}

export interface PrevisualizacionMovimiento {
  saldo_resultante: string;
  deja_saldo_negativo: boolean;
}