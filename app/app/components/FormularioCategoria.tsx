'use client';

import React, { useState } from 'react';

interface FormularioCategoriaProps {
  /** Devuelve true si la categoría se creó; solo entonces se limpia el campo. */
  onAgregar: (nombre: string, tipo: 'gasto' | 'ingreso') => Promise<boolean>;
  errores: Record<string, string>;
  modoOscuro: boolean;
}

export const FormularioCategoria: React.FC<FormularioCategoriaProps> = ({
  onAgregar,
  errores,
  modoOscuro,
}) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<'gasto' | 'ingreso'>('gasto');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    // Sin validar aquí: un nombre vacío lo rechaza el dominio y el mensaje se muestra abajo.
    const creada = await onAgregar(nombre.trim(), tipo);
    if (creada) setNombre('');
  };

  const bgInput = modoOscuro
    ? '!bg-[#0F1420] !border-gray-700 !text-white placeholder-gray-500 [&>option]:!bg-[#0F1420] [&>option]:!text-white'
    : '!bg-white !border-gray-200 !text-slate-900 placeholder-gray-400 [&>option]:!bg-white [&>option]:!text-slate-900';
  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div>
        <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Nombre</label>
        <input
          type="text"
          placeholder="Ej. Cine, Mascotas"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
        />
        {errores.nombre && <p className="text-rose-400 text-[11px] font-medium mt-1">{errores.nombre}</p>}
      </div>

      <div>
        <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Tipo</label>
        <select
          value={tipo}
          onChange={(e) => setTipo(e.target.value as 'gasto' | 'ingreso')}
          className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer appearance-none ${bgInput}`}
          style={{
            backgroundColor: modoOscuro ? '#0F1420' : '#ffffff',
            color: modoOscuro ? '#ffffff' : '#0f172a',
            borderColor: modoOscuro ? '#374151' : '#e2e8f0',
          }}
        >
          <option value="gasto" style={{ backgroundColor: modoOscuro ? '#0F1420' : '#ffffff', color: modoOscuro ? '#ffffff' : '#0f172a' }}>
            Gasto
          </option>
          <option value="ingreso" style={{ backgroundColor: modoOscuro ? '#0F1420' : '#ffffff', color: modoOscuro ? '#ffffff' : '#0f172a' }}>
            Ingreso
          </option>
        </select>
        {errores.tipo && <p className="text-rose-400 text-[11px] font-medium mt-1">{errores.tipo}</p>}
      </div>

      {errores.general && <p className="text-rose-400 text-[11px] font-medium">{errores.general}</p>}

      <button
        type="submit"
        className="w-full bg-[#9333EA] text-white py-2 rounded-xl text-xs font-semibold hover:bg-purple-600 transition shadow-md"
      >
        Añadir Categoría
      </button>
    </form>
  );
};