'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { plural } from '../../utils/formatoSocial';
import { formatearFechaDDMMYYYY, formatearMontoTexto } from '../../utils/formato';
import type { ParteGrupoPendienteDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { Tarjeta } from './Tarjeta';

interface Props {
  ctx: ContextoSocial;
  partes: ParteGrupoPendienteDto[];
  /** Muestra el nombre del grupo en cada fila (lista general de grupos). */
  mostrarGrupo?: boolean;
  /** Con la lista vacía, muestra un mensaje en vez de ocultarse. */
  mostrarVacio?: boolean;
  /** Si se da, cada fila ofrece ir al grupo para elegir su cuenta. */
  onElegirCuenta?: (grupoId: string) => void;
}

/** Partes de gasto que ya bajaron pero esperan que elijas la cuenta donde aplicarse. */
export function PartesPendientes({ ctx, partes, mostrarGrupo = false, mostrarVacio = false, onElegirCuenta }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  if (partes.length === 0 && !mostrarVacio) return null;

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Partes pendientes de aplicar"
      descripcion="Ya bajaron del servidor; todavía no están en ninguna cuenta."
      acciones={
        partes.length > 0 ? (
          <span className={`rounded-full px-2.5 py-1 text-[11px] font-bold ${tema.chipAmbar}`}>
            {plural(partes.length, 'parte', 'partes')}
          </span>
        ) : undefined
      }
    >
      {partes.length === 0 ? (
        <EstadoVacioSocial
          modoOscuro={ctx.modoOscuro}
          titulo="Nada por aplicar"
          texto="Tus partes de este grupo ya están en tu libro."
        />
      ) : (
        <div className="space-y-3">
          <div role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}>
            <p className="font-semibold">Tu libro aún no cambió</p>
            <p className="mt-0.5">
              Estas partes están guardadas, pero ninguna cuenta las recibió. Elige una cuenta para
              el grupo y se aplican solas, en orden.
            </p>
          </div>

          <div className={`overflow-x-auto rounded-xl border ${tema.interior}`}>
            <table className="w-full text-xs">
              <caption className="sr-only">Partes de gastos de grupo sin aplicar</caption>
              <thead className={tema.cabeceraTabla}>
                <tr>
                  <th scope="col" className="px-3 py-2 text-left font-semibold">
                    Gasto
                  </th>
                  <th scope="col" className="px-3 py-2 text-left font-semibold">
                    Fecha
                  </th>
                  <th scope="col" className="px-3 py-2 text-right font-semibold">
                    Tu parte
                  </th>
                </tr>
              </thead>
              <tbody>
                {partes.map((p) => (
                  <tr key={p.evento_id} className={`border-t ${tema.linea}`}>
                    <td className="px-3 py-2">
                      <span className="block break-words font-medium">{p.concepto}</span>
                      {mostrarGrupo && (
                        <span className={`block text-[11px] ${tema.textoSuave}`}>{p.grupo_nombre}</span>
                      )}
                      {onElegirCuenta && (
                        <button
                          type="button"
                          onClick={() => onElegirCuenta(p.grupo_id)}
                          className={`mt-0.5 cursor-pointer text-[11px] font-semibold underline underline-offset-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${tema.textoEnlace}`}
                        >
                          Elegir cuenta
                        </button>
                      )}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2">{formatearFechaDDMMYYYY(p.fecha)}</td>
                    <td className="whitespace-nowrap px-3 py-2 text-right font-semibold">
                      {formatearMontoTexto(p.monto, p.moneda)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </Tarjeta>
  );
}
