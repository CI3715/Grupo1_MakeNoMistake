import type { CSSProperties } from 'react';

/** Mismos colores que usan las demás ventanas de la app (modo oscuro / claro). */
export function temaSesion(modoOscuro: boolean) {
  const estiloInput: CSSProperties = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#e2e8f0', colorScheme: 'light' };

  return {
    tarjeta: modoOscuro
      ? 'bg-[#151C28] border-gray-800/80 text-white'
      : 'bg-white border-gray-200 text-slate-900',
    textoSuave: modoOscuro ? 'text-gray-400' : 'text-gray-500',
    textoEnlace: modoOscuro ? 'text-purple-300 hover:text-purple-200' : 'text-[#9333EA] hover:text-purple-700',
    estiloInput,
  };
}
