'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  ActualizarCredencialesPayload,
  ConfigurarDestinoPayload,
  DestinoSincronizacionDto,
  ErrorIntercambio,
  EstadoSincronizacionLibroDto,
  InformeSincronizacionLibroDto,
} from '../types/copia';
import { clasificarErrorIntercambio } from './erroresCopia';
import {
  actualizarCredencialesDestinoCommand,
  configurarDestinoCommand,
  estadoSincronizacionLibroCommand,
  quitarDestinoCommand,
  sincronizarLibroCommand,
} from './tauriCommands';

/*
 * Sincronización del LIBRO entre dispositivos propios (Entrega 8), con un destino Git o una
 * carpeta compartida. No confundir con `useSincronizacion`, que baja los gastos de grupo del
 * servidor (Entrega 7): son dos cosas distintas y no comparten estado.
 */

const MENSAJE_ESTADO = 'No se pudo leer la configuración de la sincronización. Intenta de nuevo.';

export interface ControlSincronizacionLibro {
  /** Todavía no se leyó el estado local. */
  cargando: boolean;
  /** No se pudo leer el estado local (no es un fallo del destino). */
  errorEstado: string | null;
  destino: DestinoSincronizacionDto | null;
  sincronizando: boolean;
  /** Resultado de la última sincronización hecha en esta sesión. */
  informe: InformeSincronizacionLibroDto | null;
  /** Fecha ISO de la última sincronización exitosa (se conserva entre sesiones). */
  ultimaSincronizacion: string | null;
  /** Por qué falló la última sincronización; `null` si salió bien o aún no hubo ninguna. */
  fallo: ErrorIntercambio | null;
  /** «Sincronizar ahora». Devuelve `true` si terminó bien. No lanza dos a la vez. */
  sincronizarAhora: () => Promise<boolean>;
  /** Guarda el destino y sincroniza. Devuelve el error para mostrarlo en el formulario. */
  guardarDestino: (datos: ConfigurarDestinoPayload) => Promise<ErrorIntercambio | null>;
  actualizarCredenciales: (datos: ActualizarCredencialesPayload) => Promise<ErrorIntercambio | null>;
  quitarDestino: () => Promise<ErrorIntercambio | null>;
}

interface Entrada {
  /** Cambia al iniciar o cerrar sesión: es otro libro, así que se empieza de cero. */
  claveLibro: string;
  /** Se llama cuando llegaron eventos de otro dispositivo: las pantallas vuelven a leer el libro. */
  alCambiarLibro: () => void;
}

/** Lo guardado se etiqueta con el libro al que pertenece: nada del libro anterior se ve. */
interface Guardado {
  clave: string;
  cargado: boolean;
  errorEstado: string | null;
  destino: DestinoSincronizacionDto | null;
  ultima: string | null;
  informe: InformeSincronizacionLibroDto | null;
  fallo: ErrorIntercambio | null;
}

function guardadoInicial(clave: string): Guardado {
  return {
    clave,
    cargado: false,
    errorEstado: null,
    destino: null,
    ultima: null,
    informe: null,
    fallo: null,
  };
}

/**
 * Un solo hook, con estado local y sin contexto global (RES-06). Lo crea `page.tsx` una vez
 * para que la sincronización de apertura ocurra desde cualquier pantalla. Al abrir la app, si
 * hay un destino configurado, sincroniza sola; sin conexión no pierde nada (RNF-17): lo
 * pendiente sale en la siguiente sincronización.
 */
