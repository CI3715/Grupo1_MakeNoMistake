'use client';

import React, { useState } from 'react';
import { Movimiento } from '../types/cuentas';

interface Props {
  movimiento: Movimiento | null;
  onConfirmar: (montoCorrecto: string) => void;
  onCancelar: () => void;
  error?: string | null;
  modoOscuro?: boolean;
}

export const DialogoCorregir: React.FC<Props> = ({
  movimiento,
  onConfirmar,
  onCancelar,
  error = null,
  modoOscuro = true,
}) => {
  const [montoCorrecto, setMontoCorrecto] = useState('');

  if (!movimiento) return null;

  // Mismos estilos que las demás ventanas (nuevo movimiento, nueva cuenta).
  const bgCard = modoOscuro
    ? 'bg-[#151C28] border-gray-800/80 text-white'
    : 'bg-white border-gray-200 text-slate-900';

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' as const }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#e2e8f0', colorScheme: 'light' as const };

  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
      <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
        <h3 className="text-lg font-bold mb-2">Corregir Movimiento</h3>
        <p className={`text-xs mb-4 ${textMuted}`}>
          Movimiento a corregir:{' '}
          <strong className={modoOscuro ? 'text-gray-100' : 'text-slate-800'}>
            {movimiento.categoria_nombre}
          </strong>{' '}
          ({movimiento.monto_formateado}). El ajuste se calculará sobre la diferencia.
        </p>

        <div>
          <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Monto Correcto</label>
          <input
            type="text"
            placeholder="Ej: 50,00 (0 para anular efecto)"
            value={montoCorrecto}
            onChange={(e) => setMontoCorrecto(e.target.value)}
            style={estiloInput}
            className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
          />
          {error && <p className="text-rose-400 text-[11px] font-medium mt-1">{error}</p>}
        </div>

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
            onClick={() => onConfirmar(montoCorrecto)}
            className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer"
          >
            Guardar Corrección
          </button>
        </div>
      </div>
    </div>
  );
};
