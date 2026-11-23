'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { BarraSincronizacion } from './BarraSincronizacion';
import { GastosEnCola } from './GastosEnCola';
import { PartesPendientes } from './PartesPendientes';

interface Props {
  ctx: ContextoSocial;
  onAbrirGrupo: (grupoId: string) => void;
}

/**
 * Sincronización en la lista de grupos: lo pendiente de todos los grupos (los gastos
 * rechazados por ya no ser miembro solo se pueden ver aquí, porque ese grupo ya no abre).
 */
export function PanelSincronizacionLista({ ctx, onAbrirGrupo }: Props) {
  const estado = ctx.sincronizacion.estado;
  return (
    <div className="mb-4 space-y-4">
      <BarraSincronizacion ctx={ctx} />
      {estado && (
        <>
          <PartesPendientes
            ctx={ctx}
            partes={estado.partes_pendientes}
            mostrarGrupo
            onElegirCuenta={onAbrirGrupo}
          />
          <GastosEnCola ctx={ctx} items={estado.cola} mostrarGrupo />
        </>
      )}
    </div>
  );
}
