/**
 * Aplica formato local a un monto en formato texto ("1250.50") junto con su símbolo/código.
 * Ejemplo: formatearMontoLocal("1250.50", "Bs.") -> "Bs. 1.250,50"
 *          formatearMontoLocal("300.00", "$") -> "$ 300,00"
 */
export function formatearMontoLocal(montoTexto: string, simboloOMoneda: string): string {
  if (!montoTexto) return `${simboloOMoneda} 0,00`;

  const partes = montoTexto.split('.');
  const entero = partes[0] || '0';
  const decimales = partes[1] || '00';

  // Formatear la parte entera añadiendo separadores de miles con punto (.)
  const enteroFormateado = entero.replace(/\B(?=(\d{3})+(?!\d))/g, '.');

  return `${simboloOMoneda} ${enteroFormateado},${decimales}`;
}