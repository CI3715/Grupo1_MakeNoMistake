'use client';

import React from 'react';

interface Props {
  modoOscuro: boolean;
  /** En la pantalla de inicio de sesión no hace falta el botón. */
  mostrarBoton: boolean;
  onIniciarSesion: () => void;
  onCerrar: () => void;
}

/** Aviso no bloqueante: las cuentas, movimientos, tasas y patrimonio siguen disponibles. */
export const AvisoSesionVencida: React.FC<Props> = ({
  modoOscuro,
  mostrarBoton,
  onIniciarSesion,
  onCerrar,
}) => (
  <div
    role="status"
    className={`mb-4 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border px-4 py-3 text-xs ${
      modoOscuro
        ? 'border-amber-700/60 bg-amber-950/40 text-amber-100'
        : 'border-amber-300 bg-amber-50 text-amber-900'
    }`}
  >
    <p className="min-w-0 flex-1 font-medium">
      Tu sesión venció, inicia sesión de nuevo. Mientras tanto puedes seguir usando la aplicación.
    </p>
    {mostrarBoton && (
      <button
        type="button"
        onClick={onIniciarSesion}
        className="rounded-lg bg-[#9333EA] px-3 py-1.5 font-semibold text-white hover:bg-purple-600 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
      >
        Iniciar sesión
      </button>
    )}
    <button
      type="button"
      onClick={onCerrar}
      aria-label="Cerrar aviso"
      className="rounded-md px-2 py-1 font-bold opacity-70 hover:opacity-100 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
    >
      ✕
    </button>
  </div>
);