export function useSincronizacionLibro({
  claveLibro,
  alCambiarLibro,
}: Entrada): ControlSincronizacionLibro {
  const [guardado, setGuardado] = useState<Guardado>(() => guardadoInicial(claveLibro));
  const [sincronizando, setSincronizando] = useState(false);

  const propio = guardado.clave === claveLibro;

  // Valores que las funciones asíncronas leen sin volver a crearse.
  const claveRef = useRef(claveLibro);
  const alCambiarRef = useRef(alCambiarLibro);
  const enCurso = useRef<{ clave: string } | null>(null);
  useEffect(() => {
    claveRef.current = claveLibro;
    alCambiarRef.current = alCambiarLibro;
  }, [claveLibro, alCambiarLibro]);

  /** Guarda un cambio solo si sigue siendo el mismo libro (si cambió, el resultado ya no sirve). */
  const aplicar = useCallback((clave: string, cambio: Partial<Omit<Guardado, 'clave'>>) => {
    if (clave !== claveRef.current) return;
    setGuardado((previo) => ({
      ...(previo.clave === clave ? previo : guardadoInicial(clave)),
      ...cambio,
    }));
  }, []);

  const recargarEstado = useCallback(async (): Promise<EstadoSincronizacionLibroDto | null> => {
    const clave = claveRef.current;
    try {
      const estado = await estadoSincronizacionLibroCommand();
      aplicar(clave, {
        cargado: true,
        errorEstado: null,
        destino: estado.destino,
        ultima: estado.ultima_sincronizacion,
      });
      return estado;
    } catch {
      aplicar(clave, { cargado: true, errorEstado: MENSAJE_ESTADO });
      return null;
    }
  }, [aplicar]);

  const sincronizarAhora = useCallback(async (): Promise<boolean> => {
    if (enCurso.current?.clave === claveRef.current) return false;
    const clave = claveRef.current;
    const mia = { clave };
    enCurso.current = mia;
    setSincronizando(true);
    try {
      const informe = await sincronizarLibroCommand();
      if (clave !== claveRef.current) return false;
      aplicar(clave, { informe, ultima: informe.ultima_sincronizacion, fallo: null });
      if (informe.recibidos > 0) alCambiarRef.current();
      return true;
    } catch (err) {
      aplicar(clave, { fallo: clasificarErrorIntercambio(err) });
      return false;
    } finally {
      // Si entretanto empezó la de otro libro, esa es la que manda sobre el indicador.
      if (enCurso.current === mia) {
        enCurso.current = null;
        setSincronizando(false);
      }
    }
  }, [aplicar]);

  // Al abrir (y al cambiar de libro): lee el estado local y, si hay destino, sincroniza.
  useEffect(() => {
    let activo = true;
    const t = window.setTimeout(() => {
      void (async () => {
        const estado = await recargarEstado();
        if (activo && estado?.destino) void sincronizarAhora();
      })();
    }, 0);
    return () => {
      activo = false;
      window.clearTimeout(t);
    };
  }, [claveLibro, recargarEstado, sincronizarAhora]);

  const guardarDestino = useCallback(
    async (datos: ConfigurarDestinoPayload): Promise<ErrorIntercambio | null> => {
      const clave = claveRef.current;
      try {
        const destino = await configurarDestinoCommand(datos);
        aplicar(clave, { cargado: true, destino, informe: null, fallo: null });
      } catch (err) {
        return clasificarErrorIntercambio(err);
      }
      // Un dispositivo nuevo se une con el mismo destino y la misma frase: la primera
      // sincronización trae todo el libro.
      void sincronizarAhora();
      return null;
    },
    [aplicar, sincronizarAhora]
  );

  const actualizarCredenciales = useCallback(
    async (datos: ActualizarCredencialesPayload): Promise<ErrorIntercambio | null> => {
      const clave = claveRef.current;
      try {
        await actualizarCredencialesDestinoCommand(datos);
        await recargarEstado();
        aplicar(clave, { fallo: null });
        return null;
      } catch (err) {
        return clasificarErrorIntercambio(err);
      }
    },
    [aplicar, recargarEstado]
  );

  const quitarDestino = useCallback(async (): Promise<ErrorIntercambio | null> => {
    const clave = claveRef.current;
    try {
      await quitarDestinoCommand();
      aplicar(clave, { destino: null, ultima: null, informe: null, fallo: null });
      return null;
    } catch (err) {
      return clasificarErrorIntercambio(err);
    }
  }, [aplicar]);

  const vacio = guardadoInicial(claveLibro);
  const actual = propio ? guardado : vacio;

  return useMemo<ControlSincronizacionLibro>(
    () => ({
      cargando: !actual.cargado,
      errorEstado: actual.errorEstado,
      destino: actual.destino,
      sincronizando,
      informe: actual.informe,
      ultimaSincronizacion: actual.ultima,
      fallo: actual.fallo,
      sincronizarAhora,
      guardarDestino,
      actualizarCredenciales,
      quitarDestino,
    }),
    [
      actual.cargado,
      actual.errorEstado,
      actual.destino,
      actual.informe,
      actual.ultima,
      actual.fallo,
      sincronizando,
      sincronizarAhora,
      guardarDestino,
      actualizarCredenciales,
      quitarDestino,
    ]
  );
}
