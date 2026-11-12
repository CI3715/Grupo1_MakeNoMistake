'use client';

import React, { useCallback } from 'react';
import type { ContextoSocial } from '../../types/social';
import { verGrupoCommand } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useCargaRemota } from '../../utils/useCargaRemota';
import { IconoMas } from '../Iconos';
import { AvisoFalloSocial } from './AvisoFalloSocial';
import { BalancesGrupo } from './BalancesGrupo';
import { BotonAccion } from './BotonAccion';
import { EncabezadoPantalla } from './EncabezadoPantalla';
import { EnlaceVolver } from './EnlaceVolver';
import { EstadoCargando } from './EstadoCargando';
import { GastosGrupo } from './GastosGrupo';
import { MiembrosGrupo } from './MiembrosGrupo';

interface Props {
  ctx: ContextoSocial;
  grupoId: string;
  /** Mensaje de éxito de la acción anterior (p. ej. «Gasto registrado»). */
  aviso: string | null;
  onVolver: () => void;
  onNuevoGasto: () => void;
}

/** Detalle de un grupo: balances, gastos con sus partes y miembros. Solo para miembros. */
export function DetalleGrupo({ ctx, grupoId, aviso, onVolver, onNuevoGasto }: Props) {
  const cargar = useCallback(() => verGrupoCommand(grupoId), [grupoId]);
  const { carga, recargar } = useCargaRemota(cargar);
  const tema = temaSocial(ctx.modoOscuro);

  return (
    <div className="@container">
      <EnlaceVolver modoOscuro={ctx.modoOscuro} texto="Volver a grupos" onClick={onVolver} />

      {carga.estado === 'cargando' && <EstadoCargando modoOscuro={ctx.modoOscuro} que="el grupo" />}

      {carga.estado === 'fallo' && (
        <AvisoFalloSocial ctx={ctx} fallo={carga.fallo} onReintentar={() => recargar()} />
      )}

      {carga.estado === 'listo' && (
        <>
          {(() => {
            const { grupo, miembros, gastos, balances } = carga.datos;
            // Nombre visible de cada usuario, para mostrarlo en vez del nombre de usuario.
            const nombres: Record<string, string> = Object.fromEntries(
              miembros.map((m) => [m.usuario, m.nombre_visible])
            );
            return (
              <>
                <EncabezadoPantalla
                  modoOscuro={ctx.modoOscuro}
                  titulo={grupo.nombre}
                  subtitulo={`Grupos / ${grupo.nombre}${grupo.actividad ? ` · ${grupo.actividad}` : ''} · Moneda ${grupo.moneda}`}
                  acciones={
                    <BotonAccion modoOscuro={ctx.modoOscuro} onClick={onNuevoGasto}>
                      <span className="flex items-center gap-1.5">
                        <IconoMas className="w-3.5 h-3.5" />
                        <span>Gasto compartido</span>
                      </span>
                    </BotonAccion>
                  }
                />
                {grupo.descripcion && (
                  <p className={`-mt-3 mb-4 text-xs ${tema.textoSuave}`}>{grupo.descripcion}</p>
                )}
                {aviso && (
                  <p role="status" className={`mb-4 rounded-xl border p-3 text-xs ${tema.avisoOk}`}>
                    {aviso}
                  </p>
                )}
                <div className="grid gap-4 @2xl:grid-cols-[minmax(0,1fr)_minmax(0,18rem)]">
                  <div className="min-w-0 space-y-4">
                    <BalancesGrupo
                      modoOscuro={ctx.modoOscuro}
                      balances={balances}
                      moneda={grupo.moneda}
                      nombres={nombres}
                    />
                    <GastosGrupo
                      modoOscuro={ctx.modoOscuro}
                      gastos={gastos}
                      moneda={grupo.moneda}
                      nombres={nombres}
                    />
                  </div>
                  <div className="min-w-0">
                    <MiembrosGrupo ctx={ctx} grupo={grupo} miembros={miembros} />
                  </div>
                </div>
              </>
            );
          })()}
        </>
      )}
    </div>
  );
}
