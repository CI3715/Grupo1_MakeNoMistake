/**
 * Helpers de PRESENTACIÓN. Aquí no hay reglas de negocio ni aritmética de dinero:
 * los montos llegan como texto ("1250.50") y solo se les cambia el formato.
 */

/** Fecha local del dispositivo como AAAA-MM-DD (no usa UTC, que puede dar «mañana»). */
export function fechaHoyLocal(): string {
  const hoy = new Date();
  const a = hoy.getFullYear();
  const m = String(hoy.getMonth() + 1).padStart(2, '0');
  const d = String(hoy.getDate()).padStart(2, '0');
  return `${a}-${m}-${d}`;
}

/** Acepta la coma decimal que escribe la persona y la entrega con punto, como texto. */
export function normalizarDecimal(texto: string): string {
  return texto.trim().replace(',', '.');
}

/**
 * Convierte AAAA-MM-DD (o AAAA-MM-DDTHH:mm:ss) a dd/mm/aaaa manipulando el texto.
 */
export function formatearFechaDDMMYYYY(fechaTexto: string | null | undefined): string {
  if (!fechaTexto) return '';
  const parteFecha = fechaTexto.trim().split('T')[0];
  const partes = parteFecha.split('-');

  if (partes.length !== 3) return fechaTexto;

  const [anio, mes, dia] = partes;
  return `${dia.padStart(2, '0')}/${mes.padStart(2, '0')}/${anio}`;
}

/**
 * Da formato local (1.250,50) a un monto en texto, sin pasar por números de coma
 * flotante: conserva exactamente los decimales que envió el backend.
 */
export function formatearMontoTexto(
  monto: string | null | undefined,
  monedaOsimbolo: string = ''
): string {
  if (monto === null || monto === undefined || monto.trim() === '') {
    return `- ${monedaOsimbolo}`.trim();
  }

  const coincidencia = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(monto.trim());
  if (!coincidencia) return `${monto} ${monedaOsimbolo}`.trim();

  const [, signo, enteros, decimales] = coincidencia;
  const miles = enteros.replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const cuerpo = decimales ? `${miles},${decimales}` : miles;

  return `${signo === '-' ? '-' : ''}${cuerpo} ${monedaOsimbolo}`.trim();
}
