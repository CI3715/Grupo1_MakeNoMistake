'use client';

import React, { useState } from 'react';
import { Movimiento } from '../types/cuentas';
import { IconoCorregir, IconoX } from './Iconos';

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
}) => {
  const [montoCorrecto, setMontoCorrecto] = useState('');

  if (!movimiento) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="titulo-corregir"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
    >
      <div className="w-full max-w-md overflow-hidden rounded-3xl border border-borde bg-superficie p-6 text-texto shadow-2xl">
        <div className="flex items-center justify-between border-b border-borde pb-3">
          <div className="flex items-center gap-2.5 text-texto">
            <span className="grid size-8 place-items-center rounded-xl bg-info-suave text-info-texto">
              <IconoCorregir width={18} height={18} />
            </span>
            <h3 id="titulo-corregir" className="text-lg font-semibold tracking-tight">
              Corregir Movimiento
            </h3>
          </div>
          <button
            type="button"
            onClick={onCancelar}
            className="cursor-pointer rounded-lg p-1.5 text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            <IconoX width={18} height={18} />
          </button>
        </div>

        <p className="mt-3 text-xs leading-relaxed text-suave">
          Movimiento a corregir:{' '}
          <strong className="text-texto">{movimiento.categoria_nombre}</strong> (
          {movimiento.monto_formateado}). El ajuste se calculará sobre la diferencia sin borrar el
          historial.
        </p>

        <div className="mt-5">
          <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
            Monto Correcto
          </label>
          <input
            type="text"
            autoFocus
            inputMode="decimal"
            placeholder="Ej: 50,00 (0 para anular efecto)"
            value={montoCorrecto}
            onChange={(e) => setMontoCorrecto(e.target.value)}
            className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-sm font-medium tabular-nums text-texto outline-none transition placeholder:text-suave/60 focus:border-acento focus:ring-4 focus:ring-acento/15"
          />
          {error && <p className="mt-1.5 text-xs font-medium text-peligro">{error}</p>}
        </div>

        <div className="mt-5 flex justify-end gap-3 border-t border-borde pt-4">
          <button
            type="button"
            onClick={onCancelar}
            className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            Cancelar
          </button>
          <button
            type="button"
            onClick={() => onConfirmar(montoCorrecto)}
            className="cursor-pointer rounded-xl bg-acento px-5 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
          >
            Guardar Corrección
          </button>
        </div>
      </div>
    </div>
  );
};

