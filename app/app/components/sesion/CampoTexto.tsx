'use client';

import React, { useState } from 'react';
import { temaSesion } from '../../utils/temaSesion';

interface Props {
  id: string;
  etiqueta: string;
  valor: string;
  onCambio: (valor: string) => void;
  tipo?: 'text' | 'email' | 'password';
  placeholder?: string;
  /** Texto de ayuda informativo bajo el campo (no valida nada). */
  ayuda?: string;
  /** Error del campo que falló; se pinta debajo del campo. */
  error?: string;
  autoComplete?: string;
  deshabilitado?: boolean;
  /** Pone el foco en el campo al abrir la pantalla. */
  autoFocus?: boolean;
  modoOscuro: boolean;
}

/** Campo de formulario con etiqueta, ayuda y error debajo. Las contraseñas traen Mostrar/Ocultar. */
export const CampoTexto: React.FC<Props> = ({
  id,
  etiqueta,
  valor,
  onCambio,
  tipo = 'text',
  placeholder,
  ayuda,
  error,
  autoComplete,
  deshabilitado = false,
  autoFocus = false,
  modoOscuro,
}) => {
  const [visible, setVisible] = useState(false);
  const tema = temaSesion(modoOscuro);
  const esContrasena = tipo === 'password';
  const tipoReal = esContrasena && visible ? 'text' : tipo;
  const idError = `${id}-error`;
  const idAyuda = `${id}-ayuda`;
  const describedBy = [error ? idError : null, ayuda ? idAyuda : null].filter(Boolean).join(' ') || undefined;

  return (
    <div>
      <label htmlFor={id} className={`block text-xs font-semibold mb-1 ${tema.textoSuave}`}>
        {etiqueta}
      </label>
      <div className="relative">
        <input
          id={id}
          name={id}
          type={tipoReal}
          value={valor}
          onChange={(e) => onCambio(e.target.value)}
          placeholder={placeholder}
          autoComplete={autoComplete}
          disabled={deshabilitado}
          autoFocus={autoFocus}
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy}
          style={error ? { ...tema.estiloInput, borderColor: '#f43f5e' } : tema.estiloInput}
          className={`w-full border px-3 py-2.5 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500 disabled:opacity-60 ${
            esContrasena ? 'pr-20' : ''
          }`}
        />
        {esContrasena && (
          <button
            type="button"
            onClick={() => setVisible((v) => !v)}
            aria-pressed={visible}
            aria-label={visible ? 'Ocultar contraseña' : 'Mostrar contraseña'}
            disabled={deshabilitado}
            className={`absolute inset-y-1 right-1.5 rounded-lg px-2 text-[11px] font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-500 ${tema.textoEnlace}`}
          >
            {visible ? 'Ocultar' : 'Mostrar'}
          </button>
        )}
      </div>
      {ayuda && !error && (
        <p id={idAyuda} className={`mt-1 text-[11px] ${tema.textoSuave}`}>
          {ayuda}
        </p>
      )}
      {error && (
        <p id={idError} role="alert" className="mt-1 text-[11px] font-medium text-rose-400">
          {error}
        </p>
      )}
    </div>
  );
};
