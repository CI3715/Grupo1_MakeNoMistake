'use client';

import React, { useId } from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  titulo: string;
  descripcion?: string;
  /** Contenido a la derecha del título (un botón, un contador). */
  acciones?: React.ReactNode;
  children: React.ReactNode;
}

/** Tarjeta con título (h2) y región con nombre para lectores de pantalla. */
export function Tarjeta({ modoOscuro, titulo, descripcion, acciones, children }: Props) {
  const idTitulo = useId();
  const tema = temaSocial(modoOscuro);
  return (
    <section
      aria-labelledby={idTitulo}
      className={`min-w-0 rounded-2xl border p-5 shadow-sm ${tema.tarjeta}`}
    >
      <div className="mb-4 flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 id={idTitulo} className="text-base font-bold tracking-tight">
            {titulo}
          </h2>
          {descripcion && <p className={`mt-0.5 text-xs ${tema.textoSuave}`}>{descripcion}</p>}
        </div>
        {acciones}
      </div>
      {children}
    </section>
  );
}
