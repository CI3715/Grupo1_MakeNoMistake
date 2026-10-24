import React, { useState } from 'react';
import { TipoMovimiento } from '../types/cuentas';
import { IconoGasto, IconoIngreso, IconoX } from './Iconos';

interface Props {
  onCrear: (nombre: string, tipo: TipoMovimiento) => Promise<void>;
  onCerrar: () => void;
  tipoInicial?: TipoMovimiento;
}

export const FormularioCategoria: React.FC<Props> = ({ onCrear, onCerrar, tipoInicial = 'gasto' }) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoMovimiento>(tipoInicial);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!nombre.trim()) {
      setError('Escribe un nombre para la categoría.');
      return;
    }

    setEnviando(true);
    setError(null);
    try {
      await onCrear(nombre.trim(), tipo);
      onCerrar();
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'errores' in err) {
        const errObj = err as { errores: Array<{ mensaje: string }> };
        if (errObj.errores?.[0]?.mensaje) {
          setError(errObj.errores[0].mensaje);
          return;
        }
      }
      setError(typeof err === 'string' ? err : 'Error al registrar la categoría.');
    } finally {
      setEnviando(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="titulo-cat"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
    >
      <div className="w-full max-w-sm overflow-hidden rounded-3xl border border-borde bg-superficie p-6 shadow-2xl">
        <div className="flex items-center justify-between pb-3 border-b border-borde">
          <h3 id="titulo-cat" className="text-base font-bold text-texto">
            Nueva Categoría
          </h3>
          <button
            type="button"
            onClick={onCerrar}
            className="rounded-lg p-1.5 text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            <IconoX width={18} height={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Tipo
            </label>
            <div className="grid grid-cols-2 gap-2 rounded-xl bg-superficie-2 p-1">
              <button
                type="button"
                onClick={() => setTipo('gasto')}
                className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
                  tipo === 'gasto'
                    ? 'bg-peligro text-white shadow-sm'
                    : 'text-suave hover:text-texto'
                }`}
              >
                <IconoGasto width={14} height={14} />
                Gasto
              </button>
              <button
                type="button"
                onClick={() => setTipo('ingreso')}
                className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
                  tipo === 'ingreso'
                    ? 'bg-exito text-white shadow-sm'
                    : 'text-suave hover:text-texto'
                }`}
              >
                <IconoIngreso width={14} height={14} />
                Ingreso
              </button>
            </div>
          </div>

          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Nombre de la categoría
            </label>
            <input
              type="text"
              autoFocus
              placeholder="Ej. Cine, Salud, Restaurante"
              value={nombre}
              onChange={(e) => setNombre(e.target.value)}
              className="w-full rounded-xl border border-borde bg-superficie px-3.5 py-2.5 text-sm outline-none transition placeholder:text-suave/60 focus:border-acento focus:ring-4 focus:ring-acento/15 text-texto"
            />
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
              {enviando ? 'Guardando…' : 'Crear Categoría'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};