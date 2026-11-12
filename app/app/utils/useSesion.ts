'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { cerrarSesionCommand, estadoSesionCommand, type SesionDto } from './tauriCommands';
import type { SesionUI } from '../types/sesion';

/** Cada cuánto se re-evalúa el vencimiento. No usa red: es una comparación local en Rust. */
const REVISAR_CADA_MS = 10_000;

const SIN_SESION: SesionUI = { estado: 'sin_sesion', nombreVisible: null, venceEn: null };

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
      // El token vencido no sirve: se borra del dispositivo (acuerdo del viernes).
      cerrarSesionCommand().catch(() => {});
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

  const alIniciarSesion = useCallback((dto: SesionDto) => aplicar(dto), [aplicar]);

  const cerrarSesion = useCallback(async (): Promise<string | null> => {
    try {
      await cerrarSesionCommand();
    } catch {
      return 'No se pudo cerrar la sesión en este dispositivo. Intenta de nuevo.';
    }
    estadoRef.current = 'sin_sesion';
    setSesion(SIN_SESION);
    setAvisoVencidaVisible(false);
    return null;
  }, []);

  const ocultarAviso = useCallback(() => setAvisoVencidaVisible(false), []);

  return { sesion, avisoVencidaVisible, ocultarAviso, alIniciarSesion, refrescar, cerrarSesion };
}
