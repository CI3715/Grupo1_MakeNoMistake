/** Tipos de PRESENTACIÓN para las pantallas de Amigos y Grupos. Sin reglas de negocio. */

import type { ControlSincronizacion } from '../utils/useSincronizacion';

export type VistaSocial = 'amigos' | 'grupos';

/**
 * Por qué falló una llamada al servidor, según lo que cuenta el mensaje del error:
 * - sin_sesion: no se ha iniciado sesión (se invita a iniciarla);
 * - vencida: el token venció (401);
 * - sin_conexion: servidor apagado o sin red (solo estas pantallas lo avisan);
 * - otro: cualquier otro error.
 */
export type TipoFallo = 'sin_sesion' | 'vencida' | 'sin_conexion' | 'otro';

export interface FalloSocial {
  tipo: TipoFallo;
  mensaje: string;
}

/** Estado de una carga: cargando, lista con datos, o fallida. */
export type CargaRemota<T> =
  | { estado: 'cargando' }
  | { estado: 'listo'; datos: T }
  | { estado: 'fallo'; fallo: FalloSocial };

/** Un mensaje por campo; «general» es para errores que no son de un campo. */
export type ErroresCampos<C extends string> = Partial<Record<C | 'general', string>>;

/** Lo que comparten todas las pantallas sociales (se pasa como un solo argumento). */
export interface ContextoSocial {
  modoOscuro: boolean;
  irAInicioSesion: () => void;
  /** Lleva a «Mis cuentas» (por ejemplo, para crear la cuenta que recibirá las partes). */
  irACuentas: () => void;
  /** Sincronización con el servidor (Entrega 7). La crea `page.tsx` una sola vez. */
  sincronizacion: ControlSincronizacion;
}

/** Subpantallas dentro de «Grupos». Estado local de la pantalla (RES-06). */
export type VistaGrupos =
  | { tipo: 'lista' }
  | { tipo: 'detalle'; grupoId: string }
  | { tipo: 'nuevo-gasto'; grupoId: string };
