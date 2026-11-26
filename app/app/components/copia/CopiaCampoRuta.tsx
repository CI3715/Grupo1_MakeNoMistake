'use client';

import React, { useId } from 'react';
import { temaSocial } from '../../utils/temaSocial';
import { BotonAccion } from '../social/BotonAccion';

interface Props {
  modoOscuro: boolean;
  etiqueta: string;
  /** Ruta elegida, o `null` si todavía no se eligió. */
  ruta: string | null;
  textoBoton: string;
  onElegir: () => void;
  deshabilitado?: boolean;
  ayuda?: string;
  error?: string;
}

/**
 * Ruta elegida con el diálogo del sistema, en un campo de solo lectura: la persona ve y
 * confirma exactamente dónde se guardará o de dónde se leerá el archivo. Para cambiarla se
 * vuelve a abrir el diálogo; no se escribe a mano.
 */
export function CopiaCampoRuta({
  modoOscuro,
  etiqueta,
  ruta,
  textoBoton,
  onElegir,
  deshabilitado = false,
  ayuda,
  error,
}: Props) {
  const id = useId();
  const idError = `${id}-error`;
  const idAyuda = `${id}-ayuda`;
  const tema = temaSocial(modoOscuro);
  const describedBy = error ? idError : ayuda ? idAyuda : undefined;

  return (
    <div>
      <label htmlFor={id} className={`mb-1 block text-xs font-semibold ${tema.textoSuave}`}>
        {etiqueta}
      </label>
      <div className="flex flex-wrap items-center gap-2">
        <input
          id={id}
          type="text"
          readOnly
          value={ruta ?? ''}
          title={ruta ?? undefined}
          placeholder="Todavía no elegiste ninguna"
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy}
          style={error ? { ...tema.estiloInput, borderColor: '#f43f5e' } : tema.estiloInput}
          className="min-w-0 flex-1 rounded-xl border px-3 py-2.5 text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
        />
        <BotonAccion
          modoOscuro={modoOscuro}
          variante="secundario"
          deshabilitado={deshabilitado}
          onClick={onElegir}
        >
          {textoBoton}
        </BotonAccion>
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
}
