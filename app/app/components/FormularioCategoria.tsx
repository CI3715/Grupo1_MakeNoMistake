'use client';

import React, { useState } from 'react';
import { IconoGasto, IconoIngreso, IconoMas } from './Iconos';

interface FormularioCategoriaProps {
  /** Devuelve true si la categoría se creó; solo entonces se limpia el campo. */
  onAgregar: (nombre: string, tipo: 'gasto' | 'ingreso') => Promise<boolean>;
  errores: Record<string, string>;
  modoOscuro?: boolean;
}

export const FormularioCategoria: React.FC<FormularioCategoriaProps> = ({
  onAgregar,
  errores,
}) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<'gasto' | 'ingreso'>('gasto');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    // Sin validar aquí: un nombre vacío lo rechaza el dominio y el mensaje se muestra abajo.
    const creada = await onAgregar(nombre.trim(), tipo);
    if (creada) setNombre('');
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3.5">
      <div>
        <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
          Tipo
        </label>
        <div className="grid grid-cols-2 gap-1.5 rounded-xl bg-superficie-2 p-1">
          <button
            type="button"
            onClick={() => setTipo('gasto')}
            className={`inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
              tipo === 'gasto'
                ? 'bg-peligro text-white shadow-sm'
                : 'text-suave hover:text-texto'
            }`}
          >
            <IconoGasto width={13} height={13} />
            Gasto
          </button>
          <button
            type="button"
            onClick={() => setTipo('ingreso')}
            className={`inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
              tipo === 'ingreso'
                ? 'bg-exito text-white shadow-sm'
                : 'text-suave hover:text-texto'
            }`}
          >
            <IconoIngreso width={13} height={13} />
            Ingreso
          </button>
        </div>
        {errores.tipo && <p className="mt-1 text-xs font-medium text-peligro">{errores.tipo}</p>}
      </div>

      <div>
        <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
          Nombre
        </label>
        <input
          type="text"
          placeholder="Ej. Cine, Mascotas"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs text-texto outline-none transition placeholder:text-suave/60 focus:border-acento focus:ring-4 focus:ring-acento/15"
        />
        {errores.nombre && <p className="mt-1 text-xs font-medium text-peligro">{errores.nombre}</p>}
      </div>

      {errores.general && <p className="text-xs font-medium text-peligro">{errores.general}</p>}

      <button
        type="submit"
        className="inline-flex w-full cursor-pointer items-center justify-center gap-1.5 rounded-xl bg-acento py-2.5 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
      >
        <IconoMas width={14} height={14} />
        Añadir Categoría
      </button>
    </form>
  );
};
