'use client';

import React from 'react';
import { useBienvenidaVista } from '../../utils/useBienvenidaVista';
import { PantallaBienvenida } from './PantallaBienvenida';

interface Props {
  modoOscuro: boolean;
  setPantallaActual: (pantalla: string) => void;
}

/** Muestra la bienvenida solo la primera vez; después no estorba. */
export function BienvenidaPrimerAcceso({ modoOscuro, setPantallaActual }: Props) {
  const [vista, marcarVista] = useBienvenidaVista();
  if (vista) return null;

  return (
    <PantallaBienvenida
      modoOscuro={modoOscuro}
      onContinuarSinRegistro={marcarVista}
      onCrearUsuario={() => {
        marcarVista();
        setPantallaActual('registro');
      }}
      onIniciarSesion={() => {
        marcarVista();
        setPantallaActual('inicio-sesion');
      }}
    />
  );
}
