'use client';

import React from 'react';
import { TEXTO_REDONDEO } from '../../utils/formatoSocial';
import { formatearMontoTexto } from '../../utils/formato';
import type { ParteDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  partes: ParteDto[];
  moneda: string;
  /** Nombre de usuario → nombre visible. */
  nombres: Record<string, string>;
  /** Descripción para lectores de pantalla. */
  descripcion: string;
}

/** Cuánto le toca a cada persona; marca la parte que recibió céntimos del residuo. */
export function TablaPartes({ modoOscuro, partes, moneda, nombres, descripcion }: Props) {
  const tema = temaSocial(modoOscuro);
  return (
    <div className={`overflow-x-auto rounded-xl border ${tema.interior}`}>
      <table className="w-full text-xs">
        <caption className="sr-only">{descripcion}</caption>
        <thead className={tema.cabeceraTabla}>
          <tr>
            <th scope="col" className="px-3 py-2 text-left font-semibold">
              Persona
            </th>
            <th scope="col" className="px-3 py-2 text-right font-semibold">
              Parte
            </th>
          </tr>
        </thead>
        <tbody>
          {partes.map((p) => (
            <tr key={p.usuario} className={`border-t ${tema.linea}`}>
              <td className="px-3 py-2">
                <span className="block truncate font-medium">{nombres[p.usuario] ?? p.usuario}</span>
                {p.incluye_redondeo && (
                  <span className={`block text-[11px] ${tema.textoSuave}`}>{TEXTO_REDONDEO}</span>
                )}
              </td>
              <td className="whitespace-nowrap px-3 py-2 text-right font-semibold">
                {formatearMontoTexto(p.monto, moneda)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
