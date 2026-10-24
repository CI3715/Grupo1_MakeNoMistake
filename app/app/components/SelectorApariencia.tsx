'use client';

import { useEffect, useRef, useState } from 'react';
import { IconoPaleta } from './Iconos';
import {
  MODOS,
  PALETAS,
  aplicarApariencia,
  guardarApariencia,
  leerApariencia,
  type Apariencia,
} from '../utils/apariencia';

/**
 * Botón con un panel flotante para elegir modo (claro/oscuro/sistema) y paleta de color.
 * Diseñado para ir sobre la cabecera con degradado (botón translúcido, texto blanco).
 */
export function SelectorApariencia() {
  const [abierto, setAbierto] = useState(false);
  const [apariencia, setApariencia] = useState<Apariencia>(leerApariencia);
  const contenedor = useRef<HTMLDivElement>(null);
  const { modo, paleta } = apariencia;

  // Cerrar con clic fuera o Escape.
  useEffect(() => {
    if (!abierto) return;
    const alPulsar = (e: PointerEvent) => {
      if (contenedor.current && !contenedor.current.contains(e.target as Node)) setAbierto(false);
    };
    const alTecla = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setAbierto(false);
    };
    document.addEventListener('pointerdown', alPulsar);
    document.addEventListener('keydown', alTecla);
    return () => {
      document.removeEventListener('pointerdown', alPulsar);
      document.removeEventListener('keydown', alTecla);
    };
  }, [abierto]);

  // En modo "sistema", seguir cambios del sistema operativo en vivo.
  useEffect(() => {
    if (modo !== 'sistema') return;
    const consulta = window.matchMedia('(prefers-color-scheme: dark)');
    const alCambiar = () => aplicarApariencia({ modo, paleta });
    consulta.addEventListener('change', alCambiar);
    return () => consulta.removeEventListener('change', alCambiar);
  }, [modo, paleta]);

  const cambiar = (nueva: Apariencia) => {
    setApariencia(nueva);
    aplicarApariencia(nueva);
    guardarApariencia(nueva);
  };

  const nombrePaleta = PALETAS.find((p) => p.id === paleta)?.nombre ?? '';

  return (
    <div ref={contenedor} className="relative">
      <button
        type="button"
        onClick={() => setAbierto((v) => !v)}
        aria-haspopup="dialog"
        aria-expanded={abierto}
        title="Apariencia"
        className="inline-flex h-9 items-center gap-2 rounded-xl bg-white/15 px-3 text-xs font-medium text-white ring-1 ring-white/25 backdrop-blur transition hover:bg-white/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white sm:h-10 sm:px-3.5 sm:text-sm"
      >
        <IconoPaleta width={16} height={16} />
        <span>Apariencia</span>
      </button>

      {abierto && (
        <div
          role="dialog"
          aria-label="Apariencia"
          className="absolute right-0 top-full z-40 mt-2 w-64 rounded-2xl border border-borde bg-superficie p-4 text-texto shadow-suave"
        >
          <p className="mb-2 text-xs font-medium text-suave">Tema</p>
          <div role="group" aria-label="Tema" className="grid grid-cols-3 gap-1 rounded-xl bg-superficie-2 p-1">
            {MODOS.map((m) => (
              <button
                key={m.id}
                type="button"
                aria-pressed={modo === m.id}
                onClick={() => cambiar({ modo: m.id, paleta })}
                className={`rounded-lg px-2 py-1.5 text-xs transition sm:text-sm ${
                  modo === m.id
                    ? 'bg-superficie font-medium text-texto shadow-sm'
                    : 'text-suave hover:text-texto'
                }`}
              >
                {m.nombre}
              </button>
            ))}
          </div>

          <p className="mb-2 mt-4 text-xs font-medium text-suave">Color · {nombrePaleta}</p>
          <div role="group" aria-label="Color" className="grid grid-cols-6 gap-2">
            {PALETAS.map((p) => (
              <button
                key={p.id}
                type="button"
                aria-label={p.nombre}
                aria-pressed={paleta === p.id}
                title={p.nombre}
                onClick={() => cambiar({ modo, paleta: p.id })}
                style={{ backgroundColor: p.muestra }}
                className={`size-8 rounded-full transition hover:scale-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento ${
                  paleta === p.id ? 'ring-2 ring-texto ring-offset-2 ring-offset-superficie' : ''
                }`}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
