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

// ---------- Sesión activa en la interfaz (Entrega 5, tareas B y C) ----------

/** Pantallas del bloque de sesión. */
export type VistaSesionCompleta = VistaSesion | 'perfil';

/** Lo que la interfaz sabe de la sesión. El token nunca llega aquí: se queda en Rust. */
export interface SesionUI {
  /** «cargando»: todavía no se preguntó al núcleo al abrir la app. */
  estado: 'cargando' | 'sin_sesion' | 'vigente' | 'vencida';
  nombreVisible: string | null;
  /** AAAA-MM-DDThh:mm:ssZ en UTC. */
  venceEn: string | null;
}
