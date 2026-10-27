'use client';

import React, { useState } from 'react';
import { ErrorCampo } from '../utils/tauriCommands';

export interface Moneda {
  codigo: string;
  nombre: string;
  simbolo: string;
}

export type TipoCuenta = 'efectivo' | 'banco' | 'billetera_digital';

export interface NuevaCuenta {
  nombre: string;
  tipo: string;
  moneda: string;
  saldo_inicial: string;
}

interface Props {
  monedas: Moneda[];
  onCrear: (datos: NuevaCuenta) => Promise<void>;
  onCancelar: () => void;
  errores: ErrorCampo[];
}

export const FormularioCuenta: React.FC<Props> = ({ monedas, onCrear, onCancelar, errores }) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoCuenta>('efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');
  const [enviando, setEnviando] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setEnviando(true);
    try {
      const saldoLimpio = saldoInicial.trim().replace(',', '.');
      await onCrear({
        nombre,
        tipo,
        moneda,
        saldo_inicial: saldoLimpio,
      });
    } finally {
      setEnviando(false);
    }
  };

  const getError = (campo: string) => {
    const mensajes = errores.filter((e) => e.campo === campo).map((e) => e.mensaje);
    return mensajes.length > 0 ? mensajes.join(' ') : null;
  };

  const erroresGenerales = errores.filter((e) => !e.campo);

  return (
    <form onSubmit={handleSubmit} className="space-y-4 text-left">
      <h3 className="text-lg font-bold mb-4">Nueva Cuenta</h3>

      {erroresGenerales.length > 0 && (
        <div role="alert" className="mb-4 rounded-md border border-red-300 bg-red-50 p-3 text-xs text-red-700">
          {erroresGenerales.map((e, i) => (
            <p key={i}>{e.mensaje}</p>
          ))}
        </div>
      )}

      <div>
        <label className="block text-xs font-semibold mb-1 text-gray-400">Nombre de la cuenta</label>
        <input
          type="text"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className={`w-full px-3 py-2 border rounded-xl text-xs bg-[#0F1420] border-gray-700 text-white focus:outline-none focus:ring-1 focus:ring-purple-500 ${
            getError('nombre') ? 'border-red-500' : ''
          }`}
          placeholder="Ej. Cartera, Cuenta Banesco"
        />
        {getError('nombre') && <p className="text-red-400 text-[11px] mt-1">{getError('nombre')}</p>}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="block text-xs font-semibold mb-1 text-gray-400">Tipo de cuenta</label>
          <select
            value={tipo}
            onChange={(e) => setTipo(e.target.value as TipoCuenta)}
            className="w-full px-3 py-2 border rounded-xl text-xs bg-[#0F1420] border-gray-700 text-white focus:outline-none cursor-pointer"
          >
            <option value="efectivo">Efectivo</option>
            <option value="banco">Banco</option>
            <option value="billetera_digital">Billetera Digital</option>
          </select>
          {getError('tipo') && <p className="text-red-400 text-[11px] mt-1">{getError('tipo')}</p>}
        </div>

        <div>
          <label className="block text-xs font-semibold mb-1 text-gray-400">Moneda</label>
          <select
            value={moneda}
            onChange={(e) => setMoneda(e.target.value)}
            className="w-full px-3 py-2 border rounded-xl text-xs bg-[#0F1420] border-gray-700 text-white focus:outline-none cursor-pointer"
          >
            {monedas.map((m) => (
              <option key={m.codigo} value={m.codigo}>
                {m.nombre} ({m.simbolo})
              </option>
            ))}
          </select>
          {getError('moneda') && <p className="text-red-400 text-[11px] mt-1">{getError('moneda')}</p>}
        </div>
      </div>

      <div>
        <label className="block text-xs font-semibold mb-1 text-gray-400">Saldo inicial</label>
        <input
          type="text"
          value={saldoInicial}
          onChange={(e) => setSaldoInicial(e.target.value)}
          className={`w-full px-3 py-2 border rounded-xl text-xs bg-[#0F1420] border-gray-700 text-white focus:outline-none focus:ring-1 focus:ring-purple-500 ${
            getError('saldo_inicial') ? 'border-red-500' : ''
          }`}
          placeholder="0,00"
        />
        {getError('saldo_inicial') && <p className="text-red-400 text-[11px] mt-1">{getError('saldo_inicial')}</p>}
      </div>

      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={onCancelar}
          className="px-4 py-2 border border-gray-700 text-gray-300 rounded-xl text-xs hover:bg-slate-800 transition font-semibold"
        >
          Cancelar
        </button>
        <button
          type="submit"
          disabled={enviando}
          className="px-4 py-2 bg-[#9333EA] hover:bg-purple-600 text-white rounded-xl text-xs font-semibold shadow transition disabled:opacity-60"
        >
          Guardar Cuenta
        </button>
      </div>
    </form>
  );
};