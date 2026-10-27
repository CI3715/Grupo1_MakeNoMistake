'use client';

import React from 'react';

interface PantallaPatrimonioProps {
  modoOscuro: boolean;
}

export const PantallaPatrimonio: React.FC<PantallaPatrimonioProps> = ({ modoOscuro }) => {
  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800/80 text-white' : 'bg-white border-gray-200 text-slate-900';

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Patrimonio Consolidado</h2>
          <p className="text-xs text-gray-400 mt-1">Consulta tu patrimonio en moneda base y subtotales por moneda</p>
        </div>
      </div>

      <div className={`p-6 rounded-2xl border shadow-sm ${bgCard}`}>
        <p className="text-sm">Pantalla preparada para desplegar la consolidación en moneda base y los avisos de tasas faltantes (RF-INF-01).</p>
      </div>
    </div>
  );
};