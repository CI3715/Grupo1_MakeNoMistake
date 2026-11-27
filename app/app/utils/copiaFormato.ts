/** Helpers de PRESENTACIÓN de la copia de seguridad y la sincronización del libro. */

/**
 * Espejo de `aplicacion::respaldo::LARGO_MINIMO_FRASE`. Solo adelanta el aviso en el
 * formulario: el núcleo vuelve a validar y es quien decide.
 */
export const LARGO_MINIMO_FRASE = 12;

/** Cuenta caracteres como lo hace Rust (`chars().count()`), no unidades UTF-16. */
export function largoDeFrase(frase: string): number {
  return [...frase].length;
}

export const MENSAJE_FRASE_CORTA = `La frase debe tener al menos ${LARGO_MINIMO_FRASE} caracteres.`;

function cantidad(n: number, singular: string, plural: string): string {
  return n === 1 ? `1 ${singular}` : `${n} ${plural}`;
}

export const textoNuevos = (n: number) => cantidad(n, 'nuevo', 'nuevos');
export const textoYaExistian = (n: number) => cantidad(n, 'ya existía', 'ya existían');
export const textoRechazados = (n: number) => cantidad(n, 'rechazado', 'rechazados');
export const textoEnviados = (n: number) => cantidad(n, 'enviado', 'enviados');
export const textoRecibidos = (n: number) => cantidad(n, 'recibido', 'recibidos');
