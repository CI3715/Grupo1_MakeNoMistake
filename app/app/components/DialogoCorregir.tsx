import React, { useState } from 'react';
import { Movimiento } from '../types/cuentas';
import { IconoCorregir, IconoX } from './Iconos';
import { normalizarSaldo } from '../utils/formato';

interface Props {
  movimiento: Movimiento;
  simbolo: string;
  onConfirmar: (movimientoId: string, montoCorrecto: string) => Promise<void>;
  onCerrar: () => void;
}

export const DialogoCorregir: React.FC<Props> = ({ movimiento, simbolo, onConfirmar, onCerrar }) => {
  const [monto, setMonto] = useState(movimiento.monto || '');
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    const normalizado = normalizarSaldo(monto);
    if (!normalizado) {
      setError('Escribe un monto válido.');
      return;
    }

    setEnviando(true);
    try {
      await onConfirmar(movimiento.id, normalizado);
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'errores' in err) {
        const errObj = err as { errores: Array<{ mensaje: string }> };
        if (errObj.errores?.[0]?.mensaje) {
          setError(errObj.errores[0].mensaje);
          return;
        }
      }
      setError(typeof err === 'string' ? err : 'Ocurrió un error al corregir el movimiento.');
    } finally {
      setEnviando(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="titulo-corregir"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
    >
      <div className="w-full max-w-md overflow-hidden rounded-3xl border border-borde bg-superficie p-6 shadow-2xl">
        <div className="flex items-center justify-between pb-3 border-b border-borde">
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
            onClick={onCerrar}
            className="rounded-lg p-1.5 text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            <IconoX width={18} height={18} />
          </button>
        </div>

        <p className="mt-3 text-xs leading-relaxed text-suave">
          Indica el monto real de esta operación en <strong className="text-texto">{movimiento.categoria_nombre}</strong>.
          El libro de eventos registrará un movimiento de ajuste por la diferencia sin borrar el historial.
        </p>

        <form onSubmit={handleSubmit} className="mt-5 space-y-4">
          <div>
            <label className="mb-1 block text-xs font-semibold text-suave uppercase tracking-wider">
              Monto registrado anteriormente
            </label>
            <input
              type="text"
              disabled
              value={`${simbolo} ${movimiento.monto}`}
              className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-sm font-semibold tabular-nums text-suave cursor-not-allowed"
            />
          </div>

          <div>
            <label className="mb-1 block text-xs font-semibold text-suave uppercase tracking-wider">
              Monto correcto real
            </label>
            <div className="flex items-center gap-2 rounded-xl border border-borde bg-superficie px-3.5 py-2.5 focus-within:border-acento focus-within:ring-4 focus-within:ring-acento/15">
              <span className="font-semibold text-suave">{simbolo}</span>
              <input
                type="text"
                autoFocus
                inputMode="decimal"
                placeholder="Ej. 50,00 o 0 (para anular)"
                value={monto}
                onChange={(e) => setMonto(e.target.value)}
                className="w-full bg-transparent text-sm font-medium tabular-nums outline-none text-texto"
              />
            </div>
            {error && <p className="mt-1.5 text-xs font-medium text-peligro">{error}</p>}
          </div>

          <div className="flex justify-end gap-3 pt-3 border-t border-borde">
            <button
              type="button"
              onClick={onCerrar}
              className="rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={enviando}
              className="rounded-xl bg-acento px-5 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98] disabled:opacity-60"
            >
              {enviando ? 'Guardando…' : 'Guardar Ajuste'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};