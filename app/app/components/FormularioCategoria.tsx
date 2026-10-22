'use client';

import React, { useState } from 'react';
import { TipoMovimiento } from '../types/cuentas';

interface Props {
  onAgregar: (nombre: string, tipo: TipoMovimiento) => void;
  error?: string;
}

export const FormularioCategoria: React.FC<Props> = ({ onAgregar, error }) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoMovimiento>('gasto');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (nombre.trim()) {
      onAgregar(nombre, tipo);
      setNombre('');
    }
  };

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3 p-5 rounded-2xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-[#151c28] shadow-sm">
      <h4 className="font-bold text-sm">Nueva Categoría</h4>
      <div className="flex gap-2">
        <input
          type="text"
          placeholder="Nombre de categoría"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className="p-2.5 flex-grow border rounded-xl dark:bg-slate-900 dark:border-gray-700 text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
        />
        <select
          value={tipo}
          onChange={(e) => setTipo(e.target.value as TipoMovimiento)}
          className="p-2.5 border rounded-xl dark:bg-slate-900 dark:border-gray-700 text-xs focus:outline-none"
        >
          <option value="gasto">Gasto</option>
          <option value="ingreso">Ingreso</option>
        </select>
        <button type="submit" className="px-4 py-2.5 bg-[#9333EA] hover:bg-purple-600 text-white rounded-xl text-xs font-semibold shadow transition">
          Añadir
        </button>
      </div>
      {error && <span className="text-xs text-rose-500">{error}</span>}
    </form>
  );
};