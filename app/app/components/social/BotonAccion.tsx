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
      ? 'bg-[#9333EA] text-white hover:bg-purple-600 shadow-sm'
      : variante === 'peligro'
        ? modoOscuro
          ? 'border border-rose-700/70 text-rose-300 hover:bg-rose-950/40'
          : 'border border-rose-300 text-rose-700 hover:bg-rose-50'
        : `border ${tema.linea} ${tema.textoEnlace} ${modoOscuro ? 'hover:bg-white/5' : 'hover:bg-slate-50'}`;

  return (
    <button
      type={tipo}
      onClick={onClick}
      disabled={deshabilitado || cargando}
      aria-busy={cargando || undefined}
      aria-label={etiquetaAria}
      className={`inline-flex items-center justify-center gap-2 rounded-xl px-3.5 py-2 text-xs font-semibold transition cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
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
