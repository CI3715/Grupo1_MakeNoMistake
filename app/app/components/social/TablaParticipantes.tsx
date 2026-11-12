'use client';

import React from 'react';
import { TEXTO_REDONDEO } from '../../utils/formatoSocial';
import { formatearMontoTexto } from '../../utils/formato';
import type { MiembroDto, ParteDto, ReglaDivision } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  miembros: MiembroDto[];
  moneda: string;
  regla: ReglaDivision;
  seleccion: Record<string, boolean>;
  valores: Record<string, string>;
  /** Partes calculadas por el núcleo; vacío si todavía no hay reparto. */
  partes: ParteDto[];
  deshabilitado: boolean;
  onSeleccion: (usuario: string, incluido: boolean) => void;
  onValor: (usuario: string, valor: string) => void;
}

/**
 * Participantes del gasto: quién entra (excluir es desmarcar), su porcentaje o monto
 * según la regla, y la parte que le toca. Las partes llegan calculadas: aquí no se suma.
 */
export function TablaParticipantes({
  modoOscuro,
  miembros,
  moneda,
  regla,
  seleccion,
  valores,
  partes,
  deshabilitado,
  onSeleccion,
  onValor,
}: Props) {
  const tema = temaSocial(modoOscuro);
  const pideValor = regla !== 'partes_iguales';
  const etiquetaValor = regla === 'porcentajes' ? 'Porcentaje' : 'Monto';

  return (
    <div className={`overflow-x-auto rounded-xl border ${tema.interior}`}>
      <table className="w-full text-xs">
        <caption className="sr-only">Participantes del gasto y la parte de cada uno</caption>
        <thead className={tema.cabeceraTabla}>
          <tr>
            <th scope="col" className="w-9 px-3 py-2 text-left font-semibold">
              <span className="sr-only">Incluir</span>
            </th>
            <th scope="col" className="px-2 py-2 text-left font-semibold">
              Persona
            </th>
            {pideValor && (
              <th scope="col" className="px-2 py-2 text-left font-semibold">
                {etiquetaValor}
              </th>
            )}
            <th scope="col" className="px-3 py-2 text-right font-semibold">
              Parte
            </th>
          </tr>
        </thead>
        <tbody>
          {miembros.map((m) => {
            const incluido = seleccion[m.usuario] === true;
            const parte = partes.find((p) => p.usuario === m.usuario);
            return (
              <tr key={m.usuario} className={`border-t align-top ${tema.linea}`}>
                <td className="px-3 py-2">
                  <input
                    type="checkbox"
                    checked={incluido}
                    disabled={deshabilitado}
                    onChange={(e) => onSeleccion(m.usuario, e.target.checked)}
                    aria-label={`Incluir a ${m.nombre_visible} en el gasto`}
                    className="h-4 w-4 cursor-pointer accent-[#9333EA] focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
                  />
                </td>
                <td className="max-w-[6.5rem] truncate px-2 py-2 font-medium">{m.nombre_visible}</td>
                {pideValor && (
                  <td className="px-2 py-1.5">
                    {incluido ? (
                      <div className="flex items-center gap-1">
                        <input
                          type="text"
                          inputMode="decimal"
                          value={valores[m.usuario] ?? ''}
                          disabled={deshabilitado}
                          onChange={(e) => onValor(m.usuario, e.target.value)}
                          placeholder={regla === 'porcentajes' ? '50' : '40,00'}
                          aria-label={`${etiquetaValor} de ${m.nombre_visible}`}
                          style={{
                            backgroundColor: modoOscuro ? '#0F1420' : '#ffffff',
                            color: modoOscuro ? '#ffffff' : '#0f172a',
                            borderColor: modoOscuro ? '#374151' : '#e2e8f0',
                          }}
                          className="w-16 rounded-lg border px-2 py-1.5 text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
                        />
                        <span className={tema.textoSuave} aria-hidden="true">
                          {regla === 'porcentajes' ? '%' : moneda}
                        </span>
                      </div>
                    ) : (
                      <span className={tema.textoSuave}>—</span>
                    )}
                  </td>
                )}
                <td className="px-3 py-2 text-right">
                  {parte ? (
                    <>
                      <span className="whitespace-nowrap font-semibold">
                        {formatearMontoTexto(parte.monto, moneda)}
                      </span>
                      {parte.incluye_redondeo && (
                        <span className={`block text-[11px] ${tema.textoSuave}`}>{TEXTO_REDONDEO}</span>
                      )}
                    </>
                  ) : (
                    <span className={tema.textoSuave}>{incluido ? '—' : 'No participa'}</span>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
