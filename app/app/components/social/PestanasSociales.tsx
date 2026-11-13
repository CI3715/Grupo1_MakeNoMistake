'use client';

import React, { useRef } from 'react';
import { temaSocial } from '../../utils/temaSocial';

interface Props<C extends string> {
  modoOscuro: boolean;
  /** Prefijo de los ids (único en la pantalla) que enlazan cada pestaña con su panel. */
  idBase: string;
  etiqueta: string;
  pestanas: { clave: C; texto: string }[];
  activa: C;
  onCambio: (clave: C) => void;
  children: React.ReactNode;
}

/**
 * Pestañas accesibles: una sola entra en el orden de tabulación y las flechas (Inicio y Fin
 * también) mueven entre ellas; cada pestaña apunta a su panel y el panel a su pestaña.
 */
export function PestanasSociales<C extends string>({
  modoOscuro,
  idBase,
  etiqueta,
  pestanas,
  activa,
  onCambio,
  children,
}: Props<C>) {
  const tema = temaSocial(modoOscuro);
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});

  const alTeclear = (e: React.KeyboardEvent<HTMLButtonElement>, indice: number) => {
    const total = pestanas.length;
    let destino: number;
    if (e.key === 'ArrowRight') destino = (indice + 1) % total;
    else if (e.key === 'ArrowLeft') destino = (indice - 1 + total) % total;
    else if (e.key === 'Home') destino = 0;
    else if (e.key === 'End') destino = total - 1;
    else return;
    e.preventDefault();
    const clave = pestanas[destino].clave;
    onCambio(clave);
    refs.current[clave]?.focus();
  };

  return (
    <div>
      <div
        role="tablist"
        aria-label={etiqueta}
        className={`mb-4 grid gap-1 rounded-xl border p-1 ${tema.interior}`}
        style={{ gridTemplateColumns: `repeat(${pestanas.length}, minmax(0, 1fr))` }}
      >
        {pestanas.map((p, i) => (
          <button
            key={p.clave}
            ref={(el) => {
              refs.current[p.clave] = el;
            }}
            id={`${idBase}-pestana-${p.clave}`}
            type="button"
            role="tab"
            aria-selected={activa === p.clave}
            aria-controls={`${idBase}-panel`}
            tabIndex={activa === p.clave ? 0 : -1}
            onClick={() => onCambio(p.clave)}
            onKeyDown={(e) => alTeclear(e, i)}
            className={`rounded-lg px-3 py-1.5 text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
              activa === p.clave ? 'bg-[#9333EA] text-white shadow' : tema.textoSuave
            }`}
          >
            {p.texto}
          </button>
        ))}
      </div>
      <div id={`${idBase}-panel`} role="tabpanel" aria-labelledby={`${idBase}-pestana-${activa}`}>
        {children}
      </div>
    </div>
  );
}
