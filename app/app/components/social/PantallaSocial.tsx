'use client';

import React, { useMemo } from 'react';
import type { ContextoSocial, VistaSocial } from '../../types/social';
import { PantallaAmigos } from './PantallaAmigos';
import { PantallaGrupos } from './PantallaGrupos';

interface Props {
  modoOscuro: boolean;
  vista: VistaSocial;
  setPantallaActual: (pantalla: string) => void;
}

/**
 * Punto único de entrada de Amigos y Grupos para page.tsx, igual que PantallaSesion:
 * page.tsx solo monta este componente y no crece con cada pantalla nueva.
 */
export function PantallaSocial({ modoOscuro, vista, setPantallaActual }: Props) {
  const ctx = useMemo<ContextoSocial>(
    () => ({ modoOscuro, irAInicioSesion: () => setPantallaActual('inicio-sesion') }),
    [modoOscuro, setPantallaActual]
  );
  return vista === 'amigos' ? <PantallaAmigos ctx={ctx} /> : <PantallaGrupos ctx={ctx} />;
}
