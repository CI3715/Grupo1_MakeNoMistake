/**
 * Apariencia (modo claro/oscuro y paleta de color).
 *
 * Es solo una preferencia visual: se guarda en el navegador embebido de la app
 * (localStorage), no en SQLite ni en el servidor, y no incluye ningún dato financiero.
 * Si localStorage no está disponible, la app usa los valores por defecto.
 */

export type Modo = 'claro' | 'oscuro' | 'sistema';

export const MODOS: { id: Modo; nombre: string }[] = [
  { id: 'claro', nombre: 'Claro' },
  { id: 'oscuro', nombre: 'Oscuro' },
  { id: 'sistema', nombre: 'Sistema' },
];

export type EscalaTexto = 'normal' | 'grande' | 'extra';

export const ESCALAS_TEXTO: { id: EscalaTexto; nombre: string; etiqueta: string }[] = [
  { id: 'normal', nombre: 'Normal', etiqueta: '100%' },
  { id: 'grande', nombre: 'Grande', etiqueta: '115%' },
  { id: 'extra', nombre: 'Muy grande', etiqueta: '130%' },
];

export type DisenoEscritorio = 'barra-superior' | 'barra-lateral';

export const DISENOS_ESCRITORIO: { id: DisenoEscritorio; nombre: string; descripcion: string }[] = [
  {
    id: 'barra-superior',
    nombre: 'Barra superior',
    descripcion: 'Barra superior flotante redondeada inspirada en Quixotic.',
  },
  {
    id: 'barra-lateral',
    nombre: 'Barra lateral',
    descripcion: 'Dock lateral flotante compacto que se expande al pasar el mouse.',
  },
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
  escala: EscalaTexto;
  diseno: DisenoEscritorio;
}

const CLAVE_MODO = 'cc-modo';
const CLAVE_PALETA = 'cc-paleta';
const CLAVE_ESCALA = 'cc-escala';
const CLAVE_DISENO = 'cc-diseno';
export const EVENTO_APARIENCIA_CAMBIO = 'cc-apariencia-cambio';
const POR_DEFECTO: Apariencia = {
  modo: 'sistema',
  paleta: 'indigo',
  escala: 'normal',
  diseno: 'barra-superior',
};

export function leerApariencia(): Apariencia {
  try {
    const modo = localStorage.getItem(CLAVE_MODO);
    const paleta = localStorage.getItem(CLAVE_PALETA);
    const escala = localStorage.getItem(CLAVE_ESCALA);
    const diseno = localStorage.getItem(CLAVE_DISENO);
    return {
      modo: MODOS.some((m) => m.id === modo) ? (modo as Modo) : POR_DEFECTO.modo,
      paleta: PALETAS.some((p) => p.id === paleta) ? (paleta as IdPaleta) : POR_DEFECTO.paleta,
      escala: ESCALAS_TEXTO.some((e) => e.id === escala)
        ? (escala as EscalaTexto)
        : POR_DEFECTO.escala,
      diseno: DISENOS_ESCRITORIO.some((d) => d.id === diseno)
        ? (diseno as DisenoEscritorio)
        : POR_DEFECTO.diseno,
    };
  } catch {
    return POR_DEFECTO;
  }
}

export function guardarApariencia(a: Apariencia): void {
  try {
    localStorage.setItem(CLAVE_MODO, a.modo);
    localStorage.setItem(CLAVE_PALETA, a.paleta);
    localStorage.setItem(CLAVE_ESCALA, a.escala);
    localStorage.setItem(CLAVE_DISENO, a.diseno);
  } catch {
    // Sin almacenamiento: la preferencia vale solo para esta sesión.
  }
}

/** Escribe data-tema, data-paleta, data-escala y data-diseno en <html>. */
export function aplicarApariencia(a: Apariencia): void {
  if (typeof window === 'undefined') return;
  const oscuro =
    a.modo === 'oscuro' ||
    (a.modo === 'sistema' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  const raiz = document.documentElement;
  raiz.dataset.tema = oscuro ? 'oscuro' : 'claro';
  raiz.dataset.paleta = a.paleta;
  raiz.dataset.escala = a.escala;
  raiz.dataset.diseno = a.diseno;
  window.dispatchEvent(new Event(EVENTO_APARIENCIA_CAMBIO));
}

export function suscribirTema(listener: () => void): () => void {
  if (typeof window === 'undefined') return () => {};
  const mql = window.matchMedia('(prefers-color-scheme: dark)');
  window.addEventListener(EVENTO_APARIENCIA_CAMBIO, listener);
  mql.addEventListener('change', listener);
  return () => {
    window.removeEventListener(EVENTO_APARIENCIA_CAMBIO, listener);
    mql.removeEventListener('change', listener);
  };
}

export function leerSnapshotOscuro(): boolean {
  if (typeof document === 'undefined') return true;
  return document.documentElement.dataset.tema !== 'claro';
}

export function leerSnapshotDiseno(): DisenoEscritorio {
  if (typeof document === 'undefined') return 'barra-superior';
  const val = document.documentElement.dataset.diseno;
  return val === 'barra-lateral' ? 'barra-lateral' : 'barra-superior';
}

/** Misma lógica que `aplicarApariencia`, como texto, para ejecutarla en <head> antes de pintar. */
export const SCRIPT_APARIENCIA = `(function(){var r=document.documentElement,m='sistema',p='indigo',e='normal',d='barra-superior';try{m=localStorage.getItem('${CLAVE_MODO}')||m;p=localStorage.getItem('${CLAVE_PALETA}')||p;e=localStorage.getItem('${CLAVE_ESCALA}')||e;d=localStorage.getItem('${CLAVE_DISENO}')||d;}catch(x){}var o=m==='oscuro'||(m!=='claro'&&window.matchMedia('(prefers-color-scheme: dark)').matches);r.dataset.tema=o?'oscuro':'claro';r.dataset.paleta=p;r.dataset.escala=e;r.dataset.diseno=d;})();`;

