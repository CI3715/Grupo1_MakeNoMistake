'use client';

import React, { useState } from 'react';
import type { ControlSincronizacionLibro } from '../../utils/useSincronizacionLibro';
import { temaSocial } from '../../utils/temaSocial';
import { CopiaAvisoSimulacion } from '../copia/CopiaAvisoSimulacion';
import { EncabezadoPantalla } from '../social/EncabezadoPantalla';
import { EstadoCargando } from '../social/EstadoCargando';
import { Tarjeta } from '../social/Tarjeta';
import { SincronizacionLibroDestinoFormulario } from './SincronizacionLibroDestinoFormulario';
import { SincronizacionLibroEstado } from './SincronizacionLibroEstado';
import { SincronizacionLibroResumenDestino } from './SincronizacionLibroResumenDestino';

interface Props {
  modoOscuro: boolean;
  /** Lo crea `page.tsx` una sola vez (sincroniza al abrir la app si hay destino). */
  control: ControlSincronizacionLibro;
}

/**
 * Sincronización del libro entre tus dispositivos (Entrega 8: RF-INT-03, RF-INT-04, RF-INT-05,
 * RF-INT-07). El destino es un repositorio Git o una carpeta que tú controlas; el servidor de
 * Cuentas Claras no participa (RNF-01, INV-16).
 */
export function PantallaSincronizacionLibro({ modoOscuro, control }: Props) {
  const tema = temaSocial(modoOscuro);
  const [cambiando, setCambiando] = useState(false);
  const [editandoCredenciales, setEditandoCredenciales] = useState(false);
  const { destino } = control;

  return (
    <div>
      <EncabezadoPantalla
        modoOscuro={modoOscuro}
        titulo="Sincronización"
        subtitulo="Mantén el mismo libro en tus dispositivos con un repositorio Git o una carpeta tuya. Todo va cifrado."
      />
      <CopiaAvisoSimulacion modoOscuro={modoOscuro} />

      {control.cargando ? (
        <EstadoCargando modoOscuro={modoOscuro} que="la sincronización" />
      ) : control.errorEstado ? (
        <p role="alert" className={`rounded-xl border p-3 text-xs ${tema.avisoError}`}>
          {control.errorEstado}
        </p>
      ) : (
        <div className="space-y-5">
          <Tarjeta
            modoOscuro={modoOscuro}
            titulo="Destino"
            descripcion={
              destino
                ? 'Dónde se guardan los lotes cifrados de tu libro.'
                : 'Elige dónde se guardarán los lotes cifrados de tu libro. Para unir otro dispositivo, configura el mismo destino con la misma frase.'
            }
          >
            {destino && !cambiando ? (
              <SincronizacionLibroResumenDestino
                modoOscuro={modoOscuro}
                destino={destino}
                editandoCredenciales={editandoCredenciales}
                onEditarCredenciales={setEditandoCredenciales}
                onCambiar={() => setCambiando(true)}
                onGuardarCredenciales={control.actualizarCredenciales}
                onQuitar={control.quitarDestino}
              />
            ) : (
              <SincronizacionLibroDestinoFormulario
                modoOscuro={modoOscuro}
                actual={destino}
                onGuardar={async (datos) => {
                  const error = await control.guardarDestino(datos);
                  if (!error) setCambiando(false);
                  return error;
                }}
                onCancelar={destino ? () => setCambiando(false) : undefined}
              />
            )}
          </Tarjeta>

          {destino && !cambiando && (
            <Tarjeta
              modoOscuro={modoOscuro}
              titulo="Sincronizar"
              descripcion="Trae lo nuevo de tus otros dispositivos y envía lo tuyo. Sin conexión no se pierde nada: lo pendiente sale en la siguiente."
            >
              <SincronizacionLibroEstado
                modoOscuro={modoOscuro}
                control={control}
                onActualizarCredenciales={() => setEditandoCredenciales(true)}
              />
            </Tarjeta>
          )}
        </div>
      )}
    </div>
  );
}
