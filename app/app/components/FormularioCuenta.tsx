import React, { useState } from 'react';
import { Moneda, NuevaCuenta, TipoCuenta, ErrorCampo } from '../types/cuentas';
import { normalizarSaldo } from '../utils/formato';

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
      await onCrear({
        nombre,
        tipo,
        moneda,
        saldo_inicial: normalizarSaldo(saldoInicial),
      });
    } finally {
      setEnviando(false);
    }
  };

  const getError = (campo: string) => {
    const mensajes = errores.filter((e) => e.campo === campo).map((e) => e.mensaje);
    return mensajes.length > 0 ? mensajes.join(' ') : null;
  };

  const erroresGenerales = errores.filter((e) => e.campo === null);

  return (
    <form onSubmit={handleSubmit} className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm max-w-md mx-auto my-4">
      <h3 className="text-lg font-bold mb-4 text-gray-800">Nueva Cuenta</h3>

      {erroresGenerales.length > 0 && (
        <div role="alert" className="mb-4 rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700">
          {erroresGenerales.map((e, i) => (
            <p key={i}>{e.mensaje}</p>
          ))}
        </div>
      )}

      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-700 mb-1">Nombre</label>
        <input
          type="text"
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className={`w-full px-3 py-2 border rounded-md ${getError('nombre') ? 'border-red-500' : 'border-gray-300'}`}
          placeholder="Ej. Cartera, Cuenta nómina"
        />
        {getError('nombre') && <p className="text-red-500 text-xs mt-1">{getError('nombre')}</p>}
      </div>

      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-700 mb-1">Tipo de cuenta</label>
        <select
          value={tipo}
          onChange={(e) => setTipo(e.target.value as TipoCuenta)}
          className="w-full px-3 py-2 border border-gray-300 rounded-md"
        >
          <option value="efectivo">Efectivo</option>
          <option value="banco">Banco</option>
          <option value="billetera_digital">Billetera Digital</option>
        </select>
        {getError('tipo') && <p className="text-red-500 text-xs mt-1">{getError('tipo')}</p>}
      </div>

      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-700 mb-1">Moneda</label>
        <select
          value={moneda}
          onChange={(e) => setMoneda(e.target.value)}
          className={`w-full px-3 py-2 border rounded-md ${getError('moneda') ? 'border-red-500' : 'border-gray-300'}`}
        >
          {monedas.map((m) => (
            <option key={m.codigo} value={m.codigo}>
              {m.nombre} ({m.simbolo})
            </option>
          ))}
        </select>
        {getError('moneda') && <p className="text-red-500 text-xs mt-1">{getError('moneda')}</p>}
      </div>

      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-700 mb-1">Saldo inicial</label>
        <input
          type="text"
          value={saldoInicial}
          onChange={(e) => setSaldoInicial(e.target.value)}
          className={`w-full px-3 py-2 border rounded-md ${getError('saldo_inicial') ? 'border-red-500' : 'border-gray-300'}`}
          placeholder="0,00"
        />
        <p className="text-gray-400 text-xs mt-1">Puedes usar coma o punto para los decimales.</p>
        {getError('saldo_inicial') && <p className="text-red-500 text-xs mt-1">{getError('saldo_inicial')}</p>}
      </div>

      <div className="flex justify-end space-x-3">
        <button
          type="button"
          onClick={onCancelar}
          className="px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
        >
          Cancelar
        </button>
        <button
          type="submit"
          disabled={enviando}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-60"
        >
          Guardar Cuenta
        </button>
      </div>
    </form>
  );
};