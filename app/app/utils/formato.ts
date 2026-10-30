/**
 * Convierte una fecha en formato YYYY-MM-DD o YYYY-MM-DDTHH:mm:ss
 * a formato legible dd/mm/aaaa manipulando directamente el texto (sin usar Date).
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
 * Formatea montos numéricos o en formato texto con separadores decimales y de millar.
 */
export function formatearMontoTexto(
  monto: number | string | null | undefined,
  monedaOsimbolo: string = ''
): string {
  if (monto === null || monto === undefined) {
    return `- ${monedaOsimbolo}`.trim();
  }

  const valorNum = typeof monto === 'string' ? parseFloat(monto) : monto;

  if (isNaN(valorNum)) {
    return `0,00 ${monedaOsimbolo}`.trim();
  }

  const partes = valorNum.toFixed(2).split('.');
  const enteros = partes[0].replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const decimales = partes[1];

  return `${enteros},${decimales} ${monedaOsimbolo}`.trim();
}