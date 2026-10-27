'use client';

import React from 'react';

interface PantallaTasasProps {
  modoOscuro: boolean;
}

export function PantallaTasas({ modoOscuro }: PantallaTasasProps) {
  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800/80 text-white' : 'bg-white border-gray-200 text-slate-900';

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Tasas de Cambio</h2>
          <p className="text-xs text-gray-400 mt-1">Registra y consulta tasas de cambio por par de monedas, fecha y fuente</p>
        </div>
      </div>

      <div className={`p-6 rounded-2xl border shadow-sm ${bgCard}`}>
        <p className="text-sm">Pantalla preparada para recibir los comandos de registro, eliminación e historial de tasas (RF-CAM-03, RF-CAM-06).</p>
      </div>
    </div>
  );
}