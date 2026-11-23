'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { FalloSocial } from '../types/social';
import type { SesionUI } from '../types/sesion';
import { clasificarFallo } from './erroresSocial';
import {
  estadoSincronizacion,
  reiniciarCursor,
  sincronizar,
  type EstadoSincronizacionDto,
} from './tauriCommands';

/** Cada cuánto se sincroniza mientras la app está abierta (RNF-17, enunciado 2.4). */
export const SINCRONIZAR_CADA_MS = 5 * 60 * 1000;

/** Texto acordado cuando el servidor responde 401. */
export const MENSAJE_SESION_VENCIDA = 'Tu sesión venció, inicia sesión de nuevo';

export interface ControlSincronizacion {
  /** Estado local (última sincronización, partes pendientes, cola). `null` mientras carga. */
  estado: EstadoSincronizacionDto | null;
  /** No se pudo leer el estado local (no es un fallo de red). */
  errorEstado: string | null;
  sincronizando: boolean;
  /** Por qué falló la última sincronización; `null` si salió bien o aún no hubo ninguna. */
  fallo: FalloSocial | null;
  /** La sesión venció: no se sincroniza sola hasta volver a iniciar sesión. */
  detenida: boolean;
  /** Sube cada vez que algo del libro pudo cambiar: las pantallas personales vuelven a leer. */
  versionDatos: number;
  /** Botón «Sincronizar». Devuelve `true` si terminó bien. No lanza dos a la vez. */
  sincronizar: () => Promise<boolean>;
  /** Botón discreto «Volver a bajar todo»: cursor a cero y sincroniza. */
  volverABajarTodo: () => Promise<boolean>;
  /** Vuelve a leer el estado local (sin red) y avisa a las pantallas personales. */
  refrescar: () => Promise<void>;
  /** Vuelve a leer solo el estado local (sin red), sin refrescar el resto de pantallas. */
  recargarEstado: () => Promise<void>;
}

interface Entrada {
  sesionEstado: SesionUI['estado'];
  /** Cambia al iniciar o cerrar sesión: es otro libro, así que se empieza de cero. */
  claveLibro: string;
}

/** Lo guardado se etiqueta con el libro al que pertenece: nada del usuario anterior se ve. */
interface Guardado {
  clave: string;
  estado: EstadoSincronizacionDto | null;
  errorEstado: string | null;
  fallo: FalloSocial | null;
  detenida: boolean;
}

type Modo = 'auto' | 'manual' | 'reinicio';

/** Fallo fijo de «sesión vencida» (una constante evita recrear el objeto en cada render). */
const FALLO_VENCIDA: FalloSocial = { tipo: 'vencida', mensaje: MENSAJE_SESION_VENCIDA };

const MENSAJE_ESTADO = 'No se pudo leer el estado de la sincronización. Intenta de nuevo.';

/**
 * Sincronización con el servidor para toda la interfaz. Un solo hook con estado local,
 * sin contexto global (RES-06). Se dispara: al abrir con sesión vigente, tras registrar un
 * gasto de grupo (lo llama quien registra), al volver la conexión (evento `online`),
 * con el botón y cada 5 minutos. Un candado impide dos a la vez; con la sesión vencida se
 * detiene sin bloquear nada de lo personal.
 */
