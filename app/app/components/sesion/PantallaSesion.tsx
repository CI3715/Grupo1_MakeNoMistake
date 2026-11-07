'use client';

import React from 'react';
import { VistaSesion } from '../../types/sesion';
import { PantallaInicioSesion } from './PantallaInicioSesion';
import { PantallaRegistro } from './PantallaRegistro';

interface Props {
  modoOscuro: boolean;
  vista: VistaSesion;
  setPantallaActual: (pantalla: string) => void;
}

/**
 * Punto único de entrada de las pantallas de sesión para page.tsx.
 * Cuando se conecten los comandos (invoke), el estado de carga y los errores
 * por campo se manejan AQUÍ y se pasan hacia abajo; page.tsx no crece.
 */
export function PantallaSesion({ modoOscuro, vista, setPantallaActual }: Props) {
  if (vista === 'registro') {
    return (
      <PantallaRegistro
        modoOscuro={modoOscuro}
        onIrAInicioSesion={() => setPantallaActual('inicio-sesion')}
      />
    );
  }
  return (
    <PantallaInicioSesion
      modoOscuro={modoOscuro}
      onIrARegistro={() => setPantallaActual('registro')}
    />
  );
}
