'use client';

import React from 'react';
import type { EventoRechazadoDto } from '../../types/copia';
import { textoRechazados } from '../../utils/copiaFormato';
import { temaSocial } from '../../utils/temaSocial';

/** Eventos que no se aplicaron, cada uno con su motivo (sección 2.7). El resto sí se aplicó. */
export function CopiaListaRechazados({
  modoOscuro,
  rechazados,
}: {
  modoOscuro: boolean;
  rechazados: EventoRechazadoDto[];
}) {
  const tema = temaSocial(modoOscuro);
  if (rechazados.length === 0) return null;
  return (
    <div className={`rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}>
      <p className="font-semibold">
        {textoRechazados(rechazados.length)}: no se aplicaron. El resto sí.
      </p>
      <ul className="mt-2 space-y-1.5">
        {rechazados.map((r) => (
          <li key={r.id} className="min-w-0">
            <span className="font-mono text-[11px]">{r.id}</span>
            <span className="opacity-90"> · {r.motivo}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
