'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';
import { IconoFlechaIzquierda } from '../Iconos';

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
      className={`mb-3 inline-flex items-center gap-1.5 rounded-md text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${tema.textoEnlace}`}
    >
      <IconoFlechaIzquierda className="w-3.5 h-3.5" />
      <span>{texto}</span>
    </button>
  );
}
