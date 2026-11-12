'use client';

import React, { useState } from 'react';
import { ErrorCampo, MonedaDto, NuevaCuentaPayload } from '../utils/tauriCommands';
import { normalizarDecimal } from '../utils/formato';

const CAMPOS_DEL_FORMULARIO = ['nombre', 'tipo', 'moneda', 'saldo_inicial'];

interface FormularioCuentaProps {
  monedas: MonedaDto[];
  onCrear: (datos: NuevaCuentaPayload) => void;
  onCancelar: () => void;
  errores?: ErrorCampo[];
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
  const [tipo, setTipo] = useState('efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');

  const errorMap = errores.reduce(
    (acc, err) => {
      if (err.campo) acc[err.campo] = err.mensaje;
      return acc;
    },
    {} as Record<string, string>
  );

  // Errores sin campo (o de un campo que este formulario no muestra): se ven arriba.
  const erroresGenerales = errores.filter(
    (err) => !err.campo || !CAMPOS_DEL_FORMULARIO.includes(err.campo)
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // El saldo viaja como texto: la validación (negativo, decimales de más) es del dominio.
    onCrear({ nombre, tipo, moneda, saldo_inicial: normalizarDecimal(saldoInicial) });
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

      {erroresGenerales.length > 0 && (
        <div className="mb-4 p-3 rounded-xl border border-rose-800/60 bg-rose-950/40 text-rose-300 text-xs space-y-1">
          {erroresGenerales.map((err, i) => (
            <p key={i}>{err.mensaje}</p>
          ))}
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
              <option value="efectivo">Efectivo</option>
              <option value="banco">Banco</option>
              <option value="billetera_digital">Billetera digital</option>
            </select>
            {errorMap.tipo && <p className="text-rose-400 text-[11px] mt-1">{errorMap.tipo}</p>}
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
            {errorMap.moneda && <p className="text-rose-400 text-[11px] mt-1">{errorMap.moneda}</p>}
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