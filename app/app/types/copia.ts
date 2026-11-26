/**
 * Tipos de PRESENTACIÓN de la Entrega 8: copia de seguridad (exportar/importar) y
 * sincronización del libro entre dispositivos propios. Sin reglas de negocio.
 *
 * No confundir con `types/social.ts`: la sincronización de aquí es la del libro con un
 * destino que controla la persona (Git o carpeta), no la de gastos de grupo con el servidor.
 */

/** Un evento que la fusión no aplicó, con el motivo en español (sección 2.7). */
export interface EventoRechazadoDto {
  id: string;
  motivo: string;
}

/** Resultado de importar una copia: «0 nuevos, 44 ya existían» (RF-INT-02, ESC-04). */
export interface ResumenImportacionDto {
  nuevos: number;
  ya_existian: number;
  rechazados: EventoRechazadoDto[];
}

export type TipoDestino = 'carpeta' | 'git';

/** Destino configurado. El token y la frase de paso nunca llegan a la interfaz. */
export interface DestinoSincronizacionDto {
  tipo: TipoDestino;
  /** Ruta de la carpeta o dirección del repositorio. */
  ubicacion: string;
  /** Solo Git. */
  usuario: string | null;
}

export type ConfigurarDestinoPayload =
  | { tipo: 'carpeta'; ubicacion: string; frase: string }
  | { tipo: 'git'; ubicacion: string; usuario: string; token: string; frase: string };

export interface ActualizarCredencialesPayload {
  usuario: string;
  token: string;
}

export interface EstadoSincronizacionLibroDto {
  destino: DestinoSincronizacionDto | null;
  /** Instante ISO de la última sincronización exitosa, o null si aún no hubo ninguna. */
  ultima_sincronizacion: string | null;
}

/** Resultado de una sincronización (RF-INT-04). */
export interface InformeSincronizacionLibroDto {
  enviados: number;
  recibidos: number;
  rechazados: EventoRechazadoDto[];
  ultima_sincronizacion: string;
}

/**
 * Qué falló, según lo que necesita la interfaz para explicarlo (sección 2.7 y campos de
 * formulario). Ver `utils/erroresCopia.ts`.
 */
export type TipoErrorIntercambio =
  | 'frase_incorrecta'
  | 'frase_requerida'
  | 'frase_corta'
  | 'version_no_soportada'
  | 'archivo_invalido'
  | 'destino_inaccesible'
  | 'sin_conexion'
  | 'credenciales_rechazadas'
  | 'otro';

export interface ErrorIntercambio {
  tipo: TipoErrorIntercambio;
  /** Texto en español, sin trazas técnicas (RNF-18). */
  mensaje: string;
  /** Basta con volver a intentar más tarde: sin conexión o destino inaccesible. */
  reintentable: boolean;
}
