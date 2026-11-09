'use client';

import React from 'react';

interface Props {
  onClick: () => void;
  /** Resalta el botón cuando la pantalla de sesión está abierta. */
  activo?: boolean;
}

/** Botón «Iniciar sesión» de la barra lateral. Solo navega; no sabe nada de la sesión. */
export const BotonIniciarSesion: React.FC<Props> = ({ onClick, activo = false }) => (
  <button
    type="button"
    onClick={onClick}
    aria-current={activo ? 'page' : undefined}
    className={`w-full flex items-center justify-center gap-2 px-3.5 py-2.5 rounded-xl text-sm font-medium border transition cursor-pointer ${
      activo
        ? 'bg-[#9333EA] border-[#9333EA] text-white shadow-md shadow-purple-900/50'
        : 'border-purple-700/70 text-purple-100 hover:bg-white/10'
    }`}
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="12" cy="8" r="4" />
      <path d="M4 21c0-4.4 3.6-8 8-8s8 3.6 8 8" />
    </svg>
    Iniciar sesión
  </button>
);
