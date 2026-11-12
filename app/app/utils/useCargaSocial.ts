'use client';

import { useCallback, useEffect, useState } from 'react';
import { erroresDeFormulario, MENSAJE_GENERICO } from './erroresSesion';

export type EstadoCarga<T> =
  | { tipo: 'cargando' }
  | { tipo: 'listo'; datos: T }
  | { tipo: 'error'; mensaje: string };

export interface CargaSocial<T> {
  estado: EstadoCarga<T>;
  /** Vuelve a pedir los datos. Si ya hay datos, se siguen mostrando mientras llegan los nuevos. */
  recargar: () => void;
}

/**
 * Carga de datos de una pantalla social. `cargar` debe ser estable (una función de
 * `tauriCommands` o un `useCallback`). Todo el estado se actualiza dentro de los
 * callbacks de la promesa, nunca de forma síncrona en el efecto.
 */
export function useCargaSocial<T>(cargar: () => Promise<T>): CargaSocial<T> {
  const [estado, setEstado] = useState<EstadoCarga<T>>({ tipo: 'cargando' });
  const [recarga, setRecarga] = useState(0);

  useEffect(() => {
    let vigente = true;
    cargar()
      .then((datos) => {
        if (vigente) setEstado({ tipo: 'listo', datos });
      })
      .catch((err) => {
        if (vigente) {
          setEstado({
            tipo: 'error',
            mensaje: erroresDeFormulario(err, []).general ?? MENSAJE_GENERICO,
          });
        }
      });
    return () => {
      vigente = false;
    };
  }, [cargar, recarga]);

  const recargar = useCallback(() => {
    setEstado((previo) => (previo.tipo === 'listo' ? previo : { tipo: 'cargando' }));
    setRecarga((n) => n + 1);
  }, []);

  return { estado, recargar };
}
