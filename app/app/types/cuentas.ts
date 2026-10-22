export type TipoCuenta = 'efectivo' | 'banco' | 'billetera_digital';

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
  campo: 'nombre' | 'moneda' | 'saldo_inicial' | 'tipo' | null;
  mensaje: string;
}