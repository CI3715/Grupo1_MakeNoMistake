'use client';

import React, { useState } from 'react';

interface MonedaOption {
  codigo: string;
  nombre: string;
  simbolo: string;
}

interface FormularioCuentaProps {
  monedas: MonedaOption[];
  onCrear: (datos: { nombre: string; tipo: string; moneda: string; saldo_inicial: number }) => void;
  onCancelar: () => void;
  errores?: { campo?: string; mensaje: string }[];
  modoOscuro?: boolean;
}

export const FormularioCuenta: React.FC<FormularioCuentaProps> = ({
  monedas,
  onCrear,
  onCancelar,
  errores = [],
  modoOscuro = true,
}) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState('Efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');

  const errorMap = errores.reduce((acc, err) => {
    if (err.campo) acc[err.campo] = err.mensaje;
    return acc;
  }, {} as Record<string, string>);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const saldoNum = parseFloat(saldoInicial.replace(',', '.')) || 0;
    onCrear({ nombre, tipo, moneda, saldo_inicial: saldoNum });
  };

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#d1d5db', colorScheme: 'light' };

  const textLabel = modoOscuro ? 'text-gray-300' : 'text-gray-700';

  return (
    <div className="w-full">
      <h3 className={`text-lg font-bold mb-4 ${modoOscuro ? 'text-white' : 'text-slate-900'}`}>
        Nueva Cuenta
      </h3>

      {errorMap.general && (
        <div className="p-3 mb-3 bg-rose-950/40 border border-rose-800/60 text-rose-300 rounded-xl text-xs">
          {errorMap.general}
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className={`block text-xs font-semibold mb-1 ${textLabel}`}>
            Nombre de la cuenta
          </label>
          <input
            type="text"
            placeholder="Ej. Cartera, Cuenta Banesco"
            value={nombre}
            onChange={(e) => setNombre(e.target.value)}
            style={estiloInput}
            className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none transition"
          />
          {errorMap.nombre && <p className="text-rose-400 text-[11px] mt-1">{errorMap.nombre}</p>}
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label className={`block text-xs font-semibold mb-1 ${textLabel}`}>
              Tipo de cuenta
            </label>
            <select
              value={tipo}
              onChange={(e) => setTipo(e.target.value)}
              style={estiloInput}
              className="w-full border px-3 py-2 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none cursor-pointer transition"
            >
              <option value="Efectivo">Efectivo</option>
              <option value="Banco">Banco</option>
              <option value="Caja de Ahorro">Caja de Ahorro</option>
              <option value="Billetera Digital">Billetera Digital</option>
            </select>
          </div>

          <div>
            <label className={`block text-xs font-semibold mb-1 ${textLabel}`}>
              Moneda
            </label>
            <select
              value={moneda}
              onChange={(e) => setMoneda(e.target.value)}
              style={estiloInput}
              className="w-full border px-3 py-2 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none cursor-pointer transition"
            >
              {monedas.map((m) => (
                <option key={m.codigo} value={m.codigo}>
                  {m.nombre} ({m.simbolo})
                </option>
              ))}
            </select>
          </div>
        </div>

        <div>
          <label className={`block text-xs font-semibold mb-1 ${textLabel}`}>
            Saldo inicial
          </label>
          <input
            type="text"
            placeholder="0.00"
            value={saldoInicial}
            onChange={(e) => setSaldoInicial(e.target.value)}
            style={estiloInput}
            className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none transition"
          />
          {errorMap.saldo_inicial && <p className="text-rose-400 text-[11px] mt-1">{errorMap.saldo_inicial}</p>}
        </div>

        <div className="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onCancelar}
            className={`px-4 py-2 rounded-xl text-xs font-semibold border transition cursor-pointer ${
              modoOscuro
                ? 'border-gray-700 hover:bg-slate-800 text-gray-300'
                : 'border-gray-300 hover:bg-gray-100 text-slate-700'
            }`}
          >
            Cancelar
          </button>
          <button
            type="submit"
            className="bg-[#9333EA] hover:bg-purple-600 text-white px-5 py-2 rounded-xl text-xs font-semibold shadow transition cursor-pointer"
          >
            Guardar Cuenta
          </button>
        </div>
      </form>
    </div>
  );
};