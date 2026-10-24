import type { TipoCuenta } from '../types/cuentas';

/** Los tipos de cuenta con el nombre legible para la persona usuaria. */
export const TIPOS: { valor: TipoCuenta; nombre: string }[] = [
  { valor: 'efectivo', nombre: 'Efectivo' },
  { valor: 'banco', nombre: 'Banco' },
  { valor: 'billetera_digital', nombre: 'Billetera Digital' },
];

export function nombreTipo(tipo: string): string {
  return TIPOS.find((t) => t.valor === tipo)?.nombre ?? tipo;
}
