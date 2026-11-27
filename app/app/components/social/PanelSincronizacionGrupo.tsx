'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { temaSocial } from '../../utils/temaSocial';
import { BarraSincronizacion } from './BarraSincronizacion';
import { BotonAccion } from './BotonAccion';
import { CuentaDelGrupo } from './CuentaDelGrupo';
import { GastosEnCola } from './GastosEnCola';
import { PartesPendientes } from './PartesPendientes';

interface Props {
  ctx: ContextoSocial;
  grupoId: string;
  moneda: string;
}

/**
 * Todo lo de la Entrega 7 en el detalle de un grupo: sincronizar, cuenta del grupo, partes
 * pendientes y gastos en cola. Vive en su propio archivo para no engordar `DetalleGrupo`.
 */
export function PanelSincronizacionGrupo({ ctx, grupoId, moneda }: Props) {
  const s = ctx.sincronizacion;
  const tema = temaSocial(ctx.modoOscuro);
  const partes = s.estado?.partes_pendientes.filter((p) => p.grupo_id === grupoId) ?? [];
  const cola = s.estado?.cola.filter((g) => g.grupo_id === grupoId) ?? [];

  return (
    <div className="space-y-4">
      <BarraSincronizacion ctx={ctx} />
      <CuentaDelGrupo ctx={ctx} grupoId={grupoId} moneda={moneda} />

      {s.estado === null && s.errorEstado && (
        <div role="alert" className={`rounded-2xl border p-4 text-xs ${tema.avisoError}`}>
          <p className="font-semibold">{s.errorEstado}</p>
          <div className="mt-3">
            <BotonAccion
              modoOscuro={ctx.modoOscuro}
              variante="secundario"
              onClick={() => void s.recargarEstado()}
            >
              Reintentar
            </BotonAccion>
          </div>
        </div>
      )}

      {s.estado === null && !s.errorEstado && (
        <div role="status" aria-busy="true">
          <span className="sr-only">Cargando pendientes y cola…</span>
          <div
            className={`h-20 animate-pulse rounded-2xl border ${tema.tarjeta}`}
            aria-hidden="true"
          />
        </div>
      )}

      {s.estado !== null && (
        <>
          <PartesPendientes ctx={ctx} partes={partes} mostrarVacio />
          <GastosEnCola ctx={ctx} items={cola} />
        </>
      )}
    </div>
  );
}
