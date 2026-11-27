'use client';

import React from 'react';
import { temaSesion } from '../../utils/temaSesion';

interface Opcion {
  valor: string;
  etiqueta: string;
}

interface Props {
  id: string;
  etiqueta: string;
  valor: string;
  opciones: readonly Opcion[];
  onCambio: (valor: string) => void;
  /** Texto de la opción vacía (si se pone, el valor "" muestra este texto). */
  placeholder?: string;
  error?: string;
  deshabilitado?: boolean;
  modoOscuro: boolean;
}

/** Lista desplegable con etiqueta y error debajo, igual que CampoTexto. */
export function CampoSelect({
  id,
  etiqueta,
  valor,
  opciones,
  onCambio,
  placeholder,
  error,
  deshabilitado = false,
  modoOscuro,
}: Props) {
  const tema = temaSesion(modoOscuro);
  const idError = `${id}-error`;
  return (
    <div>
      <label htmlFor={id} className={`mb-1 block text-xs font-semibold ${tema.textoSuave}`}>
        {etiqueta}
      </label>
      <select
        id={id}
        name={id}
        value={valor}
        onChange={(e) => onCambio(e.target.value)}
        disabled={deshabilitado}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? idError : undefined}
        style={error ? { ...tema.estiloInput, borderColor: '#f43f5e' } : tema.estiloInput}
        className="w-full rounded-xl border px-3 py-2.5 text-xs focus:outline-none focus:ring-2 focus:ring-purple-500 disabled:opacity-60"
      >
        {placeholder !== undefined && <option value="">{placeholder}</option>}
        {opciones.map((o) => (
          <option key={o.valor} value={o.valor}>
            {o.etiqueta}
          </option>
        ))}
      </select>
      {error && (
        <p id={idError} role="alert" className="mt-1 text-[11px] font-medium text-rose-400">
          {error}
        </p>
      )}
    </div>
  );
}
