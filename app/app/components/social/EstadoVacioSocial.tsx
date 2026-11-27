'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  titulo: string;
  texto: string;
}

/** Mensaje de lista vacía: dice qué falta y qué hacer. */
export function EstadoVacioSocial({ modoOscuro, titulo, texto }: Props) {
  const tema = temaSocial(modoOscuro);
  return (
    <div
      className={`rounded-xl border border-dashed p-5 text-center ${tema.linea} ${tema.textoSuave}`}
    >
      <p className="text-sm font-semibold">{titulo}</p>
      <p className="mt-1 text-xs">{texto}</p>
    </div>
  );
}
