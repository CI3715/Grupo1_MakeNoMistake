'use client';

import React from 'react';
import type { ContextoSocial, TipoFallo } from '../../types/social';
import { temaSocial } from '../../utils/temaSocial';
import { BotonAccion } from './BotonAccion';

interface Props {
  ctx: ContextoSocial;
  mensaje: string;
  tipo?: TipoFallo;
}

/**
 * Error que no es de un campo (sin conexión, sesión vencida…). Nunca muestra trazas:
 * el mensaje ya viene en español desde el núcleo. Si falta la sesión, ofrece iniciarla.
 */
export function MensajeGeneral({ ctx, mensaje, tipo = 'otro' }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  const pideSesion = tipo === 'sin_sesion' || tipo === 'vencida';
  return (
    <div
      role="alert"
      className={`flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border p-3 text-xs ${tema.avisoError}`}
    >
      <p className="min-w-0 flex-1">{mensaje}</p>
      {pideSesion && (
        <BotonAccion modoOscuro={ctx.modoOscuro} onClick={ctx.irAInicioSesion}>
          Iniciar sesión
        </BotonAccion>
      )}
    </div>
  );
}
