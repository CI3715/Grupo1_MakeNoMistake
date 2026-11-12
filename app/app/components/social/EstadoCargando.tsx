'use client';

import React from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  /** Lo que se está cargando, para el lector de pantalla («amigos», «el grupo»). */
  que: string;
}

/** Marcador de carga: bloques que pulsan y un aviso accesible. */
export function EstadoCargando({ modoOscuro, que }: Props) {
  const tema = temaSocial(modoOscuro);
  const bloque = `animate-pulse rounded-2xl border ${tema.tarjeta}`;
  return (
    <div role="status" aria-busy="true" className="space-y-4">
      <span className="sr-only">Cargando {que}…</span>
      <div className={`${bloque} h-24`} aria-hidden="true" />
      <div className={`${bloque} h-40`} aria-hidden="true" />
    </div>
  );
}
