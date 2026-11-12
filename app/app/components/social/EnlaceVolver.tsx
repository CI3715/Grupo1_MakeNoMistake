'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  texto: string;
  onClick: () => void;
}

/** «← Volver…» como botón de texto con foco visible. */
export function EnlaceVolver({ modoOscuro, texto, onClick }: Props) {
  const tema = temaSocial(modoOscuro);
  return (
    <button
      type="button"
      onClick={onClick}
      className={`mb-3 rounded-md text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${tema.textoEnlace}`}
    >
      <span aria-hidden="true">← </span>
      {texto}
    </button>
  );
}
