'use client';

import React, { useState } from 'react';

export interface Movimiento {
  id: string;
  fecha: string;
  categoria: string;
  nota: string;
  monto: number;
  tipo: 'Ingreso' | 'Gasto';
  relacion: string;
}

interface DetalleCuentaProps {
  cuenta: {
    nombre: string;
    moneda: string;
    simbolo: string;
    saldoInicial: number;
    saldoActual: number;
  };
  movimientos: Movimiento[];
  categorias: { Ingreso: string[]; Gasto: string[] };
  ingresos: number;
  gastos: number;
  ajustes: number;
  onGuardarMovimiento: (m: {
    tipo: 'Ingreso' | 'Gasto';
    monto: number;
    fecha: string;
    categoria: string;
    nota: string;
  }) => void;
  onCorregir: (mov: Movimiento) => void;
  onAgregarCategoria: (tipo: 'Ingreso' | 'Gasto', nombre: string) => void;
  modoOscuro?: boolean;
}

export default function DetalleCuenta({
  cuenta,
  movimientos,
  categorias,
  ingresos,
  gastos,
  ajustes,
  onGuardarMovimiento,
  onCorregir,
  onAgregarCategoria,
  modoOscuro = true,
}: DetalleCuentaProps) {
  const hoy = new Date().toISOString().split('T')[0];

  const [tipo, setTipo] = useState<'Ingreso' | 'Gasto'>('Gasto');
  const [monto, setMonto] = useState('');
  const [fecha, setFecha] = useState(hoy);
  const [categoria, setCategoria] = useState('');
  const [nota, setNota] = useState('');

  const [nuevaCatNombre, setNuevaCatNombre] = useState('');
  const [nuevaCatTipo, setNuevaCatTipo] = useState<'Ingreso' | 'Gasto'>('Gasto');

  const bgCard = modoOscuro
    ? 'bg-[#151C28] border-gray-800 text-white'
    : 'bg-white border-gray-200 text-slate-900';

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#d1d5db', colorScheme: 'light' };

  const handleSubmitMovimiento = (e: React.FormEvent) => {
    e.preventDefault();
    const val = parseFloat(monto);
    if (isNaN(val) || val <= 0 || !categoria) return;
    onGuardarMovimiento({ tipo, monto: val, fecha, categoria, nota });
    setMonto('');
    setNota('');
  };

  const handleNuevaCategoriaSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!nuevaCatNombre.trim()) return;
    onAgregarCategoria(nuevaCatTipo, nuevaCatNombre.trim());
    setNuevaCatNombre('');
  };

  return (
    <div className="w-full space-y-6">
      {/* HEADER */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-gray-200 dark:border-gray-800 pb-4">
        <div>
          <h1 className="text-2xl font-extrabold tracking-tight">
            {cuenta.nombre} ({cuenta.moneda})
          </h1>
          <p className="text-xs text-gray-400 mt-0.5">
            Detalle de cuenta y registro de movimientos
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setTipo('Gasto')}
            className="bg-rose-600 hover:bg-rose-700 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer shadow-sm transition"
          >
            — Nuevo Gasto
          </button>
          <button
            onClick={() => setTipo('Ingreso')}
            className="bg-emerald-600 hover:bg-emerald-700 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer shadow-sm transition"
          >
            + Nuevo Ingreso
          </button>
        </div>
      </div>

      {/* COMPOSICIÓN DEL SALDO */}
      <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
        <h2 className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-4">
          Composición del Saldo
        </h2>
        <div className="grid grid-cols-2 sm:grid-cols-5 gap-3 text-center">
          <div className="p-3 rounded-xl bg-gray-500/10">
            <span className="text-[11px] text-gray-400 font-bold block">Saldo Inicial</span>
            <span className="text-sm font-extrabold text-gray-300">
              {cuenta.simbolo} {cuenta.saldoInicial.toFixed(2)}
            </span>
          </div>
          <div className="p-3 rounded-xl bg-emerald-950/30 border border-emerald-800/30">
            <span className="text-[11px] text-emerald-400 font-bold block">Ingresos</span>
            <span className="text-sm font-extrabold text-emerald-400">
              + {cuenta.simbolo} {ingresos.toFixed(2)}
            </span>
          </div>
          <div className="p-3 rounded-xl bg-rose-950/30 border border-rose-800/30">
            <span className="text-[11px] text-rose-400 font-bold block">Gastos</span>
            <span className="text-sm font-extrabold text-rose-400">
              — {cuenta.simbolo} {gastos.toFixed(2)}
            </span>
          </div>
          <div className="p-3 rounded-xl bg-blue-950/30 border border-blue-800/30">
            <span className="text-[11px] text-blue-400 font-bold block">Ajustes</span>
            <span className="text-sm font-extrabold text-blue-400">
              {cuenta.simbolo} {ajustes.toFixed(2)}
            </span>
          </div>
          <div className="p-3 rounded-xl bg-purple-950/40 border border-purple-800/40 col-span-2 sm:col-span-1">
            <span className="text-[11px] text-purple-300 font-bold block">Saldo Actual</span>
            <span className="text-sm font-extrabold text-purple-200">
              {cuenta.simbolo} {cuenta.saldoActual.toFixed(2)}
            </span>
          </div>
        </div>
      </div>

      {/* HISTORIAL + NUEVA CATEGORÍA + FORMULARIO */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Columna Izquierda: Tabla e Inserción de Categoría */}
        <div className="lg:col-span-2 space-y-6">
          {/* Tabla de Movimientos con Scroll Horizontal Blindado */}
          <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
            <h3 className="text-sm font-bold mb-4">Historial de Movimientos</h3>
            
            {movimientos.length === 0 ? (
              <p className="text-xs text-gray-400 text-center py-8">
                No hay movimientos registrados aún en esta cuenta.
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-left text-xs border-collapse min-w-[600px]">
                  <thead>
                    <tr className="border-b border-gray-700/60 text-gray-400 font-semibold uppercase text-[10px] tracking-wider">
                      <th className="pb-2.5 pr-2">Fecha</th>
                      <th className="pb-2.5 px-2">Categoría</th>
                      <th className="pb-2.5 px-2">Nota</th>
                      <th className="pb-2.5 px-2 text-right">Monto</th>
                      <th className="pb-2.5 px-2 text-center">Estado / Relación</th>
                      <th className="pb-2.5 pl-2 text-right">Acción</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800">
                    {movimientos.map((m) => (
                      <tr key={m.id} className="hover:bg-white/5 transition-colors">
                        <td className="py-3 pr-2 text-gray-400 whitespace-nowrap">{m.fecha}</td>
                        <td className="py-3 px-2 font-medium whitespace-nowrap">{m.categoria}</td>
                        <td className="py-3 px-2 text-gray-400 max-w-[120px] truncate" title={m.nota}>
                          {m.nota || '—'}
                        </td>
                        <td
                          className={`py-3 px-2 text-right font-bold whitespace-nowrap ${
                            m.tipo === 'Gasto' ? 'text-rose-400' : 'text-emerald-400'
                          }`}
                        >
                          {m.tipo === 'Gasto' ? `- ${cuenta.simbolo}` : `+ ${cuenta.simbolo}`}{' '}
                          {m.monto.toFixed(2)}
                        </td>
                        <td className="py-3 px-2 text-center whitespace-nowrap">
                          {m.relacion === 'Corregido' ? (
                            <span className="bg-amber-950/60 text-amber-300 border border-amber-800/40 text-[10px] font-bold px-2 py-0.5 rounded-md">
                              Corregido
                            </span>
                          ) : m.relacion && m.relacion.includes('Ajuste') ? (
                            <span className="bg-blue-950/60 text-blue-300 border border-blue-800/40 text-[10px] font-bold px-2 py-0.5 rounded-md">
                              {m.relacion}
                            </span>
                          ) : (
                            <span className="text-[11px] text-gray-400">Normal</span>
                          )}
                        </td>
                        <td className="py-3 pl-2 text-right whitespace-nowrap">
                          <button
                            onClick={() => onCorregir(m)}
                            className="bg-purple-600 hover:bg-purple-700 text-white text-[11px] px-2.5 py-1 rounded-lg font-medium transition-all cursor-pointer"
                          >
                            Corregir
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          {/* Agregar Nueva Categoría */}
          <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
            <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
            <form onSubmit={handleNuevaCategoriaSubmit} className="flex flex-col sm:flex-row gap-3">
              <input
                type="text"
                placeholder="Nombre de categoría"
                value={nuevaCatNombre}
                onChange={(e) => setNuevaCatNombre(e.target.value)}
                style={estiloInput}
                className="flex-1 border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
              <select
                value={nuevaCatTipo}
                onChange={(e) => setNuevaCatTipo(e.target.value as 'Ingreso' | 'Gasto')}
                style={estiloInput}
                className="border px-3 py-2 rounded-xl text-xs font-medium focus:outline-none cursor-pointer"
              >
                <option value="Gasto">Gasto</option>
                <option value="Ingreso">Ingreso</option>
              </select>
              <button
                type="submit"
                className="bg-purple-600 hover:bg-purple-700 text-white px-5 py-2 rounded-xl text-xs font-semibold cursor-pointer transition"
              >
                Añadir
              </button>
            </form>
          </div>
        </div>

        {/* Panel Lateral: Registrar Movimiento */}
        <div className={`p-5 rounded-2xl border shadow-sm h-fit ${bgCard}`}>
          <h3 className="text-sm font-bold mb-4">Registrar Movimiento</h3>
          <form onSubmit={handleSubmitMovimiento} className="space-y-4">
            <div className="flex gap-4 text-xs font-medium">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="tipo"
                  checked={tipo === 'Gasto'}
                  onChange={() => setTipo('Gasto')}
                  className="accent-purple-500"
                />
                Gasto
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="tipo"
                  checked={tipo === 'Ingreso'}
                  onChange={() => setTipo('Ingreso')}
                  className="accent-purple-500"
                />
                Ingreso
              </label>
            </div>

            <div>
              <label htmlFor="mov-monto" className="block text-[11px] font-semibold text-gray-400 mb-1">
                Monto ({cuenta.simbolo})
              </label>
              <input
                id="mov-monto"
                type="number"
                step="0.01"
                placeholder="Ej: 50.00"
                value={monto}
                onChange={(e) => setMonto(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
                required
              />
            </div>

            <div>
              <label htmlFor="mov-fecha" className="block text-[11px] font-semibold text-gray-400 mb-1">
                Fecha
              </label>
              <input
                id="mov-fecha"
                type="date"
                value={fecha}
                onChange={(e) => setFecha(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
                required
              />
            </div>

            <div>
              <label htmlFor="mov-categoria" className="block text-[11px] font-semibold text-gray-400 mb-1">
                Categoría
              </label>
              <select
                id="mov-categoria"
                value={categoria}
                onChange={(e) => setCategoria(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer"
                required
              >
                <option value="">Seleccione categoría...</option>
                {categorias[tipo] &&
                  categorias[tipo].map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
              </select>
            </div>

            <div>
              <label htmlFor="mov-nota" className="block text-[11px] font-semibold text-gray-400 mb-1">
                Nota (opcional)
              </label>
              <input
                id="mov-nota"
                type="text"
                placeholder="Ej: Mercado del sábado"
                value={nota}
                onChange={(e) => setNota(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <button
              type="submit"
              className="w-full bg-purple-600 hover:bg-purple-700 text-white py-2.5 rounded-xl font-semibold text-xs transition cursor-pointer shadow-md mt-2"
            >
              Añadir Movimiento
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}