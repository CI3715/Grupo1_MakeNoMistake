import React, { useState } from 'react';
import { Categoria, TipoMovimiento } from '../types/cuentas';
import { normalizarSaldo } from '../utils/formato';
import { IconoGasto, IconoIngreso, IconoMas, IconoX } from './Iconos';

interface Props {
  cuentaNombre: string;
  simbolo: string;
  tipoInicial: TipoMovimiento;
  categorias: Categoria[];
  onRegistrar: (datos: {
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => Promise<void>;
  onAbrirNuevaCategoria: (tipo: TipoMovimiento) => void;
  onCerrar: () => void;
}

export const FormularioMovimiento: React.FC<Props> = ({
  cuentaNombre,
  simbolo,
  tipoInicial,
  categorias,
  onRegistrar,
  onAbrirNuevaCategoria,
  onCerrar,
}) => {
  const [tipo, setTipo] = useState<TipoMovimiento>(tipoInicial);
  const [monto, setMonto] = useState('');
  const [fecha, setFecha] = useState(() => new Date().toISOString().split('T')[0]);
  const [categoriaId, setCategoriaId] = useState('');
  const [nota, setNota] = useState('');
  const [errores, setErrores] = useState<Record<string, string>>({});
  const [enviando, setEnviando] = useState(false);

  const categoriasFiltradas = categorias.filter(
    (c) => c.tipo.toLowerCase() === tipo.toLowerCase() && !c.de_fabrica
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrores({});

    const normalizado = normalizarSaldo(monto);
    if (!normalizado || normalizado === '0' || Number(normalizado) <= 0) {
      setErrores({ monto: 'Ingresa un monto válido mayor a 0.' });
      return;
    }

    if (!categoriaId) {
      setErrores({ categoria_id: 'Selecciona una categoría.' });
      return;
    }

    setEnviando(true);
    try {
      await onRegistrar({
        tipo,
        monto: normalizado,
        fecha,
        categoria_id: categoriaId,
        nota: nota.trim(),
      });
      onCerrar();
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'errores' in err) {
        const mapa: Record<string, string> = {};
        const errObj = err as { errores: Array<{ campo?: string; mensaje: string }> };
        errObj.errores.forEach((item) => {
          mapa[item.campo || 'monto'] = item.mensaje;
        });
        setErrores(mapa);
      } else {
        setErrores({ general: typeof err === 'string' ? err : 'Error al registrar el movimiento.' });
      }
    } finally {
      setEnviando(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="titulo-mov"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
    >
      <div className="w-full max-w-md overflow-hidden rounded-3xl border border-borde bg-superficie p-6 shadow-2xl">
        <div className="flex items-center justify-between pb-3 border-b border-borde">
          <div>
            <h3 id="titulo-mov" className="text-lg font-bold text-texto">
              Registrar Movimiento
            </h3>
            <p className="text-xs text-suave mt-0.5">Cuenta: <span className="font-semibold text-texto">{cuentaNombre}</span></p>
          </div>
          <button
            type="button"
            onClick={onCerrar}
            className="rounded-lg p-1.5 text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            <IconoX width={18} height={18} />
          </button>
        </div>

        {errores.general && (
          <div className="mt-4 rounded-xl border border-peligro-borde bg-peligro-suave p-3 text-xs text-peligro font-medium">
            {errores.general}
          </div>
        )}

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          {/* Tipo de movimiento */}
          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Tipo de movimiento
            </label>
            <div className="grid grid-cols-2 gap-2 rounded-xl bg-superficie-2 p-1">
              <button
                type="button"
                onClick={() => {
                  setTipo('gasto');
                  setCategoriaId('');
                }}
                className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-2.5 text-xs font-semibold transition ${
                  tipo === 'gasto'
                    ? 'bg-peligro text-white shadow-sm'
                    : 'text-suave hover:text-texto'
                }`}
              >
                <IconoGasto width={14} height={14} />
                Gasto (−)
              </button>
              <button
                type="button"
                onClick={() => {
                  setTipo('ingreso');
                  setCategoriaId('');
                }}
                className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-2.5 text-xs font-semibold transition ${
                  tipo === 'ingreso'
                    ? 'bg-exito text-white shadow-sm'
                    : 'text-suave hover:text-texto'
                }`}
              >
                <IconoIngreso width={14} height={14} />
                Ingreso (+)
              </button>
            </div>
          </div>

          {/* Monto */}
          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Monto
            </label>
            <div className="flex items-center gap-2 rounded-xl border border-borde bg-superficie px-3.5 py-2.5 transition focus-within:border-acento focus-within:ring-4 focus-within:ring-acento/15">
              <span className="font-semibold text-suave">{simbolo}</span>
              <input
                type="text"
                autoFocus
                inputMode="decimal"
                placeholder="0,00"
                value={monto}
                onChange={(e) => setMonto(e.target.value)}
                className="w-full bg-transparent text-sm font-semibold tabular-nums outline-none text-texto sm:text-base"
              />
            </div>
            {errores.monto && <p className="mt-1 text-xs font-medium text-peligro">{errores.monto}</p>}
          </div>

          {/* Categoría */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-suave">
                Categoría
              </label>
              <button
                type="button"
                onClick={() => onAbrirNuevaCategoria(tipo)}
                className="inline-flex items-center gap-1 text-xs font-semibold text-acento-texto hover:underline"
              >
                <IconoMas width={12} height={12} />
                Nueva categoría
              </button>
            </div>
            <select
              value={categoriaId}
              onChange={(e) => setCategoriaId(e.target.value)}
              className="w-full rounded-xl border border-borde bg-superficie px-3.5 py-2.5 text-sm outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15 text-texto cursor-pointer"
            >
              <option value="">Selecciona una categoría…</option>
              {categoriasFiltradas.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.nombre}
                </option>
              ))}
            </select>
            {errores.categoria_id && (
              <p className="mt-1 text-xs font-medium text-peligro">{errores.categoria_id}</p>
            )}
            {categoriasFiltradas.length === 0 && (
              <p className="mt-1.5 text-xs text-suave">
                No tienes categorías de {tipo} creadas. Haz clic en "Nueva categoría".
              </p>
            )}
          </div>

          {/* Fecha */}
          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Fecha
            </label>
            <input
              type="date"
              value={fecha}
              onChange={(e) => setFecha(e.target.value)}
              className="w-full rounded-xl border border-borde bg-superficie px-3.5 py-2 text-sm outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15 text-texto"
            />
            {errores.fecha && <p className="mt-1 text-xs font-medium text-peligro">{errores.fecha}</p>}
          </div>

          {/* Nota */}
          <div>
            <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
              Nota (opcional)
            </label>
            <input
              type="text"
              placeholder="Ej. Mercado de la semana, Pago de nómina"
              value={nota}
              onChange={(e) => setNota(e.target.value)}
              className="w-full rounded-xl border border-borde bg-superficie px-3.5 py-2.5 text-sm outline-none transition placeholder:text-suave/60 focus:border-acento focus:ring-4 focus:ring-acento/15 text-texto"
            />
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
              className={`rounded-xl px-5 py-2 text-xs font-semibold text-white shadow-sm transition hover:brightness-110 active:scale-[0.98] disabled:opacity-60 ${
                tipo === 'gasto' ? 'bg-peligro' : 'bg-exito'
              }`}
            >
              {enviando ? 'Guardando…' : tipo === 'gasto' ? 'Registrar Gasto' : 'Registrar Ingreso'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};