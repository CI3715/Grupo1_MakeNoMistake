import { temaSesion } from './temaSesion';

/** Colores de las pantallas de Amigos y Grupos (modo oscuro / claro), sobre los de sesión. */
export function temaSocial(modoOscuro: boolean) {
  return {
    ...temaSesion(modoOscuro),
    /** Línea que separa filas dentro de una tarjeta. */
    linea: modoOscuro ? 'border-gray-800' : 'border-gray-200',
    /** Fondo de un bloque interior (resultado de búsqueda, tabla). */
    interior: modoOscuro ? 'bg-[#0F1420] border-gray-800' : 'bg-slate-50 border-gray-200',
    cabeceraTabla: modoOscuro ? 'bg-white/5 text-gray-400' : 'bg-slate-100 text-gray-500',
    positivo: modoOscuro ? 'text-emerald-400' : 'text-emerald-700',
    negativo: modoOscuro ? 'text-rose-400' : 'text-rose-700',
    neutro: modoOscuro ? 'text-gray-300' : 'text-gray-600',
    avisoMorado: modoOscuro
      ? 'border-purple-700/60 bg-purple-950/30 text-purple-100'
      : 'border-purple-200 bg-purple-50 text-purple-900',
    avisoAmbar: modoOscuro
      ? 'border-amber-700/60 bg-amber-950/40 text-amber-100'
      : 'border-amber-300 bg-amber-50 text-amber-900',
    avisoError: modoOscuro
      ? 'border-rose-800/60 bg-rose-950/40 text-rose-300'
      : 'border-rose-300 bg-rose-50 text-rose-800',
    avisoOk: modoOscuro
      ? 'border-emerald-700/50 bg-emerald-950/30 text-emerald-300'
      : 'border-emerald-300 bg-emerald-50 text-emerald-800',
    etiqueta: modoOscuro ? 'bg-purple-900/50 text-purple-200' : 'bg-purple-100 text-purple-800',
    titulo: modoOscuro ? 'text-white' : 'text-slate-900',
    avisoRojo: modoOscuro
      ? 'border-rose-800/60 bg-rose-950/40 text-rose-300'
      : 'border-rose-300 bg-rose-50 text-rose-800',
    avisoVerde: modoOscuro
      ? 'border-emerald-700/50 bg-emerald-950/30 text-emerald-300'
      : 'border-emerald-300 bg-emerald-50 text-emerald-800',
    separador: modoOscuro ? 'border-gray-800' : 'border-gray-200',
    chip: modoOscuro ? 'bg-purple-950/60 text-purple-300' : 'bg-purple-100 text-purple-700',
    chipAmbar: modoOscuro ? 'bg-amber-950/60 text-amber-300' : 'bg-amber-100 text-amber-800',
    chipVerde: modoOscuro ? 'bg-emerald-950/60 text-emerald-300' : 'bg-emerald-100 text-emerald-800',
    botonSecundario: modoOscuro
      ? 'border-gray-700 hover:bg-white/10 text-white'
      : 'border-gray-300 hover:bg-slate-100 text-slate-800',
  };
}
