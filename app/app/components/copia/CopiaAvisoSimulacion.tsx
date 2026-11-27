'use client';

import React, { useId, useState } from 'react';
import {
  ESCENARIOS_SIMULADOS,
  elegirEscenarioSimulado,
  escenarioSimulado,
  type EscenarioSimulado,
} from '../../utils/simulacionIntercambio';
import { SIMULAR_INTERCAMBIO } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';

/**
 * Solo aparece con `NEXT_PUBLIC_SIMULAR_INTERCAMBIO=1` (desarrollo). Avisa de que nada es real
 * y deja elegir qué resultado devuelven los comandos simulados, para ver los cinco errores de
 * la sección 2.7. En una instalación normal no se dibuja.
 */
export function CopiaAvisoSimulacion({ modoOscuro }: { modoOscuro: boolean }) {
  const idSelector = useId();
  const tema = temaSocial(modoOscuro);
  const [valor, setValor] = useState<EscenarioSimulado>(() => escenarioSimulado());

  if (!SIMULAR_INTERCAMBIO) return null;

  return (
    <section
      aria-label="Modo simulación"
      className={`mb-5 rounded-2xl border p-4 text-xs ${tema.avisoAmbar}`}
    >
      <p className="font-semibold">Modo simulación</p>
      <p className="mt-0.5 opacity-90">
        No se escribe ni se lee ningún archivo real, no se toca tu libro y no hay red. Los
        resultados los decide este selector.
      </p>
      <div className="mt-3">
        <label htmlFor={idSelector} className="mb-1 block font-semibold">
          Resultado simulado
        </label>
        <select
          id={idSelector}
          value={valor}
          onChange={(e) => {
            const nuevo = e.target.value as EscenarioSimulado;
            elegirEscenarioSimulado(nuevo);
            setValor(nuevo);
          }}
          style={tema.estiloInput}
          className="w-full rounded-xl border px-3 py-2 text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
        >
          {ESCENARIOS_SIMULADOS.map((e) => (
            <option key={e.valor} value={e.valor}>
              {e.texto}
            </option>
          ))}
        </select>
      </div>
    </section>
  );
}
