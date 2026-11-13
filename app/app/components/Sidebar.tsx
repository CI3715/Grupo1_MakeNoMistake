'use client';

import React from 'react';
import { LogoCuentasClaras } from './LogoCuentasClaras';
import { BloqueSesionSidebar } from './sesion/BloqueSesionSidebar';
import type { ControlSesion } from '../utils/useSesion';
import {
  IconoBilletera,
  IconoDetalleCuenta,
  IconoIntercambio,
  IconoPatrimonio,
  IconoAmigos,
  IconoGrupos,
  IconoSol,
  IconoLuna,
} from './Iconos';

export interface SidebarProps {
  modoOscuro: boolean;
  toggleModoOscuro: () => void;
  pantallaActual: string;
  setPantallaActual: (pantalla: string) => void;
  control: ControlSesion;
}

export const Sidebar: React.FC<SidebarProps> = ({
  modoOscuro,
  toggleModoOscuro,
  pantallaActual,
  setPantallaActual,
  control,
}) => {
  return (
    <aside
      className={`w-[224px] h-screen overflow-y-auto p-[28px_18px_24px] flex flex-col justify-between shrink-0 border-r transition-colors duration-200 ${
        modoOscuro ? 'bg-[#180a29] text-white border-purple-900/40' : 'bg-[#211337] text-white border-purple-900/20'
      }`}
    >
      <div>
        <div className="flex items-center gap-2 border-b border-purple-800/60 pb-4 mb-6">
          <LogoCuentasClaras tono="claro" />
        </div>

        <nav aria-label="Secciones" className="flex flex-col gap-2">
          <button
            onClick={() => setPantallaActual('mis-cuentas')}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer ${
              pantallaActual === 'mis-cuentas'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoBilletera className="w-4 h-4 shrink-0" />
            <span>Mis cuentas</span>
          </button>
          
          <button
            onClick={() => setPantallaActual('detalle-cuenta')}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer ${
              pantallaActual === 'detalle-cuenta'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoDetalleCuenta className="w-4 h-4 shrink-0" />
            <span>Detalle de Cuenta</span>
          </button>

          <button
            onClick={() => setPantallaActual('tasas')}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer ${
              pantallaActual === 'tasas'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoIntercambio className="w-4 h-4 shrink-0" />
            <span>Tasas de Cambio</span>
          </button>

          <button
            onClick={() => setPantallaActual('patrimonio')}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer ${
              pantallaActual === 'patrimonio'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoPatrimonio className="w-4 h-4 shrink-0" />
            <span>Patrimonio</span>
          </button>

          <button
            onClick={() => setPantallaActual('amigos')}
            aria-current={pantallaActual === 'amigos' ? 'page' : undefined}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
              pantallaActual === 'amigos'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoAmigos className="w-4 h-4 shrink-0" />
            <span>Amigos</span>
          </button>

          <button
            onClick={() => setPantallaActual('grupos')}
            aria-current={pantallaActual === 'grupos' ? 'page' : undefined}
            className={`flex items-center gap-2.5 text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
              pantallaActual === 'grupos'
                ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
                : 'hover:bg-white/10 text-purple-200'
            }`}
          >
            <IconoGrupos className="w-4 h-4 shrink-0" />
            <span>Grupos</span>
          </button>
        </nav>
      </div>

      <div className="pt-4 border-t border-purple-800/60 flex flex-col gap-3">
        <BloqueSesionSidebar
          control={control}
          pantallaActual={pantallaActual}
          setPantallaActual={setPantallaActual}
        />
        <div className="flex items-center justify-between text-xs">
          <div className="flex items-center gap-2">
            <span className="text-purple-300">{modoOscuro ? 'Oscuro' : 'Claro'}</span>
          </div>
          <button
            onClick={toggleModoOscuro}
            aria-label={modoOscuro ? 'Cambiar a modo claro' : 'Cambiar a modo oscuro'}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-[#9333EA] hover:bg-purple-600 text-white rounded-full font-medium text-[11px] shadow transition cursor-pointer"
          >
            {modoOscuro ? <IconoLuna className="w-3.5 h-3.5" /> : <IconoSol className="w-3.5 h-3.5" />}
            <span>{modoOscuro ? 'Oscuro' : 'Claro'}</span>
          </button>
        </div>
      </div>
    </aside>
  );
};