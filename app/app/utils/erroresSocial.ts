import type { ErroresCampos, FalloSocial, TipoFallo } from '../types/social';
import { MENSAJE_GENERICO, erroresDeFormulario } from './erroresSesion';
import { esErroresDto } from './tauriCommands';

/**
 * Los comandos no marcan el tipo de fallo, solo el mensaje (en español, sin trazas).
 * Se reconoce por los textos acordados: «No hay conexión…», «Tu sesión venció…» y
 * «No has iniciado sesión.». Cualquier otro mensaje es un error normal.
 */
function tipoPorMensaje(mensaje: string): TipoFallo {
  const texto = mensaje.toLowerCase();
  if (texto.includes('no hay conexión')) return 'sin_conexion';
  if (texto.includes('sesión venció')) return 'vencida';
  if (texto.includes('no has iniciado sesión')) return 'sin_sesion';
  return 'otro';
}

/** Fallo de una carga: el primer error sin campo (o el primero) y su tipo. */
export function clasificarFallo(err: unknown): FalloSocial {
  if (!esErroresDto(err) || err.errores.length === 0) {
    return { tipo: 'otro', mensaje: MENSAJE_GENERICO };
  }
  const general = err.errores.find((e) => !e.campo) ?? err.errores[0];
  return { tipo: tipoPorMensaje(general.mensaje), mensaje: general.mensaje };
}

export interface ResultadoError<C extends string> {
  errores: ErroresCampos<C>;
  tipo: TipoFallo;
}

/** Errores de una acción de formulario: uno por campo y un «general» para el resto. */
export function erroresDeAccion<C extends string>(
  err: unknown,
  campos: readonly C[]
): ResultadoError<C> {
  return { errores: erroresDeFormulario(err, campos), tipo: clasificarFallo(err).tipo };
}

/**
 * Igual que `erroresDeAccion`, pero para formularios de UN solo dato (la persona escrita).
 * Algunos rechazos del servidor llegan sin campo («Usuario no encontrado.», «No es tu amigo»,
 * «Ya es miembro»…) aunque hablan de ese dato: se muestran debajo de `campoPorDefecto`.
 * Sin sesión, sesión vencida, sin conexión y el mensaje genérico siguen como «general»,
 * porque no son del campo (y algunos llevan su propio botón).
 */
export function erroresDeAccionEnCampo<C extends string>(
  err: unknown,
  campos: readonly C[],
  campoPorDefecto: C
): ResultadoError<C> {
  const r = erroresDeAccion(err, campos);
  const general = r.errores.general;
  if (r.tipo !== 'otro' || !general || general === MENSAJE_GENERICO) return r;
  const copia = { ...(r.errores as unknown as Record<string, string | undefined>) };
  delete copia.general;
  if (!copia[campoPorDefecto]) copia[campoPorDefecto] = general;
  return { tipo: r.tipo, errores: copia as unknown as ErroresCampos<C> };
}
