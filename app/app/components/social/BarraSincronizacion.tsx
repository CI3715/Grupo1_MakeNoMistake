'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { formatearFechaHora } from '../../utils/formato';
import { temaSocial } from '../../utils/temaSocial';
import { BotonAccion } from './BotonAccion';

/** Texto del aviso de fallo. Solo la sincronización habla de «sin conexión»: lo personal no. */
function textoDelFallo(tipo: string, mensaje: string): string {
  switch (tipo) {
    case 'sin_conexion':
      return `${mensaje} Tus cuentas y movimientos siguen funcionando.`;
    case 'sin_sesion':
      return 'Inicia sesión para sincronizar. Lo personal funciona igual sin sesión.';
    default:
      return mensaje;
  }
}

/**
 * Botón «Sincronizar» con la fecha de la última sincronización exitosa y, discreto, «Volver
 * a bajar todo» (pone el cursor en cero: lo que ya está en el libro se ignora, no se duplica).
 */
export function BarraSincronizacion({ ctx }: { ctx: ContextoSocial }) {
  const s = ctx.sincronizacion;
  const tema = temaSocial(ctx.modoOscuro);
  const ultima = s.estado?.ultima_sincronizacion ?? null;

  const estadoTexto = s.sincronizando
    ? 'Sincronizando…'
    : ultima
      ? `Última sincronización exitosa: ${formatearFechaHora(ultima)}`
      : 'Todavía no hay una sincronización exitosa.';

  const pideSesion = s.fallo?.tipo === 'vencida' || s.fallo?.tipo === 'sin_sesion';

  return (
    <section
      aria-label="Sincronización"
      className={`min-w-0 rounded-2xl border p-4 text-xs ${tema.avisoMorado}`}
    >
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
        <div className="min-w-0">
          <p className="font-semibold">Tus partes bajan del servidor</p>
          <p role="status" aria-live="polite" className="mt-0.5 opacity-90">
            {estadoTexto}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={() => void s.volverABajarTodo()}
            disabled={s.sincronizando}
            title="Pide de nuevo todos tus gastos de grupo. Los que ya están en tu libro se ignoran: no se duplican."
            className="cursor-pointer text-[11px] underline-offset-2 opacity-80 hover:underline hover:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            Volver a bajar todo
          </button>
          <BotonAccion
            modoOscuro={ctx.modoOscuro}
            variante="secundario"
            cargando={s.sincronizando}
            onClick={() => void s.sincronizar()}
          >
            Sincronizar
          </BotonAccion>
        </div>
      </div>

      {s.fallo && (
        <div
          role="alert"
          className={`mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border px-3 py-2 ${
            s.fallo.tipo === 'sin_conexion' ? tema.avisoAmbar : tema.avisoError
          }`}
        >
          <p className="min-w-0 flex-1 font-medium">{textoDelFallo(s.fallo.tipo, s.fallo.mensaje)}</p>
          {pideSesion && (
            <button
              type="button"
              onClick={ctx.irAInicioSesion}
              className="cursor-pointer rounded-lg bg-acento px-3 py-1.5 font-semibold text-acento-fg hover:brightness-110 focus:outline-none focus-visible:ring-2 focus-visible:ring-acento"
            >
              Iniciar sesión
            </button>
          )}
        </div>
      )}
    </section>
  );
}
