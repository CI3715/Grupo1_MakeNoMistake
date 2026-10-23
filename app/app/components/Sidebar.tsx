'use client';

import React from 'react';

interface SidebarProps {
  modoOscuro: boolean;
  toggleModoOscuro: () => void;
  pantallaActual: string;
  setPantallaActual: (pantalla: string) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  modoOscuro,
  toggleModoOscuro,
  pantallaActual,
  setPantallaActual,
}) => {
  return (
    <aside
      className={`w-[224px] min-h-[1024px] p-[28px_18px_24px] flex flex-col gap-[28px] shrink-0 border-r transition-colors duration-200 ${
        modoOscuro ? 'bg-[#180a29] text-white border-purple-900/40' : 'bg-[#211337] text-white border-purple-900/20'
      }`}
    >
      <div className="flex items-center gap-2 border-b border-purple-800/60 pb-4">
        <span className="text-xl font-bold tracking-wide text-white">
          Cuentas Claras
        </span>
      </div>

      <nav className="flex flex-col gap-2 flex-grow">
        <button
          onClick={() => setPantallaActual('mis-cuentas')}
          className={`text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition ${
            pantallaActual === 'mis-cuentas'
              ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
              : 'hover:bg-white/10 text-purple-200'
          }`}
        >
          Mis cuentas
        </button>
        <button
          onClick={() => setPantallaActual('detalle-cuenta')}
          className={`text-left px-3.5 py-2.5 rounded-xl font-medium text-sm transition ${
            pantallaActual === 'detalle-cuenta'
              ? 'bg-[#9333EA] text-white shadow-md shadow-purple-900/50'
              : 'hover:bg-white/10 text-purple-200'
          }`}
        >
          Detalle de Cuenta
        </button>
      </nav>

      <div className="pt-4 border-t border-purple-800/60 flex items-center justify-between text-xs">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded-full bg-black text-white flex items-center justify-center font-bold text-[10px]">
            N
          </div>
          <span className="text-purple-300">{modoOscuro ? 'Oscuro' : 'Claro'}</span>
        </div>
        <button
          onClick={toggleModoOscuro}
          className="px-3 py-1.5 bg-[#9333EA] hover:bg-purple-600 text-white rounded-full font-medium text-[11px] shadow transition"
        >
          {modoOscuro ? '🌙 Oscuro' : '☀️ Claro'}
        </button>
      </div>
    </aside>
  );
};