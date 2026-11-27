'use client';

import React from 'react';
import {
  DISENOS_ESCRITORIO,
  DisenoEscritorio,
  guardarApariencia,
  aplicarApariencia,
  leerApariencia,
} from '../utils/apariencia';

interface Props {
  disenoActual: DisenoEscritorio;
  alCambiarDiseno?: (nuevo: DisenoEscritorio) => void;
  variante?: 'traslucido' | 'superficie';
}

export function ConmutadorDiseno({
  disenoActual,
  alCambiarDiseno,
  variante = 'traslucido',
}: Props) {
  const seleccionar = (d: DisenoEscritorio) => {
    alCambiarDiseno?.(d);
    const actual = leerApariencia();
    const nueva = { ...actual, diseno: d };
    aplicarApariencia(nueva);
    guardarApariencia(nueva);
  };

  const estiloContenedor =
    variante === 'traslucido'
      ? 'bg-black/25 ring-1 ring-white/20 backdrop-blur-md p-1'
      : 'bg-superficie-2 border border-borde p-1';

  const estiloBotonActivo =
    variante === 'traslucido'
      ? 'bg-white text-slate-950 shadow-xs font-bold'
      : 'bg-superficie text-texto shadow-xs font-bold';

  const estiloBotonInactivo =
    variante === 'traslucido'
      ? 'text-white/80 hover:bg-white/15 hover:text-white font-medium'
      : 'text-suave hover:text-texto hover:bg-superficie/60 font-medium';

  return (
    <div
      role="group"
      aria-label="Conmutar diseño de escritorio"
      className={`inline-flex items-center gap-1 rounded-xl ${estiloContenedor}`}
    >
      <span className="hidden sm:inline-block px-2 text-[11px] font-bold uppercase tracking-wider text-suave opacity-80">
        Diseño:
      </span>
      {DISENOS_ESCRITORIO.map((d) => {
        const activo = disenoActual === d.id;
        return (
          <button
            key={d.id}
            type="button"
            title={d.descripcion}
            aria-pressed={activo}
            onClick={() => seleccionar(d.id)}
            className={`cursor-pointer rounded-lg px-2.5 py-1 text-xs transition ${
              activo ? estiloBotonActivo : estiloBotonInactivo
            }`}
          >
            {d.nombre.replace('Barra ', '')}
          </button>
        );
      })}
    </div>
  );
}
