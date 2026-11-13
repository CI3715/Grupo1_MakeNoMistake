'use client';

import React from 'react';
import { formatearBalance, signoBalance } from '../../utils/formatoSocial';
import { formatearMontoTexto } from '../../utils/formato';
import type { BalanceDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { Tarjeta } from './Tarjeta';

interface Props {
  modoOscuro: boolean;
  balances: BalanceDto[];
  moneda: string;
  nombres: Record<string, string>;
}

/** Balance de cada miembro (+ le deben, − debe). Lo calcula el servidor; aquí solo se muestra. */
export function BalancesGrupo({ modoOscuro, balances, moneda, nombres }: Props) {
  const tema = temaSocial(modoOscuro);
  return (
    <Tarjeta
      modoOscuro={modoOscuro}
      titulo="Balances"
      descripcion={`Quién debe y a quién le deben, en ${moneda}.`}
    >
      {balances.length === 0 ? (
        <EstadoVacioSocial
          modoOscuro={modoOscuro}
          titulo="Sin balances todavía"
          texto="Los balances aparecen cuando se registra el primer gasto."
        />
      ) : (
        <div className={`overflow-x-auto rounded-xl border ${tema.interior}`}>
          <table className="w-full text-xs">
            <caption className="sr-only">Balance de cada miembro del grupo</caption>
            <thead className={tema.cabeceraTabla}>
              <tr>
                <th scope="col" className="px-3 py-2 text-left font-semibold">
                  Persona
                </th>
                <th scope="col" className="px-3 py-2 text-right font-semibold">
                  Pagó
                </th>
                <th scope="col" className="px-3 py-2 text-right font-semibold">
                  Le corresponde
                </th>
                <th scope="col" className="px-3 py-2 text-right font-semibold">
                  Balance
                </th>
              </tr>
            </thead>
            <tbody>
              {balances.map((b) => {
                const signo = signoBalance(b.balance);
                const color =
                  signo === 'positivo' ? tema.positivo : signo === 'negativo' ? tema.negativo : tema.neutro;
                return (
                  <tr key={b.usuario} className={`border-t ${tema.linea}`}>
                    <td className="max-w-[7rem] truncate px-3 py-2 font-medium">
                      {nombres[b.usuario] ?? b.usuario}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2 text-right">
                      {formatearMontoTexto(b.pago)}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2 text-right">
                      {formatearMontoTexto(b.le_corresponde)}
                    </td>
                    <td className={`whitespace-nowrap px-3 py-2 text-right font-bold ${color}`}>
                      {formatearBalance(b.balance, moneda)}
                      <span className="sr-only">
                        {signo === 'positivo' ? ' (le deben)' : signo === 'negativo' ? ' (debe)' : ''}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      <p className={`mt-3 text-[11px] ${tema.textoSuave}`}>
        Positivo: le deben. Negativo: debe. La suma de todos los balances del grupo es siempre 0,00.
      </p>
    </Tarjeta>
  );
}
