'use client';

import React from 'react';
import { ETIQUETA_REGLA } from '../../utils/formatoSocial';
import { formatearFechaDDMMYYYY, formatearMontoTexto } from '../../utils/formato';
import type { GastoGrupoDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { TablaPartes } from './TablaPartes';
import { Tarjeta } from './Tarjeta';

interface Props {
  modoOscuro: boolean;
  gastos: GastoGrupoDto[];
  moneda: string;
  nombres: Record<string, string>;
}

/** Gastos del grupo (del más reciente al más antiguo) con sus partes. */
export function GastosGrupo({ modoOscuro, gastos, moneda, nombres }: Props) {
  const tema = temaSocial(modoOscuro);
  return (
    <Tarjeta
      modoOscuro={modoOscuro}
      titulo="Gastos"
      descripcion="Lo que se registró en el grupo y cómo se dividió."
    >
      {gastos.length === 0 ? (
        <EstadoVacioSocial
          modoOscuro={modoOscuro}
          titulo="Todavía no hay gastos"
          texto="Registra el primero con «Gasto compartido»."
        />
      ) : (
        <ul className="space-y-5">
          {gastos.map((g) => (
            <li key={g.id}>
              <div className="mb-2 flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="break-words text-sm font-semibold">{g.concepto}</p>
                  <p className={`text-[11px] ${tema.textoSuave}`}>
                    {formatearFechaDDMMYYYY(g.fecha)} · Pagó {nombres[g.pagador] ?? g.pagador} ·{' '}
                    {ETIQUETA_REGLA[g.regla]}
                  </p>
                </div>
                <p className="shrink-0 whitespace-nowrap text-sm font-bold">
                  {formatearMontoTexto(g.monto, moneda)}
                </p>
              </div>
              <TablaPartes
                modoOscuro={modoOscuro}
                partes={g.partes}
                moneda={moneda}
                nombres={nombres}
                descripcion={`Partes del gasto ${g.concepto}`}
              />
            </li>
          ))}
        </ul>
      )}
    </Tarjeta>
  );
}
