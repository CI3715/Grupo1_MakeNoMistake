'use client';

import { useEffect, useRef, useState } from 'react';
import { IconoPaleta } from './Iconos';
import {
  DISENOS_ESCRITORIO,
  ESCALAS_TEXTO,
  MODOS,
  PALETAS,
  aplicarApariencia,
  guardarApariencia,
  leerApariencia,
  type Apariencia,
} from '../utils/apariencia';

/**
 * Botón con un panel flotante para elegir modo (claro/oscuro/sistema), paleta de color,
 * tamaño de texto (accesibilidad) y disposición de escritorio.
 */
export function SelectorApariencia() {
  const [abierto, setAbierto] = useState(false);
  const [apariencia, setApariencia] = useState<Apariencia>(leerApariencia);
  const contenedor = useRef<HTMLDivElement>(null);
  const { modo, paleta, escala, diseno } = apariencia;

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
    const alCambiar = () => aplicarApariencia({ modo, paleta, escala, diseno });
    consulta.addEventListener('change', alCambiar);
    return () => consulta.removeEventListener('change', alCambiar);
  }, [modo, paleta, escala, diseno]);

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
        title="Personalizar interfaz y accesibilidad"
        className="inline-flex h-9 items-center gap-2 rounded-xl bg-white/15 px-3 text-xs font-semibold text-white ring-1 ring-white/25 backdrop-blur transition hover:bg-white/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white sm:h-10 sm:px-3.5 sm:text-sm cursor-pointer"
      >
        <IconoPaleta width={16} height={16} />
        <span>Personalizar</span>
      </button>

      {abierto && (
        <div
          role="dialog"
          aria-label="Apariencia y Accesibilidad"
          className="absolute right-0 top-full z-50 mt-2 w-72 max-w-[calc(100vw-2rem)] rounded-2xl border border-borde bg-superficie p-4 text-texto shadow-2xl"
        >
          {/* Disposición de escritorio */}
          <div className="mb-4">
            <div className="flex items-center justify-between mb-1.5">
              <p className="text-xs font-bold text-suave uppercase tracking-wider">Disposición</p>
              <span className="text-[11px] font-medium text-acento-texto">Escritorio</span>
            </div>
            <div
              role="group"
              aria-label="Disposición de escritorio"
              className="grid grid-cols-2 gap-1 rounded-xl bg-superficie-2 p-1"
            >
              {DISENOS_ESCRITORIO.map((d) => (
                <button
                  key={d.id}
                  type="button"
                  title={d.descripcion}
                  aria-pressed={diseno === d.id}
                  onClick={() => cambiar({ ...apariencia, diseno: d.id })}
                  className={`rounded-lg px-2 py-1.5 text-xs font-medium transition cursor-pointer text-center ${
                    diseno === d.id
                      ? 'bg-superficie text-texto shadow-sm'
                      : 'text-suave hover:text-texto'
                  }`}
                >
                  {d.nombre.replace('Barra ', '')}
                </button>
              ))}
            </div>
          </div>

          {/* Tamaño de fuente / Accesibilidad */}
          <div className="mb-4">
            <div className="flex items-center justify-between mb-1.5">
              <p className="text-xs font-bold text-suave uppercase tracking-wider">Tamaño de letra</p>
              <span className="text-[11px] font-medium text-suave">Accesibilidad</span>
            </div>
            <div
              role="group"
              aria-label="Tamaño de texto"
              className="grid grid-cols-3 gap-1 rounded-xl bg-superficie-2 p-1"
            >
              {ESCALAS_TEXTO.map((e) => (
                <button
                  key={e.id}
                  type="button"
                  aria-pressed={escala === e.id}
                  onClick={() => cambiar({ ...apariencia, escala: e.id })}
                  className={`rounded-lg px-2 py-1.5 text-xs font-medium transition cursor-pointer text-center ${
                    escala === e.id
                      ? 'bg-superficie text-texto shadow-sm'
                      : 'text-suave hover:text-texto'
                  }`}
                >
                  <span>{e.nombre}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Tema */}
          <div className="mb-4">
            <p className="mb-1.5 text-xs font-bold text-suave uppercase tracking-wider">Tema</p>
            <div
              role="group"
              aria-label="Tema"
              className="grid grid-cols-3 gap-1 rounded-xl bg-superficie-2 p-1"
            >
              {MODOS.map((m) => (
                <button
                  key={m.id}
                  type="button"
                  aria-pressed={modo === m.id}
                  onClick={() => cambiar({ ...apariencia, modo: m.id })}
                  className={`rounded-lg px-2 py-1.5 text-xs font-medium transition cursor-pointer ${
                    modo === m.id
                      ? 'bg-superficie text-texto shadow-sm'
                      : 'text-suave hover:text-texto'
                  }`}
                >
                  {m.nombre}
                </button>
              ))}
            </div>
          </div>

          {/* Color de acento */}
          <div>
            <p className="mb-1.5 text-xs font-bold text-suave uppercase tracking-wider">
              Acento · {nombrePaleta}
            </p>
            <div role="group" aria-label="Color" className="grid grid-cols-6 gap-2 pt-1">
              {PALETAS.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  aria-label={p.nombre}
                  aria-pressed={paleta === p.id}
                  title={p.nombre}
                  onClick={() => cambiar({ ...apariencia, paleta: p.id })}
                  style={{ backgroundColor: p.muestra }}
                  className={`size-7 rounded-full transition hover:scale-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento cursor-pointer ${
                    paleta === p.id ? 'ring-2 ring-texto ring-offset-2 ring-offset-superficie' : ''
                  }`}
                />
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
