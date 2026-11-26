'use client';

import React from 'react';
import { EncabezadoPantalla } from '../social/EncabezadoPantalla';
import { CopiaAvisoSimulacion } from './CopiaAvisoSimulacion';
import { CopiaExportar } from './CopiaExportar';
import { CopiaImportar } from './CopiaImportar';

interface Props {
  modoOscuro: boolean;
  /** Avisa a las demás pantallas de que el libro pudo cambiar (tras importar). */
  alCambiarLibro: () => void;
}

/**
 * Copia de seguridad (Entrega 8, RF-INT-01 y RF-INT-02). Todo es entre este dispositivo y un
 * archivo que elige la persona: nada del libro viaja al servidor (RNF-01, INV-16).
 */
export function PantallaCopiaSeguridad({ modoOscuro, alCambiarLibro }: Props) {
  return (
    <div>
      <EncabezadoPantalla
        modoOscuro={modoOscuro}
        titulo="Copia de seguridad"
        subtitulo="Guarda todo tu libro en un archivo y restáuralo cuando quieras, incluso en otro dispositivo."
      />
      <CopiaAvisoSimulacion modoOscuro={modoOscuro} />
      <div className="space-y-5">
        <CopiaExportar modoOscuro={modoOscuro} />
        <CopiaImportar modoOscuro={modoOscuro} alCambiarLibro={alCambiarLibro} />
      </div>
    </div>
  );
}
