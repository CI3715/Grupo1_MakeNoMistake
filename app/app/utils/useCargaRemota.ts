'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import type { CargaRemota } from '../types/social';
import { clasificarFallo } from './erroresSocial';

export interface ControlCarga<T> {
  carga: CargaRemota<T>;
  /** Vuelve a pedir los datos. Con `mostrarCarga` falso conserva lo que hay en pantalla. */
  recargar: (mostrarCarga?: boolean) => void;
}

/**
 * Carga datos de un comando con estados «cargando», «listo» y «fallo». Estado local de
 * la pantalla, sin contexto global (RES-06). `cargar` debe ser estable (una función de
 * módulo o un useCallback). El estado solo cambia dentro de las promesas.
 */
export function useCargaRemota<T>(cargar: () => Promise<T>): ControlCarga<T> {
  const [carga, setCarga] = useState<CargaRemota<T>>({ estado: 'cargando' });
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let activo = true;
    cargar()
      .then((datos) => {
        if (activo) setCarga({ estado: 'listo', datos });
      })
      .catch((err: unknown) => {
        if (activo) setCarga({ estado: 'fallo', fallo: clasificarFallo(err) });
      });
    return () => {
      activo = false;
    };
  }, [cargar, version]);

  const recargar = useCallback((mostrarCarga = true) => {
    if (mostrarCarga) setCarga({ estado: 'cargando' });
    setVersion((v) => v + 1);
  }, []);

  return { carga, recargar };
}

/** `true` mientras el componente sigue en pantalla: evita actualizar tras desmontarse. */
export function useMontado() {
  const montado = useRef(true);
  useEffect(() => {
    montado.current = true;
    return () => {
      montado.current = false;
    };
  }, []);
  return montado;
}
