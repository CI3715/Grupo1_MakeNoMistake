'use client';

import React, { useState } from 'react';
import { Movimiento } from '../types/cuentas';

interface Props {
  movimiento: Movimiento | null;
  onConfirmar: (montoCorrecto: string) => void;
  onCancelar: () => void;
}

export const DialogoCorregir: React.FC<Props> = ({ movimiento, onConfirmar, onCancelar }) => {
  const [montoCorrecto, setMontoCorrecto] = useState('');

  if (!movimiento) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 p-6 rounded-2xl max-w-md w-full shadow-xl flex flex-col gap-4">
        <h3 className="text-lg font-bold text-gray-900 dark:text-white">Corregir Movimiento</h3>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          Movimiento original: <strong className="text-gray-800 dark:text-gray-200">{movimiento.categoria_nombre}</strong> ({movimiento.monto_formateado}).
          El ajuste se calculará sobre la diferencia.
        </p>

        <div className="flex flex-col gap-1">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">Monto Correcto</label>
          <input
            type="text"
            placeholder="Ej: 50,00 (0 para anular efecto)"
            value={montoCorrecto}
            onChange={(e) => setMontoCorrecto(e.target.value)}
            className="p-2.5 text-sm border rounded-xl dark:bg-gray-900 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
          />
        </div>

        <div className="flex justify-end gap-3 pt-2">
          <button
            onClick={onCancelar}
            className="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded-xl hover:bg-gray-300 dark:hover:bg-gray-600 transition"
          >
            Cancelar
          </button>
          <button
            onClick={() => onConfirmar(montoCorrecto)}
            className="px-4 py-2 text-sm bg-[#9333EA] hover:bg-purple-600 text-white rounded-xl font-semibold transition"
          >
            Guardar Corrección
          </button>
        </div>
      </div>
    </div>
  );
};