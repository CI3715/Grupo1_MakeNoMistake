'use client';

import React from 'react';
import { Movimiento } from '../types/cuentas';

interface Props {
  movimientos: Movimiento[];
  onCorregir: (movimiento: Movimiento) => void;
  modoOscuro?: boolean;
}

export const ListaMovimientos: React.FC<Props> = ({ movimientos, onCorregir, modoOscuro }) => {
  return (
    <div className={`overflow-x-auto rounded-2xl border ${modoOscuro ? 'border-gray-800 bg-[#151c28]' : 'border-gray-200 bg-white'} shadow-sm`}>
      <table className="w-full text-left text-xs">
        <thead className={modoOscuro ? 'bg-gray-900/50 text-gray-400' : 'bg-gray-50/50 text-gray-500'}>
          <tr className="border-b border-gray-200 dark:border-gray-800">
            <th className="p-3.5 font-semibold">Fecha</th>
            <th className="p-3.5 font-semibold">Categoría</th>
            <th className="p-3.5 font-semibold">Nota</th>
            <th className="p-3.5 text-right font-semibold">Monto</th>
            <th className="p-3.5 text-center font-semibold">Estado / Relación</th>
            <th className="p-3.5 text-center font-semibold">Acción</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-200 dark:divide-gray-800">
          {movimientos.map((m) => {
            const esGasto = m.tipo === 'gasto';
            const colorMonto = esGasto ? 'text-rose-500 dark:text-rose-400' : 'text-emerald-500 dark:text-emerald-400';

            return (
              <tr key={m.id} className="hover:bg-gray-50/50 dark:hover:bg-slate-800/30 transition">
                <td className="p-3.5 whitespace-nowrap">{m.fecha}</td>
                <td className="p-3.5 font-medium">{m.categoria_nombre}</td>
                <td className="p-3.5 text-gray-500 dark:text-gray-400">{m.nota || '—'}</td>
                <td className={`p-3.5 text-right font-bold ${colorMonto}`}>
                  {m.monto_formateado}
                </td>
                <td className="p-3.5 text-center">
                  {m.es_corregido && (
                    <span className="px-2 py-0.5 text-[10px] rounded-md bg-amber-200 text-amber-900 dark:bg-amber-900/60 dark:text-amber-200 font-bold">
                      Corregido
                    </span>
                  )}
                  {m.es_ajuste && (
                    <span className="px-2 py-0.5 text-[10px] rounded-md bg-blue-200 text-blue-900 dark:bg-blue-900/60 dark:text-blue-200 font-bold">
                      Ajuste {m.movimiento_referenciado_id ? `(Ref: ${m.movimiento_referenciado_id})` : ''}
                    </span>
                  )}
                  {!m.es_corregido && !m.es_ajuste && (
                    <span className="text-xs text-gray-400">Normal</span>
                  )}
                </td>
                <td className="p-3.5 text-center">
                  {!m.es_ajuste && (
                    <button
                      onClick={() => onCorregir(m)}
                      className="px-3 py-1 text-xs bg-[#9333EA] hover:bg-purple-600 text-white font-medium rounded-lg transition"
                    >
                      Corregir
                    </button>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};