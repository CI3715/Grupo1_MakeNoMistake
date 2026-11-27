'use client';

import React, { useState } from 'react';
import type { ControlSesion } from '../../utils/useSesion';
import { BotonIniciarSesion } from './BotonIniciarSesion';

interface Props {
  control: ControlSesion;
  pantallaActual: string;
  setPantallaActual: (pantalla: string) => void;
}

/**
 * Zona de sesión de la barra lateral:
 * - sin sesión o vencida: botón «Iniciar sesión»;
 * - sesión vigente: nombre visible (lleva al perfil) y «Cerrar sesión».
 */
export const BloqueSesionSidebar: React.FC<Props> = ({ control, pantallaActual, setPantallaActual }) => {
  const [cerrando, setCerrando] = useState(false);
  const [errorCierre, setErrorCierre] = useState<string | null>(null);
  const { sesion } = control;

  if (sesion.estado === 'cargando') {
    return <div className="h-10 rounded-xl bg-white/10 animate-pulse" aria-hidden="true" />;
  }

  if (sesion.estado !== 'vigente') {
    return (
      <BotonIniciarSesion
        activo={pantallaActual === 'inicio-sesion' || pantallaActual === 'registro'}
        sesionVencida={sesion.estado === 'vencida'}
        onClick={() => setPantallaActual('inicio-sesion')}
      />
    );
  }

  const nombre = sesion.nombreVisible ?? '';
  const inicial = nombre.trim().charAt(0).toUpperCase() || '?';
  const enPerfil = pantallaActual === 'perfil';

  const cerrar = async () => {
    if (cerrando) return;
    setCerrando(true);
    setErrorCierre(null);
    const error = await control.cerrarSesion();
    setCerrando(false);
    if (error) {
      setErrorCierre(error);
    } else if (enPerfil) {
      setPantallaActual('mis-cuentas');
    }
  };

  return (
    <div className={`rounded-xl border p-3 ${enPerfil ? 'border-[#9333EA] bg-[#9333EA]/20' : 'border-purple-700/70 bg-white/5'}`}>
      <button
        type="button"
        onClick={() => setPantallaActual('perfil')}
        aria-label={`Ver mi perfil, sesión de ${nombre}`}
        aria-current={enPerfil ? 'page' : undefined}
        className="flex w-full items-center gap-2.5 rounded-lg text-left cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
      >
        <span
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-[#9333EA] text-sm font-bold text-white"
          aria-hidden="true"
        >
          {inicial}
        </span>
        <span className="min-w-0">
          <span className="block truncate text-sm font-semibold text-white">{nombre}</span>
          <span className="block text-[11px] text-purple-300">Sesión activa</span>
        </span>
      </button>

      <button
        type="button"
        onClick={cerrar}
        disabled={cerrando}
        className="mt-2.5 w-full rounded-lg border border-purple-700/70 py-1.5 text-xs font-medium text-purple-100 hover:bg-white/10 disabled:opacity-60 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
      >
        {cerrando ? 'Cerrando…' : 'Cerrar sesión'}
      </button>

      {errorCierre && (
        <p role="alert" className="mt-2 text-[11px] font-medium text-rose-300">
          {errorCierre}
        </p>
      )}
    </div>
  );
};