export function useSincronizacion({ sesionEstado, claveLibro }: Entrada): ControlSincronizacion {
  const [guardado, setGuardado] = useState<Guardado>({
    clave: claveLibro,
    estado: null,
    errorEstado: null,
    fallo: null,
    detenida: false,
  });
  const [sincronizando, setSincronizando] = useState(false);
  const [versionDatos, setVersionDatos] = useState(0);

  const propio = guardado.clave === claveLibro;
  const detenida = sesionEstado === 'vencida' || (propio && guardado.detenida);
  const sesionVigente = sesionEstado === 'vigente';

  // Valores que los temporizadores y eventos leen sin volver a suscribirse.
  /** Sincronización en vuelo y de qué libro es: la de otro libro no bloquea a la del actual. */
  const enCurso = useRef<{ clave: string } | null>(null);
  const claveRef = useRef(claveLibro);
  const vigenteRef = useRef(sesionVigente);
  const detenidaRef = useRef(detenida);
  useEffect(() => {
    claveRef.current = claveLibro;
    vigenteRef.current = sesionVigente;
    detenidaRef.current = detenida;
  }, [claveLibro, sesionVigente, detenida]);

  /** Guarda un cambio solo si sigue siendo el mismo libro (si cambió, el resultado ya no sirve). */
  const aplicar = useCallback((clave: string, cambio: Partial<Omit<Guardado, 'clave'>>) => {
    if (clave !== claveRef.current) return;
    setGuardado((previo) => ({
      ...(previo.clave === clave
        ? previo
        : { clave, estado: null, errorEstado: null, fallo: null, detenida: false }),
      ...cambio,
    }));
  }, []);

  const recargarEstado = useCallback(async () => {
    const clave = claveRef.current;
    try {
      const estado = await estadoSincronizacion();
      aplicar(clave, { estado, errorEstado: null });
    } catch {
      aplicar(clave, { errorEstado: MENSAJE_ESTADO });
    }
  }, [aplicar]);

  const refrescar = useCallback(async () => {
    await recargarEstado();
    setVersionDatos((v) => v + 1);
  }, [recargarEstado]);

  const ejecutar = useCallback(
    async (modo: Modo): Promise<boolean> => {
      if (enCurso.current?.clave === claveRef.current) return false;
      // Solo los disparadores automáticos respetan «sin sesión» y «detenida»; si la persona
      // pulsa el botón, el núcleo responde con el motivo y se le muestra.
      if (modo === 'auto' && (!vigenteRef.current || detenidaRef.current)) return false;

      const clave = claveRef.current;
      const mia = { clave };
      enCurso.current = mia;
      setSincronizando(true);
      try {
        if (modo === 'reinicio') await reiniciarCursor();
        const estado = await sincronizar();
        if (clave !== claveRef.current) return false;
        aplicar(clave, { estado, errorEstado: null, fallo: null });
        setVersionDatos((v) => v + 1);
        return true;
      } catch (err) {
        const fallo = clasificarFallo(err);
        if (fallo.tipo === 'vencida') {
          aplicar(clave, { fallo: FALLO_VENCIDA, detenida: true });
        } else {
          aplicar(clave, { fallo });
        }
        // La cola y las pendientes son locales: se actualizan aunque no haya red.
        await recargarEstado();
        return false;
      } finally {
        // Si entretanto empezó la de otro libro, esa es la que manda sobre el indicador.
        if (enCurso.current === mia) {
          enCurso.current = null;
          setSincronizando(false);
        }
      }
    },
    [aplicar, recargarEstado]
  );

  // Estado local (sin red): al abrir y cada vez que cambia el libro. Va antes que la
  // sincronización de apertura para que el resultado de esta última sea el que quede.
  useEffect(() => {
    const t = window.setTimeout(() => {
      void recargarEstado();
    }, 0);
    return () => window.clearTimeout(t);
  }, [claveLibro, recargarEstado]);

  // Al abrir la app con sesión vigente (y al iniciar sesión, que cambia el libro).
  useEffect(() => {
    if (!sesionVigente) return;
    const t = window.setTimeout(() => {
      void ejecutar('auto');
    }, 0);
    return () => window.clearTimeout(t);
  }, [sesionVigente, claveLibro, ejecutar]);

  // Al volver la conexión.
  useEffect(() => {
    const alVolver = () => {
      void ejecutar('auto');
    };
    window.addEventListener('online', alVolver);
    return () => window.removeEventListener('online', alVolver);
  }, [ejecutar]);

  // Cada 5 minutos mientras la app esté abierta y la sesión sirva.
  useEffect(() => {
    if (!sesionVigente || detenida) return;
    const reloj = window.setInterval(() => {
      void ejecutar('auto');
    }, SINCRONIZAR_CADA_MS);
    return () => window.clearInterval(reloj);
  }, [sesionVigente, detenida, ejecutar]);

  const fallo: FalloSocial | null =
    propio && guardado.fallo
      ? guardado.fallo
      : detenida
        ? FALLO_VENCIDA
        : null;

  const estado = propio ? guardado.estado : null;
  const errorEstado = propio ? guardado.errorEstado : null;

  return useMemo<ControlSincronizacion>(
    () => ({
      estado,
      errorEstado,
      sincronizando,
      fallo,
      detenida,
      versionDatos,
      sincronizar: () => ejecutar('manual'),
      volverABajarTodo: () => ejecutar('reinicio'),
      refrescar,
      recargarEstado,
    }),
    [estado, errorEstado, sincronizando, fallo, detenida, versionDatos, ejecutar, refrescar, recargarEstado]
  );
}
