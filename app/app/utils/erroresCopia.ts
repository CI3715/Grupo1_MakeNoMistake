import type { ErrorIntercambio, TipoErrorIntercambio } from '../types/copia';
import { MENSAJE_GENERICO, registrarErrorCrudo } from './erroresSesion';
import { esErroresDto } from './tauriCommands';

/** Texto exacto de la sección 2.7 para una frase incorrecta o un archivo alterado. */
export const MENSAJE_FRASE_INCORRECTA = 'Frase de paso incorrecta o archivo dañado.';

const CODIGOS: readonly TipoErrorIntercambio[] = [
  'frase_incorrecta',
  'frase_requerida',
  'frase_corta',
  'version_no_soportada',
  'archivo_invalido',
  'destino_inaccesible',
  'sin_conexion',
  'credenciales_rechazadas',
];

function tipoPorCodigo(codigo: string | null | undefined): TipoErrorIntercambio | null {
  return CODIGOS.find((c) => c === codigo) ?? null;
}

/**
 * Cuando el núcleo no manda `codigo`, el tipo se reconoce por los textos de
 * `aplicacion::respaldo` y `aplicacion::transporte`. Si cambia uno de esos textos, hay que
 * cambiarlo aquí (o pedir el `codigo`).
 */
function tipoPorMensaje(mensaje: string): TipoErrorIntercambio {
  const texto = mensaje.toLowerCase();
  if (texto.includes('frase de paso incorrecta')) return 'frase_incorrecta';
  if (texto.includes('está cifrado')) return 'frase_requerida';
  if (texto.includes('al menos 12 caracteres')) return 'frase_corta';
  if (texto.includes('versión') && texto.includes('soporta')) return 'version_no_soportada';
  if (texto.includes('no tiene el formato')) return 'archivo_invalido';
  if (texto.includes('credenciales') && texto.includes('rechazad')) return 'credenciales_rechazadas';
  if (texto.includes('sin conexión')) return 'sin_conexion';
  if (texto.includes('no se pudo llegar')) return 'destino_inaccesible';
  return 'otro';
}

/**
 * Convierte el error de un comando de copia o sincronización en algo que la interfaz sabe
 * explicar, sin trazas técnicas (RNF-18). Si el error no tiene la forma esperada, muestra
 * un mensaje genérico y deja el detalle en la consola solo en desarrollo.
 */
export function clasificarErrorIntercambio(err: unknown): ErrorIntercambio {
  if (!esErroresDto(err) || err.errores.length === 0) {
    registrarErrorCrudo(err);
    return { tipo: 'otro', mensaje: MENSAJE_GENERICO, reintentable: false };
  }
  const primero = err.errores.find((e) => !e.campo) ?? err.errores[0];
  const tipo = tipoPorCodigo(primero.codigo) ?? tipoPorMensaje(primero.mensaje);
  const mensaje = tipo === 'frase_incorrecta' ? MENSAJE_FRASE_INCORRECTA : primero.mensaje;
  return {
    tipo,
    mensaje,
    reintentable: tipo === 'sin_conexion' || tipo === 'destino_inaccesible',
  };
}

/** Qué pasó con el libro, para decírselo a la persona junto al error (sección 2.7). */
export function efectoEnElLibro(
  tipo: TipoErrorIntercambio,
  contexto: 'copia' | 'sincronizacion'
): string | null {
  if (tipo === 'otro') return null;
  if (contexto === 'sincronizacion' && (tipo === 'sin_conexion' || tipo === 'destino_inaccesible')) {
    return 'Tu libro no cambió. Lo pendiente se conserva y se enviará en la próxima sincronización.';
  }
  return 'Tu libro no cambió.';
}
