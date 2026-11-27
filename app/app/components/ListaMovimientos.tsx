'use client';

import React from 'react';
import { Movimiento } from '../types/cuentas';
import { IconoCorregir, IconoGasto, IconoIngreso } from './Iconos';

interface Props {
  movimientos: Movimiento[];
  onCorregir: (movimiento: Movimiento) => void;
  modoOscuro?: boolean;
}

export const ListaMovimientos: React.FC<Props> = ({ movimientos, onCorregir }) => {
  if (movimientos.length === 0) {
    return (
      <div className="rounded-2xl border border-dashed border-borde bg-superficie-2/40 p-10 text-center">
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
    <div className="overflow-hidden rounded-2xl border border-borde bg-superficie">
      {/* Vista móvil: tarjetas en pantallas pequeñas (< 640px) */}
      <div className="divide-y divide-borde sm:hidden">
        {movimientos.map((m) => {
          const textoMonto = m.monto_formateado ?? m.monto;
          const esNegativo = textoMonto.trim().startsWith('-');
          const estaCorregido = Boolean(m.es_corregido || m.es_ajuste);
          const ayuda = m.es_ajuste
            ? `Ajuste que corrige el movimiento ${m.movimiento_referenciado_id ?? ''}`.trim()
            : 'Este movimiento tiene una corrección';

          return (
            <div key={m.id} className="space-y-2.5 p-4 transition-colors hover:bg-superficie-2/50">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-texto">
                  {esNegativo ? (
                    <span className="text-peligro">
                      <IconoGasto width={12} height={12} />
                    </span>
                  ) : (
                    <span className="text-exito">
                      <IconoIngreso width={12} height={12} />
                    </span>
                  )}
                  <span>{m.categoria_nombre}</span>
                </span>
                <span
                  className={`text-sm font-bold tabular-nums ${
                    esNegativo ? 'text-peligro' : 'text-exito'
                  }`}
                >
                  {textoMonto}
                </span>
              </div>

              {m.nota && <p className="line-clamp-2 text-xs text-suave">{m.nota}</p>}

              <div className="flex items-center justify-between pt-1 text-xs">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-medium tabular-nums text-suave">{m.fecha}</span>
                  {m.es_gasto_de_grupo ? (
                    <span
                      title="Parte de un gasto compartido en un grupo"
                      className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-acento-texto"
                    >
                      Gasto de grupo
                    </span>
                  ) : m.es_ajuste ? (
                    <span
                      title={ayuda}
                      className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-info-texto"
                    >
                      Ajuste
                    </span>
                  ) : estaCorregido ? (
                    <span
                      title={ayuda}
                      className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-bold text-aviso-texto"
                    >
                      Corregido
                    </span>
                  ) : (
                    <span className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-suave">
                      Normal
                    </span>
                  )}
                </div>

                {m.es_gasto_de_grupo ? (
                  <span className="text-[11px] text-suave">Se corrige en el grupo</span>
                ) : (
                  <button
                    type="button"
                    onClick={() => onCorregir(m)}
                    className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-borde px-2.5 py-1 text-xs font-semibold text-acento-texto transition hover:border-acento hover:bg-acento-suave"
                  >
                    <IconoCorregir width={12} height={12} />
                    Corregir
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Vista de tabla: pantallas medianas y grandes (>= 640px) */}
      <div className="hidden overflow-x-auto sm:block">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-borde bg-superficie-2/60 text-xs text-suave uppercase tracking-wider">
              <th className="p-3.5 font-semibold">Fecha</th>
              <th className="p-3.5 font-semibold">Categoría</th>
              <th className="p-3.5 font-semibold">Nota</th>
              <th className="p-3.5 text-right font-semibold">Monto</th>
              <th className="p-3.5 text-center font-semibold">Estado / Relación</th>
              <th className="p-3.5 text-center font-semibold">Acción</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-borde">
            {movimientos.map((m) => {
              const textoMonto = m.monto_formateado ?? m.monto;
              const esNegativo = textoMonto.trim().startsWith('-');
              const estaCorregido = Boolean(m.es_corregido || m.es_ajuste);
              const ayuda = m.es_ajuste
                ? `Ajuste que corrige el movimiento ${m.movimiento_referenciado_id ?? ''}`.trim()
                : 'Este movimiento tiene una corrección';

              return (
                <tr key={m.id} className="transition-colors hover:bg-superficie-2/50">
                  <td className="whitespace-nowrap p-3.5 font-medium tabular-nums text-suave">
                    {m.fecha}
                  </td>
                  <td className="p-3.5 font-semibold text-texto">
                    <span className="inline-flex items-center gap-1.5 rounded-lg bg-superficie-2 px-2.5 py-1 text-xs">
                      {esNegativo ? (
                        <span className="text-peligro">
                          <IconoGasto width={12} height={12} />
                        </span>
                      ) : (
                        <span className="text-exito">
                          <IconoIngreso width={12} height={12} />
                        </span>
                      )}
                      <span>{m.categoria_nombre}</span>
                    </span>
                  </td>
                  <td className="max-w-[180px] truncate p-3.5 text-suave">{m.nota || '—'}</td>
                  <td
                    className={`whitespace-nowrap p-3.5 text-right text-sm font-bold tabular-nums ${
                      esNegativo ? 'text-peligro' : 'text-exito'
                    }`}
                  >
                    {textoMonto}
                  </td>
                  <td className="whitespace-nowrap p-3.5 text-center">
                    {m.es_gasto_de_grupo ? (
                      <span
                        title="Parte de un gasto compartido en un grupo"
                        className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-acento-texto"
                      >
                        Gasto de grupo
                      </span>
                    ) : m.es_ajuste ? (
                      <span
                        title={ayuda}
                        className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-info-texto"
                      >
                        Ajuste
                      </span>
                    ) : estaCorregido ? (
                      <span
                        title={ayuda}
                        className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-bold text-aviso-texto"
                      >
                        Corregido
                      </span>
                    ) : (
                      <span className="inline-flex items-center rounded-lg bg-superficie-2 px-2.5 py-1 text-xs font-semibold text-suave">
                        Normal
                      </span>
                    )}
                  </td>
                  <td className="whitespace-nowrap p-3.5 text-center">
                    {m.es_gasto_de_grupo ? (
                      <span className="text-[11px] text-suave">Se corrige en el grupo</span>
                    ) : (
                      <button
                        type="button"
                        onClick={() => onCorregir(m)}
                        className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-borde px-2.5 py-1 text-xs font-semibold text-acento-texto transition hover:border-acento hover:bg-acento-suave focus-visible:outline-2 focus-visible:outline-acento"
                      >
                        <IconoCorregir width={13} height={13} />
                        Corregir
                      </button>
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

