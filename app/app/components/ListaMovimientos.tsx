import React from 'react';
import { Movimiento } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';
import { IconoCorregir, IconoGasto, IconoIngreso } from './Iconos';

interface Props {
  movimientos: Movimiento[];
  simbolo: string;
  onCorregir: (movimiento: Movimiento) => void;
}

export const ListaMovimientos: React.FC<Props> = ({ movimientos, simbolo, onCorregir }) => {
  if (movimientos.length === 0) {
    return (
      <div className="rounded-3xl border border-dashed border-borde bg-superficie p-10 text-center shadow-sm">
        <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-superficie-2 text-suave">
          <IconoIngreso width={22} height={22} />
        </div>
        <h4 className="mt-3 text-sm font-semibold text-texto">Sin movimientos registrados</h4>
        <p className="mt-1 text-xs text-suave">
          Esta cuenta todavía no tiene ingresos ni gastos. Registra el primero para ver el historial.
        </p>
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-3xl border border-borde bg-superficie shadow-suave">
      <div className="border-b border-borde px-6 py-4">
        <h4 className="text-base font-semibold tracking-tight text-texto">Historial de Movimientos</h4>
        <p className="text-xs text-suave mt-0.5">Operaciones registradas en el libro local de eventos</p>
      </div>

      {/* Vista móvil: tarjetas en pantallas pequeñas (< 640px) */}
      <div className="divide-y divide-borde sm:hidden">
        {movimientos.map((m) => {
          const esGasto = m.tipo.toLowerCase() === 'gasto';
          const montoFormateado = formatearMontoLocal(m.monto, simbolo);
          const signo = esGasto ? '−' : '+';

          return (
            <div key={m.id} className="p-4 space-y-2.5 transition-colors hover:bg-superficie-2/50">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-texto">
                  {esGasto ? (
                    <span className="text-peligro"><IconoGasto width={12} height={12} /></span>
                  ) : (
                    <span className="text-exito"><IconoIngreso width={12} height={12} /></span>
                  )}
                  <span>{m.categoria_nombre}</span>
                </span>
                <span
                  className={`font-bold tabular-nums text-sm ${
                    esGasto ? 'text-peligro' : 'text-exito'
                  }`}
                >
                  {signo} {montoFormateado}
                </span>
              </div>

              {m.nota && (
                <p className="text-xs text-suave line-clamp-2">
                  {m.nota}
                </p>
              )}

              <div className="flex items-center justify-between pt-1 text-xs">
                <div className="flex items-center gap-2">
                  <span className="text-[11px] tabular-nums font-medium text-suave">{m.fecha}</span>
                  {m.estado === 'corregido' ? (
                    <span className="inline-flex items-center rounded-full border border-aviso-borde bg-aviso-suave px-2 py-0.5 text-[10px] font-semibold text-aviso-texto">
                      Corregido
                    </span>
                  ) : m.estado === 'ajuste' ? (
                    <span className="inline-flex items-center rounded-full border border-info-borde bg-info-suave px-2 py-0.5 text-[10px] font-semibold text-info-texto">
                      Ajuste
                    </span>
                  ) : (
                    <span className="inline-flex items-center rounded-full border border-borde bg-superficie-2 px-2 py-0.5 text-[10px] font-medium text-suave">
                      Normal
                    </span>
                  )}
                </div>

                {m.estado !== 'ajuste' ? (
                  <button
                    type="button"
                    onClick={() => onCorregir(m)}
                    className="inline-flex items-center gap-1 rounded-lg border border-borde px-2.5 py-1 text-xs font-semibold text-acento-texto transition hover:border-acento hover:bg-acento-suave"
                  >
                    <IconoCorregir width={12} height={12} />
                    Corregir
                  </button>
                ) : (
                  <span className="text-[11px] text-suave/50">—</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Vista de tabla: pantallas medianas y grandes (>= 640px) */}
      <div className="hidden overflow-x-auto sm:block">
        <table className="w-full text-left text-xs">
          <thead>
            <tr className="border-b border-borde bg-superficie-2/50 text-suave">
              <th className="px-6 py-3 font-semibold">Fecha</th>
              <th className="px-6 py-3 font-semibold">Categoría</th>
              <th className="px-6 py-3 font-semibold">Nota</th>
              <th className="px-6 py-3 font-semibold text-right">Monto</th>
              <th className="px-6 py-3 font-semibold text-center">Estado</th>
              <th className="px-6 py-3 font-semibold text-right">Acción</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-borde">
            {movimientos.map((m) => {
              const esGasto = m.tipo.toLowerCase() === 'gasto';
              const montoFormateado = formatearMontoLocal(m.monto, simbolo);
              const signo = esGasto ? '−' : '+';

              return (
                <tr key={m.id} className="transition-colors hover:bg-superficie-2/60">
                  <td className="whitespace-nowrap px-6 py-4 tabular-nums text-suave font-medium">
                    {m.fecha}
                  </td>
                  <td className="px-6 py-4 font-semibold text-texto">
                    <span className="inline-flex items-center gap-1.5 rounded-lg bg-superficie-2 px-2.5 py-1 text-xs">
                      {esGasto ? (
                        <span className="text-peligro"><IconoGasto width={12} height={12} /></span>
                      ) : (
                        <span className="text-exito"><IconoIngreso width={12} height={12} /></span>
                      )}
                      <span>{m.categoria_nombre}</span>
                    </span>
                  </td>
                  <td className="max-w-[180px] truncate px-6 py-4 text-suave">
                    {m.nota || '—'}
                  </td>
                  <td
                    className={`whitespace-nowrap px-6 py-4 text-right font-bold tabular-nums text-sm ${
                      esGasto ? 'text-peligro' : 'text-exito'
                    }`}
                  >
                    {signo} {montoFormateado}
                  </td>
                  <td className="whitespace-nowrap px-6 py-4 text-center">
                    {m.estado === 'corregido' ? (
                      <span className="inline-flex items-center rounded-full border border-aviso-borde bg-aviso-suave px-2.5 py-0.5 text-[11px] font-semibold text-aviso-texto">
                        Corregido
                      </span>
                    ) : m.estado === 'ajuste' ? (
                      <span className="inline-flex items-center rounded-full border border-info-borde bg-info-suave px-2.5 py-0.5 text-[11px] font-semibold text-info-texto">
                        Ajuste
                      </span>
                    ) : (
                      <span className="inline-flex items-center rounded-full border border-borde bg-superficie-2 px-2.5 py-0.5 text-[11px] font-medium text-suave">
                        Normal
                      </span>
                    )}
                  </td>
                  <td className="whitespace-nowrap px-6 py-4 text-right">
                    {m.estado !== 'ajuste' ? (
                      <button
                        type="button"
                        onClick={() => onCorregir(m)}
                        className="inline-flex items-center gap-1.5 rounded-lg border border-borde px-2.5 py-1 text-xs font-semibold text-acento-texto transition hover:border-acento hover:bg-acento-suave focus-visible:outline-2 focus-visible:outline-acento"
                      >
                        <IconoCorregir width={13} height={13} />
                        Corregir
                      </button>
                    ) : (
                      <span className="text-[11px] text-suave/50">—</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};