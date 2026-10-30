'use client';

import React, { useState } from 'react';

interface FormularioCategoriaProps {
  onAgregar: (nombre: string, tipo: 'gasto' | 'ingreso') => Promise<void>;
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
  const [enviando, setEnviando] = useState(false);
  const [mensajeExito, setMensajeExito] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const nombreLimpio = nombre.trim();
    if (!nombreLimpio || enviando) return;

    setEnviando(true);
    setMensajeExito('');
    try {
      await onAgregar(nombreLimpio, tipo);
      setNombre('');
      setMensajeExito(`Categoría "${nombreLimpio}" agregada correctamente.`);
      setTimeout(() => setMensajeExito(''), 3500);
    } catch {
      // Los errores se manejan por props en 'errores'
    } finally {
      setEnviando(false);
    }
  };

  const bgInput = modoOscuro
    ? '!bg-[#0F1420] !border-gray-700 !text-white placeholder-gray-500 [&>option]:!bg-[#0F1420] [&>option]:!text-white'
    : '!bg-white !border-gray-200 !text-slate-900 placeholder-gray-400 [&>option]:!bg-white [&>option]:!text-slate-900';
  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      {mensajeExito && (
        <div className="p-2 rounded-xl text-[11px] font-semibold bg-emerald-950/60 text-emerald-300 border border-emerald-800/40">
          {mensajeExito}
        </div>
      )}

      {errores.general && (
        <div className="p-2 rounded-xl text-[11px] font-semibold bg-rose-950/60 text-rose-300 border border-rose-800/40">
          {errores.general}
        </div>
      )}

      <div>
        <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Nombre</label>
        <input
          type="text"
          placeholder="Ej. Cine, Mascotas"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          disabled={enviando}
          className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
        />
        {errores.nombre && <p className="text-rose-400 text-[11px] font-medium mt-1">{errores.nombre}</p>}
      </div>

      <div>
        <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Tipo</label>
        <select
          value={tipo}
          onChange={(e) => setTipo(e.target.value as 'gasto' | 'ingreso')}
          disabled={enviando}
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

      <button
        type="submit"
        disabled={enviando || !nombre.trim()}
        className="w-full bg-[#9333EA] disabled:opacity-50 text-white py-2 rounded-xl text-xs font-semibold hover:bg-purple-600 transition shadow-md cursor-pointer"
      >
        {enviando ? 'Guardando...' : 'Añadir Categoría'}
      </button>
    </form>
  );
};
