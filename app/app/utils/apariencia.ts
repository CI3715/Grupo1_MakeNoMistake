/**
 * Apariencia (modo claro/oscuro y paleta de color).
 *
 * Es solo una preferencia visual: se guarda en el navegador embebido de la app
 * (localStorage), no en SQLite ni en el servidor, y no incluye ningún dato financiero.
 * Si localStorage no está disponible, la app usa los valores por defecto.
 */

import { useSyncExternalStore } from 'react';

export type Modo = 'claro' | 'oscuro' | 'sistema';

export const MODOS: { id: Modo; nombre: string }[] = [
  { id: 'claro', nombre: 'Claro' },
  { id: 'oscuro', nombre: 'Oscuro' },
  { id: 'sistema', nombre: 'Sistema' },
];

/** `muestra` es solo el color de la bolita del selector; los colores reales viven en globals.css. */
export const PALETAS = [
  { id: 'indigo', nombre: 'Índigo', muestra: '#6366f1' },
  { id: 'esmeralda', nombre: 'Esmeralda', muestra: '#10b981' },
  { id: 'oceano', nombre: 'Océano', muestra: '#06b6d4' },
  { id: 'violeta', nombre: 'Violeta', muestra: '#8b5cf6' },
  { id: 'terracota', nombre: 'Terracota', muestra: '#f97316' },
  { id: 'grafito', nombre: 'Grafito', muestra: '#71717a' },
] as const;

export type IdPaleta = (typeof PALETAS)[number]['id'];

export interface Apariencia {
  modo: Modo;
  paleta: IdPaleta;
}

const CLAVE_MODO = 'cc-modo';
const CLAVE_PALETA = 'cc-paleta';
const EVENTO_CAMBIO = 'cc-apariencia-cambio';
const POR_DEFECTO: Apariencia = { modo: 'sistema', paleta: 'indigo' };

export function leerApariencia(): Apariencia {
  try {
    const modo = localStorage.getItem(CLAVE_MODO);
    const paleta = localStorage.getItem(CLAVE_PALETA);
    return {
      modo: MODOS.some((m) => m.id === modo) ? (modo as Modo) : POR_DEFECTO.modo,
      paleta: PALETAS.some((p) => p.id === paleta) ? (paleta as IdPaleta) : POR_DEFECTO.paleta,
    };
  } catch {
    return POR_DEFECTO;
  }
}

export function guardarApariencia(a: Apariencia): void {
  try {
    localStorage.setItem(CLAVE_MODO, a.modo);
    localStorage.setItem(CLAVE_PALETA, a.paleta);
  } catch {
    // Sin almacenamiento: la preferencia vale solo para esta sesión.
  }
}

/** Escribe data-tema y data-paleta en <html>. "sistema" sigue la preferencia del sistema operativo. */
export function aplicarApariencia(a: Apariencia): void {
  if (typeof window === 'undefined') return;
  const oscuro =
    a.modo === 'oscuro' ||
    (a.modo === 'sistema' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  const raiz = document.documentElement;
  raiz.dataset.tema = oscuro ? 'oscuro' : 'claro';
  raiz.dataset.paleta = a.paleta;
  window.dispatchEvent(new Event(EVENTO_CAMBIO));
}

function suscribirTema(listener: () => void): () => void {
  if (typeof window === 'undefined') return () => {};
  const mql = window.matchMedia('(prefers-color-scheme: dark)');
  window.addEventListener(EVENTO_CAMBIO, listener);
  mql.addEventListener('change', listener);
  return () => {
    window.removeEventListener(EVENTO_CAMBIO, listener);
    mql.removeEventListener('change', listener);
  };
}

function leerSnapshotOscuro(): boolean {
  if (typeof document === 'undefined') return true;
  return document.documentElement.dataset.tema !== 'claro';
}

/** Hook reactivo que indica si el tema activo en `<html data-tema>` es oscuro. */
export function useModoOscuro(): boolean {
  return useSyncExternalStore(suscribirTema, leerSnapshotOscuro, () => true);
}

/** Misma lógica que `aplicarApariencia`, como texto, para ejecutarla en <head> antes de pintar. */
export const SCRIPT_APARIENCIA = `(function(){var r=document.documentElement,m='sistema',p='indigo';try{m=localStorage.getItem('${CLAVE_MODO}')||m;p=localStorage.getItem('${CLAVE_PALETA}')||p}catch(e){}var d=m==='oscuro'||(m!=='claro'&&window.matchMedia('(prefers-color-scheme: dark)').matches);r.dataset.tema=d?'oscuro':'claro';r.dataset.paleta=p})();`;
