'use client';

import React, { useState, useRef, useEffect } from 'react';
import { CuentaDto } from '../utils/tauriCommands';
import { IconoBilletera, IconoCheckCirculo } from './Iconos';

interface Props {
  cuentas: CuentaDto[];
  cuentaActivaId: string;
  onSeleccionarCuenta: (id: string) => void;
}

export function SelectorCuentaMinimalista({
  cuentas,
  cuentaActivaId,
  onSeleccionarCuenta,
}: Props) {
  const [abierto, setAbierto] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    const alClic = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setAbierto(false);
      }
    };
    const alTecla = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setAbierto(false);
    };
    document.addEventListener('mousedown', alClic);
    document.addEventListener('keydown', alTecla);
    return () => {
      document.removeEventListener('mousedown', alClic);
      document.removeEventListener('keydown', alTecla);
    };
  }, [abierto]);

  const cuentaActiva = cuentas.find((c) => c.id === cuentaActivaId) ?? cuentas[0];

  if (!cuentaActiva) return null;

  return (
    <div ref={ref} className="relative w-full sm:w-auto">
      <button
        type="button"
        onClick={() => setAbierto((v) => !v)}
        aria-haspopup="listbox"
        aria-expanded={abierto}
        className="flex h-10 w-full sm:w-auto cursor-pointer items-center justify-between gap-2.5 rounded-xl border border-borde bg-superficie px-3.5 text-xs font-semibold text-texto shadow-xs transition hover:border-acento hover:bg-superficie-2 focus-visible:outline-2 focus-visible:outline-acento sm:text-sm"
      >
        <span className="flex items-center gap-2 truncate">
          <span className="text-suave">
            <IconoBilletera width={15} height={15} />
          </span>
          <span className="truncate">{cuentaActiva.nombre}</span>
          <span className="rounded-md bg-superficie-2 px-1.5 py-0.5 text-[11px] font-bold text-suave">
            {cuentaActiva.moneda}
          </span>
        </span>
        <svg
          className={`size-4 shrink-0 text-suave transition-transform ${abierto ? 'rotate-180' : ''}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {abierto && (
        <div
          role="listbox"
          className="absolute left-0 sm:left-auto sm:right-0 top-full z-50 mt-1.5 max-h-60 w-full sm:w-72 overflow-y-auto rounded-2xl border border-borde bg-superficie p-1.5 text-texto shadow-2xl backdrop-blur-md"
        >
          <div className="px-2.5 py-1 text-[11px] font-bold uppercase tracking-wider text-suave">
            Seleccionar cuenta
          </div>
          <div className="mt-1 space-y-1">
            {cuentas.map((c) => {
              const esActiva = c.id === cuentaActivaId;
              return (
                <button
                  key={c.id}
                  type="button"
                  role="option"
                  aria-selected={esActiva}
                  onClick={() => {
                    onSeleccionarCuenta(c.id);
                    setAbierto(false);
                  }}
                  className={`flex w-full cursor-pointer items-center justify-between rounded-xl px-3 py-2 text-left text-xs sm:text-sm transition ${
                    esActiva
                      ? 'bg-acento text-acento-fg font-bold shadow-xs'
                      : 'text-texto hover:bg-superficie-2 hover:text-texto'
                  }`}
                >
                  <div className="min-w-0 flex-1 pr-2">
                    <p className="truncate font-medium">{c.nombre}</p>
                    <p className={`text-[11px] ${esActiva ? 'text-white/80' : 'text-suave'}`}>
                      {c.moneda} · Saldo actual
                    </p>
                  </div>
                  {esActiva && (
                    <IconoCheckCirculo size={16} width={16} height={16} className="size-4 shrink-0 text-current ml-2" />
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
