import type { CSSProperties } from 'react';

/** Tokens visuales semánticos basados en globals.css (temas claro/oscuro y 6 paletas). */
export function temaSesion(modoOscuro: boolean) {
  const estiloInput: CSSProperties = {
    backgroundColor: 'var(--superficie-2)',
    color: 'var(--texto)',
    borderColor: 'var(--borde)',
    colorScheme: modoOscuro ? 'dark' : 'light',
  };

  return {
    tarjeta: 'bg-superficie border-borde text-texto shadow-suave rounded-3xl',
    textoSuave: 'text-suave',
    textoEnlace: 'text-acento-texto hover:underline font-semibold',
    estiloInput,
  };
}
