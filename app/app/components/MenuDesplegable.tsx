'use client';

import React, { useState, useRef, useEffect } from 'react';
import { IconoCheckCirculo } from './Iconos';

export interface OpcionDesplegable<T extends string = string> {
  valor: T;
  etiqueta: string;
  secundario?: string;
  icono?: React.ReactNode;
}

export interface MenuDesplegableProps<T extends string = string> {
  id?: string;
  valor: T;
  opciones: OpcionDesplegable<T>[];
  alCambiar: (nuevoValor: T) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  anchoPopup?: string;
  icono?: React.ReactNode;
  variante?: 'formulario' | 'boton';
}

export function MenuDesplegable<T extends string = string>({
  id,
  valor,
  opciones,
  alCambiar,
  placeholder = 'Seleccionar...',
  disabled = false,
  className = '',
  anchoPopup = 'w-full',
  icono,
  variante = 'formulario',
}: MenuDesplegableProps<T>) {
  const [abierto, setAbierto] = useState(false);
  const contenedor = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    const alPulsar = (e: MouseEvent) => {
      if (contenedor.current && !contenedor.current.contains(e.target as Node)) {
        setAbierto(false);
      }
    };
    const alTecla = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setAbierto(false);
    };
    document.addEventListener('mousedown', alPulsar);
    document.addEventListener('keydown', alTecla);
    return () => {
      document.removeEventListener('mousedown', alPulsar);
      document.removeEventListener('keydown', alTecla);
    };
  }, [abierto]);

  const seleccionada = opciones.find((op) => op.valor === valor);

  const estiloBaseTrigger =
    variante === 'formulario'
      ? 'w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15 hover:border-acento/50 cursor-pointer flex items-center justify-between gap-2 shadow-xs'
      : 'inline-flex items-center justify-between gap-2.5 rounded-xl border border-borde bg-superficie px-3.5 py-2 text-xs sm:text-sm font-semibold text-texto shadow-xs transition hover:border-acento hover:bg-superficie-2 cursor-pointer';

  return (
    <div ref={contenedor} className={`relative ${className}`}>
      <button
        id={id}
        type="button"
        disabled={disabled}
        onClick={() => setAbierto((v) => !v)}
        aria-haspopup="listbox"
        aria-expanded={abierto}
        className={`${estiloBaseTrigger} ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
      >
        <span className="flex items-center gap-2 truncate">
          {icono ?? seleccionada?.icono}
          <span className="truncate">
            {seleccionada ? seleccionada.etiqueta : placeholder}
          </span>
          {seleccionada?.secundario && (
            <span className="rounded-md bg-superficie px-1.5 py-0.5 text-[10px] font-bold text-suave border border-borde/50">
              {seleccionada.secundario}
            </span>
          )}
        </span>

        <svg
          className={`size-4 shrink-0 text-suave transition-transform duration-200 ${
            abierto ? 'rotate-180' : ''
          }`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {abierto && (
        <div
          role="listbox"
          className={`absolute left-0 top-full z-50 mt-1.5 max-h-60 ${anchoPopup} overflow-y-auto rounded-2xl border border-borde bg-superficie p-1.5 text-texto shadow-2xl backdrop-blur-md`}
        >
          {opciones.length === 0 ? (
            <div className="px-3 py-2 text-xs text-suave">Sin opciones disponibles</div>
          ) : (
            opciones.map((op) => {
              const activa = op.valor === valor;
              return (
                <button
                  key={op.valor}
                  type="button"
                  role="option"
                  aria-selected={activa}
                  onClick={() => {
                    alCambiar(op.valor);
                    setAbierto(false);
                  }}
                  className={`flex w-full cursor-pointer items-center justify-between rounded-xl px-3 py-2 text-left text-xs sm:text-sm font-medium transition ${
                    activa
                      ? 'bg-acento text-acento-fg font-bold shadow-xs'
                      : 'text-texto hover:bg-superficie-2 hover:text-texto'
                  }`}
                >
                  <div className="flex items-center gap-2 truncate pr-2">
                    {op.icono}
                    <span className="truncate">{op.etiqueta}</span>
                    {op.secundario && (
                      <span
                        className={`rounded-md px-1.5 py-0.5 text-[10px] font-semibold ${
                          activa
                            ? 'bg-white/20 text-white'
                            : 'bg-superficie-2 text-suave border border-borde/50'
                        }`}
                      >
                        {op.secundario}
                      </span>
                    )}
                  </div>
                  {activa && (
                    <IconoCheckCirculo
                      width={16}
                      height={16}
                      className="shrink-0 text-current"
                    />
                  )}
                </button>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}
