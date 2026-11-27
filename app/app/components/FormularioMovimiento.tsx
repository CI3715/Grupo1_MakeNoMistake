'use client';

import React, { useState } from 'react';
import { Categoria, TipoMovimiento, ValidacionMovimientoError } from '../types/cuentas';

interface Props {
  categorias: Categoria[];
  onSubmit: (datos: {
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => void;
  errores: ValidacionMovimientoError;
}

export const FormularioMovimiento: React.FC<Props> = ({ categorias, onSubmit, errores }) => {
  const hoyLocal = new Date().toISOString().split('T')[0];

  const [tipo, setTipo] = useState<TipoMovimiento>('gasto');
  const [monto, setMonto] = useState('');
  const [fecha, setFecha] = useState(hoyLocal);
  const [categoriaId, setCategoriaId] = useState('');
  const [nota, setNota] = useState('');

  const categoriasFiltradas = categorias.filter((c) => c.tipo === tipo);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({ tipo, monto, fecha, categoria_id: categoriaId, nota });
  };

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4 p-5 rounded-2xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-[#151c28] shadow-sm">
      <h3 className="text-base font-bold">Registrar Movimiento</h3>

      <div className="flex gap-4 text-xs font-medium">
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="radio"
            name="tipo"
            value="gasto"
            checked={tipo === 'gasto'}
            onChange={() => {
              setTipo('gasto');
              setCategoriaId('');
            }}
            className="accent-[#9333EA]"
          />
          <span>Gasto</span>
        </label>
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="radio"
            name="tipo"
            value="ingreso"
            checked={tipo === 'ingreso'}
            onChange={() => {
              setTipo('ingreso');
              setCategoriaId('');
            }}
            className="accent-[#9333EA]"
          />
          <span>Ingreso</span>
        </label>
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-[11px] font-semibold text-gray-500 dark:text-gray-400">Monto</label>
        <input
          type="text"
          placeholder="Ej: 50,00"
          value={monto}
          onChange={(e) => setMonto(e.target.value)}
          className="p-2.5 text-xs border rounded-xl dark:bg-slate-900 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
        />
        {errores.monto && <span className="text-xs text-rose-500">{errores.monto}</span>}
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-[11px] font-semibold text-gray-500 dark:text-gray-400">Fecha</label>
        <input
          type="date"
          value={fecha}
          onChange={(e) => setFecha(e.target.value)}
          className="p-2.5 text-xs border rounded-xl dark:bg-slate-900 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
        />
        {errores.fecha && <span className="text-xs text-rose-500">{errores.fecha}</span>}
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-[11px] font-semibold text-gray-500 dark:text-gray-400">Categoría</label>
        <select
          value={categoriaId}
          onChange={(e) => setCategoriaId(e.target.value)}
          className="p-2.5 text-xs border rounded-xl dark:bg-slate-900 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
        >
          <option value="">Seleccione categoría...</option>
          {categoriasFiltradas.map((c) => (
            <option key={c.id} value={c.id}>
              {c.nombre}
            </option>
          ))}
        </select>
        {errores.categoria && <span className="text-xs text-rose-500">{errores.categoria}</span>}
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-[11px] font-semibold text-gray-500 dark:text-gray-400">Nota (opcional)</label>
        <input
          type="text"
          placeholder="Ej: Mercado del sábado"
          value={nota}
          onChange={(e) => setNota(e.target.value)}
          className="p-2.5 text-xs border rounded-xl dark:bg-slate-900 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
        />
      </div>

      <button type="submit" className="mt-2 py-2.5 bg-[#9333EA] hover:bg-purple-600 text-white font-semibold text-xs rounded-xl shadow-md transition">
        Guardar Movimiento
      </button>
    </form>
  );
};