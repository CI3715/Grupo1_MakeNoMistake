import React, { useState } from 'react';
import { Moneda, NuevaCuenta, TipoCuenta, ErrorCampo } from '../types/cuentas';

interface Props {
  monedas: Moneda[];
  onCrear: (datos: NuevaCuenta) => Promise<void>;
  onCancelar: () => void;
  errorServidor: ErrorCampo | null;
}

export const FormularioCuenta: React.FC<Props> = ({ monedas, onCrear, onCancelar, errorServidor }) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoCuenta>('efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onCrear({
      nombre,
      tipo,
      moneda,
      saldo_inicial: saldoInicial,
    });
  };

  const getError = (campo: string) => {
    return errorServidor?.campo === campo ? errorServidor.mensaje : null;
  };

  return (
    <form onSubmit={handleSubmit} className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm max-w-md mx-auto my-4">
      <h3 className="text-lg font-bold mb-4 text-gray-800">Nueva Cuenta</h3>

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
          placeholder="0.00"
        />
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
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          Guardar Cuenta
        </button>
      </div>
    </form>
  );
};