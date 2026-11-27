'use client';

import React from 'react';
import { IconoAdvertencia } from './Iconos';

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
}) => {
  if (!visible) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="titulo-advertencia"
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
    >
      <div className="w-full max-w-md overflow-hidden rounded-3xl border border-aviso-borde bg-superficie p-6 text-texto shadow-2xl">
        <div className="flex items-start gap-4">
          <div className="grid size-12 shrink-0 place-items-center rounded-2xl bg-aviso-suave text-aviso">
            <IconoAdvertencia width={24} height={24} />
          </div>
          <div className="min-w-0 flex-1">
            <h3 id="titulo-advertencia" className="text-lg font-bold text-texto">
              Advertencia de Saldo Negativo
            </h3>
            <p className="mt-2 text-xs leading-relaxed text-suave">
              Este gasto dejará la cuenta con un saldo negativo de{' '}
              <strong className="font-mono font-bold tabular-nums text-peligro">
                {montoResultante}
              </strong>
              . ¿Desea continuar de todos modos?
            </p>
          </div>
        </div>

        <div className="mt-6 flex justify-end gap-3 border-t border-borde pt-4">
          <button
            type="button"
            onClick={onCancelar}
            className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            Cancelar
          </button>
          <button
            type="button"
            onClick={onConfirmar}
            className="cursor-pointer rounded-xl bg-aviso px-5 py-2 text-xs font-bold text-slate-950 shadow-sm transition hover:brightness-110 active:scale-[0.98]"
          >
            Confirmar Gasto
          </button>
        </div>
      </div>
    </div>
  );
};

