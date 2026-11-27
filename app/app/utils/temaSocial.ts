import { temaSesion } from './temaSesion';

/** Tokens semánticos para Amigos, Grupos, Copia de Seguridad y Sincronización. */
export function temaSocial(modoOscuro: boolean) {
  return {
    ...temaSesion(modoOscuro),
    /** Línea que separa filas dentro de una tarjeta. */
    linea: 'border-borde',
    /** Fondo de un bloque interior (resultado de búsqueda, tabla). */
    interior: 'bg-superficie-2 border-borde',
    cabeceraTabla: 'bg-superficie-2/70 text-suave',
    positivo: 'text-exito-texto',
    negativo: 'text-peligro-texto',
    neutro: 'text-suave',
    avisoMorado: 'border-acento/40 bg-acento-suave text-acento-texto',
    avisoAmbar: 'border-aviso-borde bg-aviso-suave text-aviso-texto',
    avisoError: 'border-peligro-borde bg-peligro-suave text-peligro-texto',
    avisoOk: 'border-exito-borde bg-exito-suave text-exito-texto',
    etiqueta: 'bg-acento-suave text-acento-texto',
    titulo: 'text-texto',
    avisoRojo: 'border-peligro-borde bg-peligro-suave text-peligro-texto',
    avisoVerde: 'border-exito-borde bg-exito-suave text-exito-texto',
    separador: 'border-borde',
    chip: 'bg-acento-suave text-acento-texto',
    chipAmbar: 'bg-aviso-suave text-aviso-texto',
    chipVerde: 'bg-exito-suave text-exito-texto',
    botonSecundario: 'border-borde hover:bg-superficie-hover text-texto',
  };
}
