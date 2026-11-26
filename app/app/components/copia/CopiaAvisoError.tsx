'use client';

import React from 'react';
import type { ErrorIntercambio } from '../../types/copia';
import { efectoEnElLibro } from '../../utils/erroresCopia';
import { temaSocial } from '../../utils/temaSocial';
import { BotonAccion } from '../social/BotonAccion';

interface Props {
  modoOscuro: boolean;
  error: ErrorIntercambio;
  /** Dónde ocurrió: cambia lo que se dice sobre el libro. */
  contexto: 'copia' | 'sincronizacion';
  /** Si se da, los errores que se resuelven reintentando ofrecen «Reintentar». */
  onReintentar?: () => void;
  /** Si se da, las credenciales rechazadas ofrecen «Actualizar credenciales». */
  onActualizarCredenciales?: () => void;
}

/**
 * Error de copia o sincronización (sección 2.7): dice qué ocurrió, qué pasó con el libro y qué
 * hacer, sin trazas técnicas (RNF-18). Los fallos que se resuelven solos con el tiempo (sin
 * conexión, destino inaccesible) van en ámbar; el resto, en rojo.
 */
export function CopiaAvisoError({
  modoOscuro,
  error,
  contexto,
  onReintentar,
  onActualizarCredenciales,
}: Props) {
  const tema = temaSocial(modoOscuro);
  const efecto = efectoEnElLibro(error.tipo, contexto);
  const colores = error.reintentable ? tema.avisoAmbar : tema.avisoError;

  return (
    <div role="alert" className={`rounded-xl border p-3 text-xs ${colores}`}>
      <p className="font-medium">{error.mensaje}</p>
      {efecto && <p className="mt-1 opacity-90">{efecto}</p>}
      {(error.reintentable && onReintentar) ||
      (error.tipo === 'credenciales_rechazadas' && onActualizarCredenciales) ? (
        <div className="mt-3 flex flex-wrap gap-2">
          {error.reintentable && onReintentar && (
            <BotonAccion modoOscuro={modoOscuro} variante="secundario" onClick={onReintentar}>
              Reintentar
            </BotonAccion>
          )}
          {error.tipo === 'credenciales_rechazadas' && onActualizarCredenciales && (
            <BotonAccion modoOscuro={modoOscuro} onClick={onActualizarCredenciales}>
              Actualizar credenciales
            </BotonAccion>
          )}
        </div>
      ) : null}
    </div>
  );
}
