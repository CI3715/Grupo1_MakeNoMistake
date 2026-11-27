'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { erroresDeFormulario } from './erroresSesion';
import type { ErroresSesion } from '../types/sesion';

export type ResultadoEnvio<T> = { ok: true; valor: T } | { ok: false };

export interface ControlEnvio<C extends string> {
  /** Un mensaje por campo; «general» para lo que no es de un campo (sin conexión, sesión vencida…). */
  errores: ErroresSesion<C>;
  /** Clave de la acción en curso, o null si no hay ninguna. */
  enCurso: string | null;
  cargando: boolean;
  /** Ejecuta un comando. Con `clave` se sabe cuál fila está trabajando. Ignora dobles envíos. */
  enviar: <T>(accion: () => Promise<T>, clave?: string) => Promise<ResultadoEnvio<T>>;
  limpiar: () => void;
}

/**
 * Estado de «enviar»: carga, errores por campo y protección contra doble clic. Convierte los
 * errores de los comandos en mensajes legibles (nunca texto técnico, RNF-18).
 */
export function useEnvio<C extends string = never>(campos: readonly C[] = []): ControlEnvio<C> {
  const [enCurso, setEnCurso] = useState<string | null>(null);
  const [errores, setErrores] = useState<ErroresSesion<C>>({});
  const montado = useRef(true);
  const ocupado = useRef(false);

  useEffect(() => {
    montado.current = true;
    return () => {
      montado.current = false;
    };
  }, []);

  const enviar = useCallback(
    async <T,>(accion: () => Promise<T>, clave = 'envio'): Promise<ResultadoEnvio<T>> => {
      if (ocupado.current) return { ok: false };
      ocupado.current = true;
      setEnCurso(clave);
      setErrores({});
      try {
        const valor = await accion();
        return { ok: true, valor };
      } catch (err) {
        if (montado.current) setErrores(erroresDeFormulario(err, campos));
        return { ok: false };
      } finally {
        ocupado.current = false;
        if (montado.current) setEnCurso(null);
      }
    },
    // `campos` suele ser una constante de módulo; se compara por referencia.
    [campos]
  );

  const limpiar = useCallback(() => setErrores({}), []);

  return { errores, enCurso, cargando: enCurso !== null, enviar, limpiar };
}
