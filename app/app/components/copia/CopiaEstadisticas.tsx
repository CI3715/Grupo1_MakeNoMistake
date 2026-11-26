'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Dato {
  etiqueta: string;
  valor: number;
}

/** Fila de contadores (nuevos / ya existían / rechazados, o enviados / recibidos / rechazados). */
export function CopiaEstadisticas({ modoOscuro, datos }: { modoOscuro: boolean; datos: Dato[] }) {
  const tema = temaSocial(modoOscuro);
  return (
    <dl className="grid grid-cols-3 gap-2">
      {datos.map((d) => (
        <div key={d.etiqueta} className={`rounded-xl border px-3 py-2.5 ${tema.interior}`}>
          <dt className={`text-[11px] font-semibold ${tema.textoSuave}`}>{d.etiqueta}</dt>
          <dd className="mt-0.5 text-lg font-bold tabular-nums">{d.valor}</dd>
        </div>
      ))}
    </dl>
  );
}
