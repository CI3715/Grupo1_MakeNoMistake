import type { TipoCuenta } from '../types/cuentas';

/** Los tres tipos de cuenta de la Entrega 2, con el nombre que ve la persona usuaria. */
export const TIPOS: { valor: TipoCuenta; nombre: string }[] = [
  { valor: 'efectivo', nombre: 'Efectivo' },
  { valor: 'banco', nombre: 'Banco' },
  { valor: 'billetera_digital', nombre: 'Billetera Digital' },
];

export function nombreTipo(tipo: string): string {
  return TIPOS.find((t) => t.valor === tipo)?.nombre ?? tipo;
}
