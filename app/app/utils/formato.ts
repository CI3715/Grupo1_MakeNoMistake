export function formatearMontoTexto(
  montoTexto: string,
  tipo: 'ingreso' | 'gasto' | 'ajuste' | 'neutral',
  simboloMoneda: string = '$'
): string {
  if (!montoTexto) return `${simboloMoneda} 0,00`;

  let limpio = montoTexto.trim();
  const esNegativo = limpio.startsWith('-') || limpio.startsWith('−');

  if (esNegativo) {
    limpio = limpio.substring(1).trim();
  } else if (limpio.startsWith('+')) {
    limpio = limpio.substring(1).trim();
  }

  if (limpio.includes('.') && !limpio.includes(',')) {
    const partes = limpio.split('.');
    limpio = `${partes[0].replace(/\B(?=(\d{3})+(?!\d))/g, '.')},${partes[1]}`;
  }

  const signoPrefix = tipo === 'gasto' || esNegativo ? '− ' : tipo === 'ingreso' ? '+ ' : '';
  return `${signoPrefix}${simboloMoneda} ${limpio}`;
}