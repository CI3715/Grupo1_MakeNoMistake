'use client';

import React, { useEffect, useRef } from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props {
  modoOscuro: boolean;
  titulo: string;
  subtitulo?: string;
  /** Botón o acción a la derecha del título. */
  acciones?: React.ReactNode;
}

/**
 * Título de pantalla (h1). Recibe el foco al abrirse para que quien usa teclado o lector
 * de pantalla sepa dónde está tras cambiar de vista.
 */
export function EncabezadoPantalla({ modoOscuro, titulo, subtitulo, acciones }: Props) {
  const ref = useRef<HTMLHeadingElement>(null);
  const tema = temaSocial(modoOscuro);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  return (
    <header className="mb-5 flex flex-wrap items-start justify-between gap-3">
      <div className="min-w-0">
        <h1
          ref={ref}
          tabIndex={-1}
          className="text-2xl font-bold tracking-tight focus:outline-none"
        >
          {titulo}
        </h1>
        {subtitulo && <p className={`mt-1 text-xs ${tema.textoSuave}`}>{subtitulo}</p>}
      </div>
      {acciones}
    </header>
  );
}
