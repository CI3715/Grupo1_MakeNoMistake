import { formatearMontoTexto } from './formato';
import type { ReglaDivision } from './tauriCommands';

/**
 * Helpers de PRESENTACIÓN de Amigos y Grupos. No suman ni dividen nada: los montos,
 * partes y balances los calcula el núcleo y aquí solo se les da formato.
 */

export const REGLAS: readonly ReglaDivision[] = ['partes_iguales', 'porcentajes', 'montos_fijos'];

export const ETIQUETA_REGLA: Record<ReglaDivision, string> = {
  partes_iguales: 'Partes iguales',
  porcentajes: 'Porcentajes',
  montos_fijos: 'Montos fijos',
};

/** Marca que se muestra junto a la parte que recibió un céntimo del residuo. */
export const TEXTO_REDONDEO = 'incluye 0,01 de redondeo';

export type SignoBalance = 'positivo' | 'negativo' | 'cero';

const ES_CERO = /^[+-]?0+(\.0+)?$/;

export function signoBalance(monto: string): SignoBalance {
  const texto = monto.trim();
  if (ES_CERO.test(texto)) return 'cero';
  return texto.startsWith('-') ? 'negativo' : 'positivo';
}

/** «36.66» → «+36,66 USD»; «-48.33» → «−48,33 USD»; «0.00» → «0,00 USD». */
export function formatearBalance(monto: string, moneda: string): string {
  const base = formatearMontoTexto(monto, moneda);
  switch (signoBalance(monto)) {
    case 'negativo':
      return `−${base.replace(/^-/, '')}`;
    case 'positivo':
      return `+${base}`;
    default:
      return base;
  }
}

/** Primera letra en mayúscula para el avatar. */
export function inicialDe(texto: string): string {
  return texto.trim().charAt(0).toUpperCase() || '?';
}

/** El servidor y la interfaz acuerdan nombres de usuario en minúsculas y sin espacios sobrantes. */
export function normalizarUsuario(texto: string): string {
  return texto.trim().toLowerCase();
}

/** «Una persona» / «3 personas». */
export function plural(cantidad: number, singular: string, pluralTexto: string): string {
  return `${cantidad} ${cantidad === 1 ? singular : pluralTexto}`;
}
