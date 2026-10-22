'use client';

import React from 'react';
import { ComposicionSaldoData } from '../types/cuentas';

interface Props {
  composicion: ComposicionSaldoData;
  moneda: string;
  modoOscuro?: boolean;
}

export const ComposicionSaldo: React.FC<Props> = ({ composicion, moneda, modoOscuro }) => {
  const cardClass = modoOscuro ? 'bg-[#151c28] border-gray-800 text-gray-100' : 'bg-white border-gray-200 text-gray-800';

  return (
    <div className={`p-5 rounded-2xl shadow-sm border ${cardClass} mb-6`}>
      <h3 className="text-[11px] font-bold text-gray-400 dark:text-gray-400 mb-3 uppercase tracking-wider">
        Composición del Saldo
      </h3>
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-center">
        <div className="p-3 rounded-xl bg-gray-100/80 dark:bg-gray-800/60">
          <span className="block text-xs text-gray-500 dark:text-gray-400">Saldo Inicial</span>
          <span className="font-bold text-sm mt-1 block">{moneda} {composicion.saldo_inicial}</span>
        </div>
        <div className="p-3 rounded-xl bg-emerald-50 dark:bg-emerald-950/30 text-emerald-600 dark:text-emerald-400 border border-emerald-100 dark:border-emerald-900/30">
          <span className="block text-xs font-medium">Ingresos</span>
          <span className="font-bold text-sm mt-1 block">+ {moneda} {composicion.total_ingresos}</span>
        </div>
        <div className="p-3 rounded-xl bg-rose-50 dark:bg-rose-950/30 text-rose-600 dark:text-rose-400 border border-rose-100 dark:border-rose-900/30">
          <span className="block text-xs font-medium">Gastos</span>
          <span className="font-bold text-sm mt-1 block">− {moneda} {composicion.total_gastos}</span>
        </div>
        <div className="p-3 rounded-xl bg-blue-50 dark:bg-blue-950/30 text-blue-600 dark:text-blue-400 border border-blue-100 dark:border-blue-900/30">
          <span className="block text-xs font-medium">Ajustes</span>
          <span className="font-bold text-sm mt-1 block">{moneda} {composicion.total_ajustes}</span>
        </div>
        <div className="p-3 rounded-xl bg-purple-50 dark:bg-purple-950/30 text-[#9333EA] dark:text-purple-300 border border-purple-100 dark:border-purple-900/30 font-bold">
          <span className="block text-xs font-medium">Saldo Actual</span>
          <span className="text-sm font-extrabold mt-1 block">{moneda} {composicion.saldo_actual}</span>
        </div>
      </div>
    </div>
  );
};