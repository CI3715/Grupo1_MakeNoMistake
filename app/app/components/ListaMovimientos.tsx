'use client';

import React from 'react';
import { Movimiento } from '../types/cuentas';

interface Props {
  movimientos: Movimiento[];
  onCorregir: (movimiento: Movimiento) => void;
  modoOscuro?: boolean;
}

export const ListaMovimientos: React.FC<Props> = ({ movimientos, onCorregir, modoOscuro = true }) => {
  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';
  const divisor = modoOscuro ? 'border-gray-800' : 'border-gray-200';

  return (
    <div
      className={`overflow-x-auto rounded-2xl border shadow-sm ${
        modoOscuro ? 'border-gray-800 bg-[#151c28]' : 'border-gray-200 bg-white'
      }`}
    >
      <table className="w-full text-left text-xs">
        <thead className={modoOscuro ? 'bg-gray-900/50 text-gray-400' : 'bg-gray-50/50 text-gray-500'}>
          <tr className={`border-b ${divisor}`}>
            <th className="p-3.5 font-semibold">Fecha</th>
            <th className="p-3.5 font-semibold">Categoría</th>
            <th className="p-3.5 font-semibold">Nota</th>
            <th className="p-3.5 text-right font-semibold">Monto</th>
            <th className="p-3.5 text-center font-semibold">Estado / Relación</th>
            <th className="p-3.5 text-center font-semibold">Acción</th>
          </tr>
        </thead>
        <tbody className={`divide-y ${modoOscuro ? 'divide-gray-800' : 'divide-gray-200'}`}>
          {movimientos.map((m) => {
            const esNegativo = m.monto_formateado.trim().startsWith('-');
            const colorMonto = esNegativo
              ? modoOscuro ? 'text-rose-400' : 'text-rose-500'
              : modoOscuro ? 'text-emerald-400' : 'text-emerald-500';

            // El original corregido y su ajuste muestran el mismo estado «Corregido».
            // La referencia al original solo se deja como ayuda (title), no como texto.
            const estaCorregido = m.es_corregido || m.es_ajuste;
            const ayuda = m.es_ajuste
              ? `Ajuste que corrige el movimiento ${m.movimiento_referenciado_id ?? ''}`.trim()
              : 'Este movimiento tiene una corrección';

            return (
              <tr
                key={m.id}
                className={modoOscuro ? 'hover:bg-slate-800/30 transition' : 'hover:bg-gray-50/50 transition'}
              >
                <td className="p-3.5 whitespace-nowrap">{m.fecha}</td>
                <td className="p-3.5 font-medium">{m.categoria_nombre}</td>
                <td className={`p-3.5 ${textMuted}`}>{m.nota || '—'}</td>
                <td className={`p-3.5 text-right font-bold ${colorMonto}`}>{m.monto_formateado}</td>
                <td className="p-3.5 text-center">
                  {estaCorregido ? (
                    <span
                      title={ayuda}
                      className={`px-2 py-0.5 text-[10px] rounded-md font-bold ${
                        modoOscuro ? 'bg-amber-900/60 text-amber-200' : 'bg-amber-200 text-amber-900'
                      }`}
                    >
                      Corregido
                    </span>
                  ) : (
                    <span className={`text-xs ${textMuted}`}>Normal</span>
                  )}
                </td>
                <td className="p-3.5 text-center">
                  {/* El botón se mantiene siempre: un movimiento (o su ajuste) se puede volver a corregir. */}
                  <button
                    type="button"
                    onClick={() => onCorregir(m)}
                    className="px-3 py-1 text-xs bg-[#9333EA] hover:bg-purple-600 text-white font-medium rounded-lg transition cursor-pointer"
                  >
                    Corregir
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};
