'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  variante?: 'primario' | 'secundario' | 'peligro';
  tipo?: 'button' | 'submit';
  cargando?: boolean;
  deshabilitado?: boolean;
  /** Ocupa todo el ancho disponible. */
  ancho?: boolean;
  etiquetaAria?: string;
  onClick?: () => void;
  children: React.ReactNode;
}

/** Botón con estado de carga (deshabilitado + indicador) y foco visible. */
export function BotonAccion({
  modoOscuro,
  variante = 'primario',
  tipo = 'button',
  cargando = false,
  deshabilitado = false,
  ancho = false,
  etiquetaAria,
  onClick,
  children,
}: Props) {
  const tema = temaSocial(modoOscuro);
  const colores =
    variante === 'primario'
      ? 'bg-acento text-acento-fg hover:brightness-110 shadow-sm'
      : variante === 'peligro'
        ? 'border border-peligro-borde bg-peligro-suave text-peligro hover:brightness-110'
        : `border ${tema.linea} ${tema.textoEnlace} hover:bg-superficie-2`;

  return (
    <button
      type={tipo}
      onClick={onClick}
      disabled={deshabilitado || cargando}
      aria-busy={cargando || undefined}
      aria-label={etiquetaAria}
      className={`inline-flex items-center justify-center gap-2 rounded-xl px-3.5 py-2 text-xs font-semibold transition cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 focus:outline-none focus-visible:ring-2 focus-visible:ring-acento ${
        ancho ? 'w-full' : ''
      } ${colores}`}
    >
      {cargando && (
        <span
          className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-current/40 border-t-current"
          aria-hidden="true"
        />
      )}
      {children}
    </button>
  );
}
