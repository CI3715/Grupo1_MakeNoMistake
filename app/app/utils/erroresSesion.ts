import { esErroresDto } from './tauriCommands';
import type { ErroresSesion } from '../types/sesion';

/** Mensaje de último recurso: nunca se muestra el texto crudo de un error técnico (RNF-18). */
export const MENSAJE_GENERICO = 'No se pudo completar la acción. Intenta de nuevo en un momento.';

/**
 * Convierte el error de un comando en errores por campo para un formulario.
 * - Un error con `campo` conocido va debajo de ese campo.
 * - Cualquier otro (sin campo, sin conexión, sesión vencida…) va como «general».
 * - Si el error no tiene la forma {errores: [...]}, se usa un mensaje genérico:
 *   así nunca aparece texto técnico en pantalla.
 */
export function erroresDeFormulario<C extends string>(
  err: unknown,
  campos: readonly C[]
): ErroresSesion<C> {
  const resultado: Record<string, string> = {};

  if (!esErroresDto(err) || err.errores.length === 0) {
    resultado.general = MENSAJE_GENERICO;
    return resultado as unknown as ErroresSesion<C>;
  }

  const generales: string[] = [];
  for (const e of err.errores) {
    const campo = campos.find((c) => c === e.campo);
    if (campo !== undefined) {
      if (!resultado[campo]) resultado[campo] = e.mensaje;
    } else {
      generales.push(e.mensaje);
    }
  }
  if (generales.length > 0) resultado.general = generales.join(' ');
  return resultado as unknown as ErroresSesion<C>;
}

/** Primer mensaje de un error de comando, o el genérico. */
export function primerMensaje(err: unknown): string {
  if (esErroresDto(err) && err.errores.length > 0) return err.errores[0].mensaje;
  return MENSAJE_GENERICO;
}
