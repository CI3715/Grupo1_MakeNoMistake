'use client';

import { olvidarGruposVistos } from './cacheGrupos';
import { useCallback, useEffect, useRef, useState } from 'react';
import { cerrarSesionCommand, estadoSesionCommand, type SesionDto } from './tauriCommands';
import type { SesionUI } from '../types/sesion';

/** Cada cuánto se re-evalúa el vencimiento. No usa red: es una comparación local en Rust. */
const REVISAR_CADA_MS = 10_000;

const SIN_SESION: SesionUI = { estado: 'sin_sesion', nombreVisible: null, venceEn: null };

/** Qué libro está abierto: cuántas veces cambió y, si se conoce, de quién es. */
interface LibroAbierto {
  generacion: number;
  usuarioId: string | null;
}

export interface ControlSesion {
  sesion: SesionUI;
  /** Aviso «Tu sesión venció…» visible (no bloquea nada). */
  avisoVencidaVisible: boolean;
  ocultarAviso: () => void;
  /** Llamar con el resultado de `iniciar_sesion`. */
  alIniciarSesion: (dto: SesionDto) => void;
  /** Vuelve a preguntar al núcleo por el estado (sin red). */
  refrescar: () => Promise<void>;
  /** Borra la sesión del dispositivo. Devuelve un mensaje si falló, o null si salió bien. */
  cerrarSesion: () => Promise<string | null>;
  /**
   * Clave del libro abierto. Cambia al iniciar o cerrar sesión (otro libro) y NO cambia al
   * vencer la sesión (el libro del usuario sigue abierto). Úsala como `key` para volver a
   * montar las pantallas sin dejar a la vista nada del usuario anterior.
   */
  claveLibro: string;
}

/**
 * Estado de la sesión para toda la interfaz. Un solo hook con estado local,
 * sin contexto global (RES-06). Al abrir la app pregunta `estado_sesion`, que
 * NO usa red, así que la sesión se recuerda aunque el servidor esté apagado.
 */
export function useSesion(): ControlSesion {
  const [sesion, setSesion] = useState<SesionUI>({
    estado: 'cargando',
    nombreVisible: null,
    venceEn: null,
  });
  const [avisoVencidaVisible, setAvisoVencidaVisible] = useState(false);
  const [libro, setLibro] = useState<LibroAbierto>({ generacion: 0, usuarioId: null });
  const estadoRef = useRef<SesionUI['estado']>('cargando');

  const aplicar = useCallback((dto: SesionDto) => {
    if (dto.estado === 'vigente') {
      estadoRef.current = 'vigente';
      setSesion({ estado: 'vigente', nombreVisible: dto.nombre_visible, venceEn: dto.vence_en });
      setAvisoVencidaVisible(false);
      return;
    }

    if (dto.estado === 'vencida') {
      if (estadoRef.current === 'vencida') return;
      estadoRef.current = 'vencida';
      setSesion({ estado: 'vencida', nombreVisible: dto.nombre_visible, venceEn: dto.vence_en });
      setAvisoVencidaVisible(true);
      // Entrega 7: la sesión vencida se conserva en el dispositivo. Cerrarla aquí abriría el
      // libro «sin sesión» y el usuario dejaría de ver sus cuentas; el acuerdo es que con el
      // token vencido el libro del usuario sigue abierto y lo personal sigue funcionando.
      return;
    }

    // sin_sesion: si ya se avisó que venció, se conserva el aviso hasta iniciar sesión de nuevo.
    if (estadoRef.current === 'vencida') return;
    estadoRef.current = 'sin_sesion';
    setSesion(SIN_SESION);
  }, []);

  // Si no se puede leer el estado, se trabaja como «sin sesión»: lo personal sigue funcionando.
  const alFallarLectura = useCallback(() => {
    if (estadoRef.current === 'cargando') {
      estadoRef.current = 'sin_sesion';
      setSesion(SIN_SESION);
    }
  }, []);

  const refrescar = useCallback(async () => {
    try {
      aplicar(await estadoSesionCommand());
    } catch {
      alFallarLectura();
    }
  }, [aplicar, alFallarLectura]);

  // Revisión periódica. Todo el estado se actualiza dentro de callbacks (promesa, temporizador,
  // evento), nunca de forma síncrona en el cuerpo del efecto.
  useEffect(() => {
    let activo = true;
    const revisar = () => {
      estadoSesionCommand()
        .then((dto) => {
          if (activo) aplicar(dto);
        })
        .catch(() => {
          if (activo) alFallarLectura();
        });
    };
    const inicial = window.setTimeout(revisar, 0);
    const reloj = window.setInterval(revisar, REVISAR_CADA_MS);
    window.addEventListener('focus', revisar);
    return () => {
      activo = false;
      window.clearTimeout(inicial);
      window.clearInterval(reloj);
      window.removeEventListener('focus', revisar);
    };
  }, [aplicar, alFallarLectura]);

  // Iniciar sesión cambia el libro en Rust antes de que `iniciar_sesion` responda: al subir
  // la clave, las pantallas se montan de nuevo y piden sus datos al libro nuevo.
  const alIniciarSesion = useCallback(
    (dto: SesionDto) => {
      aplicar(dto);
      olvidarGruposVistos();
      setLibro((previo) => ({
        generacion: previo.generacion + 1,
        usuarioId: dto.usuario_id ?? null,
      }));
    },
    [aplicar]
  );

  const cerrarSesion = useCallback(async (): Promise<string | null> => {
    try {
      await cerrarSesionCommand();
    } catch {
      return 'No se pudo cerrar la sesión en este dispositivo. Intenta de nuevo.';
    }
    estadoRef.current = 'sin_sesion';
    setSesion(SIN_SESION);
    setAvisoVencidaVisible(false);
    olvidarGruposVistos();
    setLibro((previo) => ({ generacion: previo.generacion + 1, usuarioId: null }));
    return null;
  }, []);

  const ocultarAviso = useCallback(() => setAvisoVencidaVisible(false), []);

  const claveLibro = `${libro.usuarioId ?? 'libro'}-${libro.generacion}`;

  return {
    sesion,
    avisoVencidaVisible,
    ocultarAviso,
    alIniciarSesion,
    refrescar,
    cerrarSesion,
    claveLibro,
  };
}
