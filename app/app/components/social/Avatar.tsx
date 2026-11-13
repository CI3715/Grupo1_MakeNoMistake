import React from 'react';
import { inicialDe } from '../../utils/formatoSocial';

/** Círculo con la inicial de la persona. Decorativo: el nombre ya está en el texto. */
export function Avatar({ nombre }: { nombre: string }) {
  return (
    <span
      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-[#9333EA] text-sm font-bold text-white"
      aria-hidden="true"
    >
      {inicialDe(nombre)}
    </span>
  );
}
