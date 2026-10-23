'use client';

import React from 'react';

interface Props {
  visible: boolean;
  montoResultante: string;
  onConfirmar: () => void;
  onCancelar: () => void;
}

export const DialogoAdvertencia: React.FC<Props> = ({ visible, montoResultante, onConfirmar, onCancelar }) => {
  if (!visible) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 p-6 rounded-2xl max-w-md w-full shadow-xl flex flex-col gap-4">
        <h3 className="text-lg font-bold text-amber-600 dark:text-amber-400">⚠️ Advertencia de Saldo Negativo</h3>
        <p className="text-sm text-gray-600 dark:text-gray-300">
          Este gasto dejará la cuenta con un saldo negativo de <strong className="font-mono">{montoResultante}</strong>.
          ¿Desea continuar de todos modos?
        </p>
        <div className="flex justify-end gap-3">
          <button onClick={onCancelar} className="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-700 rounded-xl hover:bg-gray-300">
            Cancelar
          </button>
          <button onClick={onConfirmar} className="px-4 py-2 text-sm bg-amber-600 hover:bg-amber-700 text-white rounded-xl font-semibold">
            Confirmar Gasto
          </button>
        </div>
      </div>
    </div>
  );
};