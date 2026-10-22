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

/**
 * Convierte lo que escribe la persona ("1250,50", "1.250,50", "1250.50") al texto
 * decimal con punto que espera el backend ("1250.50"). Solo normaliza el separador;
 * todas las reglas de validación viven en el dominio (Rust).
 */
export function normalizarSaldo(texto: string): string {
  const limpio = texto.trim();
  if (limpio.includes(',')) {
    return limpio.replace(/\./g, '').replace(',', '.');
  }
  return limpio;
}
