'use client';

import React, { useMemo } from 'react';
import type { ContextoSocial, VistaSocial } from '../../types/social';
import type { ControlSincronizacion } from '../../utils/useSincronizacion';
import { PantallaAmigos } from './PantallaAmigos';
import { PantallaGrupos } from './PantallaGrupos';

interface Props {
  modoOscuro: boolean;
  vista: VistaSocial;
  setPantallaActual: (pantalla: string) => void;
  /** Sincronización con el servidor: la crea `page.tsx` una sola vez. */
  sincronizacion: ControlSincronizacion;
}

/**
 * Punto único de entrada de Amigos y Grupos para page.tsx, igual que PantallaSesion:
 * page.tsx solo monta este componente y no crece con cada pantalla nueva.
 */
export function PantallaSocial({ modoOscuro, vista, setPantallaActual, sincronizacion }: Props) {
  const ctx = useMemo<ContextoSocial>(
    () => ({
      modoOscuro,
      irAInicioSesion: () => setPantallaActual('inicio-sesion'),
      irACuentas: () => setPantallaActual('mis-cuentas'),
      sincronizacion,
    }),
    [modoOscuro, setPantallaActual, sincronizacion]
  );
  return vista === 'amigos' ? <PantallaAmigos ctx={ctx} /> : <PantallaGrupos ctx={ctx} />;
}
