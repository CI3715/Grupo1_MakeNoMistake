/** Tipos de PRESENTACIÓN para las pantallas de sesión. Sin reglas de negocio. */

export type VistaSesion = 'inicio-sesion' | 'registro';

/** Nombres de campo acordados con el servidor (errores como lista de {campo, mensaje}). */
export type CampoRegistro = 'nombre_usuario' | 'nombre_visible' | 'correo' | 'contrasena';
export type CampoInicioSesion = 'correo' | 'contrasena';

/** Un mensaje por campo; «general» es para errores que no son de un campo (p. ej. sin conexión). */
export type ErroresSesion<C extends string> = Partial<Record<C | 'general', string>>;

export interface DatosRegistro {
  nombre_usuario: string;
  nombre_visible: string;
  correo: string;
  contrasena: string;
}

export interface DatosInicioSesion {
  correo: string;
  contrasena: string;
}
