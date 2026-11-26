'use client';

import React from 'react';
import type { ControlSincronizacionLibro } from '../../utils/useSincronizacionLibro';
import { textoEnviados, textoRecibidos, textoRechazados } from '../../utils/copiaFormato';
import { formatearFechaHora } from '../../utils/formato';
import { temaSocial } from '../../utils/temaSocial';
import { CopiaAvisoError } from '../copia/CopiaAvisoError';
import { CopiaEstadisticas } from '../copia/CopiaEstadisticas';
import { CopiaListaRechazados } from '../copia/CopiaListaRechazados';
import { BotonAccion } from '../social/BotonAccion';

interface Props {
  modoOscuro: boolean;
  control: ControlSincronizacionLibro;
  /** Abre «Actualizar credenciales» en la tarjeta del destino. */
  onActualizarCredenciales: () => void;
}

/**
 * «Sincronizar ahora», la fecha de la última sincronización exitosa y el informe de la última
 * (enviados, recibidos y rechazados con su motivo, RF-INT-04). Si falla, explica qué pasó y
 * que lo pendiente se conserva (RNF-17, RNF-18).
 */
export function SincronizacionLibroEstado({ modoOscuro, control, onActualizarCredenciales }: Props) {
  const tema = temaSocial(modoOscuro);
  const { sincronizando, ultimaSincronizacion, informe, fallo } = control;

  const estadoTexto = sincronizando
    ? 'Sincronizando…'
    : ultimaSincronizacion
      ? `Última sincronización exitosa: ${formatearFechaHora(ultimaSincronizacion)}`
      : 'Todavía no hay una sincronización exitosa.';

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
        <p role="status" aria-live="polite" className={`min-w-0 text-xs ${tema.textoSuave}`}>
          {estadoTexto}
        </p>
        <BotonAccion
          modoOscuro={modoOscuro}
          cargando={sincronizando}
          onClick={() => void control.sincronizarAhora()}
        >
          {sincronizando ? 'Sincronizando…' : 'Sincronizar ahora'}
        </BotonAccion>
      </div>

      {fallo && (
        <CopiaAvisoError
          modoOscuro={modoOscuro}
          error={fallo}
          contexto="sincronizacion"
          onReintentar={() => void control.sincronizarAhora()}
          onActualizarCredenciales={onActualizarCredenciales}
        />
      )}

      {informe && !fallo && (
        <div className="space-y-3">
          <p className={`text-xs ${tema.textoSuave}`}>
            {textoEnviados(informe.enviados)}, {textoRecibidos(informe.recibidos)},{' '}
            {textoRechazados(informe.rechazados.length)}.
            {informe.enviados === 0 && informe.recibidos === 0 && informe.rechazados.length === 0
              ? ' Todo estaba al día.'
              : ''}
          </p>
          <CopiaEstadisticas
            modoOscuro={modoOscuro}
            datos={[
              { etiqueta: 'Enviados', valor: informe.enviados },
              { etiqueta: 'Recibidos', valor: informe.recibidos },
              { etiqueta: 'Rechazados', valor: informe.rechazados.length },
            ]}
          />
          <CopiaListaRechazados modoOscuro={modoOscuro} rechazados={informe.rechazados} />
        </div>
      )}
    </div>
  );
}
