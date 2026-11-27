'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { formatearFechaDDMMYYYY, formatearMontoTexto } from '../../utils/formato';
import type { GastoEnColaDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { Tarjeta } from './Tarjeta';

interface Props {
  ctx: ContextoSocial;
  items: GastoEnColaDto[];
  /** Muestra el nombre del grupo en cada gasto (lista general de grupos). */
  mostrarGrupo?: boolean;
}

/**
 * Gastos de grupo guardados en este dispositivo que aún no se enviaron («pendiente de envío»)
 * y los que el servidor rechazó, con su motivo. Un rechazado no bloquea a los demás.
 */
export function GastosEnCola({ ctx, items, mostrarGrupo = false }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  if (items.length === 0) return null;

  const chipRechazado = ctx.modoOscuro ? 'bg-rose-950/60 text-rose-300' : 'bg-rose-100 text-rose-800';
  // Primero los que esperan envío; después los rechazados.
  const ordenados = [...items].sort(
    (a, b) => Number(a.estado === 'rechazado') - Number(b.estado === 'rechazado')
  );

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Gastos en cola"
      descripcion="Registrados en este dispositivo. Se envían en orden, una sola vez."
    >
      <ul className="space-y-3">
        {ordenados.map((g) => {
          const rechazado = g.estado === 'rechazado';
          return (
            <li key={g.clave} className={`rounded-xl border p-3 ${tema.interior}`}>
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="break-words text-sm font-semibold">{g.concepto}</p>
                  <p className={`text-[11px] ${tema.textoSuave}`}>
                    {mostrarGrupo ? `${g.grupo_nombre} · ` : ''}
                    {formatearFechaDDMMYYYY(g.fecha)}
                  </p>
                </div>
                <div className="flex shrink-0 flex-col items-end gap-1.5">
                  <p className="whitespace-nowrap text-sm font-bold">
                    {formatearMontoTexto(g.monto, g.moneda)}
                  </p>
                  <span
                    className={`rounded-full px-2.5 py-0.5 text-[10px] font-bold uppercase tracking-wide ${
                      rechazado ? chipRechazado : tema.chipAmbar
                    }`}
                  >
                    {rechazado ? 'Rechazado' : 'Pendiente de envío'}
                  </span>
                </div>
              </div>
              {rechazado ? (
                <p role="alert" className={`mt-2 rounded-lg border px-3 py-2 text-xs ${tema.avisoError}`}>
                  <span className="font-semibold">Motivo: </span>
                  {g.motivo ?? 'El servidor no aceptó este gasto.'}
                </p>
              ) : (
                <p className={`mt-2 text-[11px] ${tema.textoSuave}`}>
                  Se enviará cuando vuelva la conexión. Tus cuentas y movimientos no viajan.
                </p>
              )}
            </li>
          );
        })}
      </ul>
    </Tarjeta>
  );
}
