/**
 * Helpers de PRESENTACIÓN para las categorías. Solo cambian el orden en que se
 * muestran en el selector; no deciden nada de negocio.
 */

const NOMBRES_QUE_VAN_AL_FINAL = ['otros gastos', 'otros ingresos'];

/**
 * Devuelve las categorías con «Otros gastos» / «Otros ingresos» siempre de última,
 * sin importar cuántas categorías nuevas se hayan agregado. El resto conserva el
 * orden que trae el backend.
 */
export function ordenarOtrosAlFinal<T extends { nombre: string }>(categorias: T[]): T[] {
  const esOtros = (c: T) => NOMBRES_QUE_VAN_AL_FINAL.includes(c.nombre.trim().toLowerCase());
  return [...categorias.filter((c) => !esOtros(c)), ...categorias.filter(esOtros)];
}
