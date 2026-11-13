'use client';

import { useEffect, useRef, useState } from 'react';
import type { ErroresCampos } from '../types/social';
import { erroresDeFormulario } from './erroresSesion';
import { normalizarDecimal } from './formato';
import {
  previsualizarDivisionCommand,
  type ParticipantePayload,
  type PrevisualizacionDivisionDto,
  type ReglaDivision,
} from './tauriCommands';

export type CampoPrevia = 'monto' | 'regla' | 'participantes' | 'porcentajes' | 'montos';
const CAMPOS_PREVIA: readonly CampoPrevia[] = ['monto', 'regla', 'participantes', 'porcentajes', 'montos'];

/**
 * - inactiva: falta el monto o los participantes, no hay nada que calcular;
 * - lista: el núcleo devolvió el reparto (`cuadra`) o la diferencia exacta (`!cuadra`);
 * - error: algún dato es inválido (monto mal escrito, valor vacío…), con su campo.
 */
export type VistaPrevia =
  | { estado: 'inactiva' }
  | { estado: 'lista'; datos: PrevisualizacionDivisionDto }
  | { estado: 'error'; errores: ErroresCampos<CampoPrevia> };

const INACTIVA: VistaPrevia = { estado: 'inactiva' };
const ESPERA_MS = 250;

/**
 * Vista previa EN VIVO de la división: pide a `previsualizar_division` (cálculo local con
 * decimales exactos, sin red) cada vez que cambia algo, esperando 250 ms a que la persona
 * termine de escribir. La interfaz no calcula: solo muestra lo que devuelve el núcleo.
 * Si llegan dos respuestas desordenadas, vale solo la de la última petición.
 */
export function useVistaPreviaDivision(
  monto: string,
  regla: ReglaDivision,
  participantes: ParticipantePayload[]
): VistaPrevia {
  const [vista, setVista] = useState<VistaPrevia>(INACTIVA);
  const turno = useRef(0);
  const activa = monto.trim() !== '' && participantes.length > 0;

  useEffect(() => {
    if (!activa) return;
    const mio = ++turno.current;
    const espera = window.setTimeout(() => {
      previsualizarDivisionCommand({ monto: normalizarDecimal(monto), regla, participantes })
        .then((datos) => {
          if (turno.current === mio) setVista({ estado: 'lista', datos });
        })
        .catch((err: unknown) => {
          if (turno.current === mio) {
            setVista({ estado: 'error', errores: erroresDeFormulario(err, CAMPOS_PREVIA) });
          }
        });
    }, ESPERA_MS);
    return () => window.clearTimeout(espera);
  }, [activa, monto, regla, participantes]);

  return activa ? vista : INACTIVA;
}
