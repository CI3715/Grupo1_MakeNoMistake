'use client';

import React from 'react';

interface CuentaProps {
  cuentas: Array<{ id: string; nombre: string; tipo: string; moneda: string; saldo: string }>;
  onSeleccionarCuenta: (id: string) => void;
}

export const MisCuentasView: React.FC<CuentaProps> = ({ cuentas, onSeleccionarCuenta }) => {
  return (
    <div className="max-w-5xl mx-auto">
      <div className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Mis cuentas</h2>
          <p className="text-xs text-gray-500 mt-1">Listado de cuentas y saldos actuales</p>
        </div>
        <button className="bg-[#9333EA] text-white px-5 py-2.5 rounded-xl text-sm font-semibold hover:bg-purple-600 shadow-md">
          + Nueva cuenta
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {cuentas.map((c, idx) => (
          <div
            key={c.id}
            onClick={() => onSeleccionarCuenta(c.id)}
            className={`bg-white dark:bg-[#151c28] p-6 rounded-2xl cursor-pointer transition-all relative ${
              idx === 0 ? 'border-2 border-purple-400 shadow-sm' : 'border border-gray-200 dark:border-gray-800 shadow-sm hover:border-purple-300'
            }`}
          >
            <div className="flex justify-between items-start">
              <div>
                <h3 className="text-lg font-bold">{c.nombre}</h3>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">Tipo: {c.tipo}</p>
              </div>
              <span className="bg-purple-100 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 text-[10px] font-bold px-2.5 py-1 rounded-md">
                {c.moneda}
              </span>
            </div>

            <div className="mt-8 flex justify-between items-baseline">
              <span className="text-xs text-gray-500 font-medium">Saldo actual</span>
              <span className="text-2xl font-extrabold text-[#9333EA]">
                {c.moneda === 'USD' ? '$' : 'Bs.'} {c.saldo}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};