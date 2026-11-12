'use client';

import React from 'react';
import { IconoAlerta } from './Iconos';

interface Props {
  visible: boolean;
  montoResultante: string;
  onConfirmar: () => void;
  onCancelar: () => void;
  modoOscuro?: boolean;
}

export const DialogoAdvertencia: React.FC<Props> = ({
  visible,
  montoResultante,
  onConfirmar,
  onCancelar,
  modoOscuro = true,
}) => {
  if (!visible) return null;

  const bgCard = modoOscuro
    ? 'bg-[#151C28] border-gray-800/80 text-white'
    : 'bg-white border-gray-200 text-slate-900';

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-[60]">
      <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
        <h3 className="text-lg font-bold text-amber-500 mb-2 flex items-center gap-2">
          <IconoAlerta className="w-5 h-5 text-amber-500 shrink-0" />
          <span>Advertencia de Saldo Negativo</span>
        </h3>
        <p className={`text-xs ${modoOscuro ? 'text-gray-300' : 'text-gray-600'}`}>
          Este gasto dejará la cuenta con un saldo negativo de{' '}
          <strong className="font-mono">{montoResultante}</strong>. ¿Desea continuar de todos modos?
        </p>
        <div className="flex gap-3 justify-end pt-4">
          <button
            type="button"
            onClick={onCancelar}
            className={`px-4 py-2 rounded-xl text-xs font-semibold border cursor-pointer ${
              modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'
            }`}
          >
            Cancelar
          </button>
          <button
            type="button"
            onClick={onConfirmar}
            className="px-4 py-2 text-xs bg-amber-600 hover:bg-amber-700 text-white rounded-xl font-semibold cursor-pointer"
          >
            Confirmar Gasto
          </button>
        </div>
      </div>
    </div>
  );
};
