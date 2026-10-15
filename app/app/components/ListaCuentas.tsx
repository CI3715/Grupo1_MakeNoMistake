import React from 'react';
import { Cuenta, Moneda } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';

interface Props {
  cuentas: Cuenta[];
  monedas: Moneda[];
}

export const ListaCuentas: React.FC<Props> = ({ cuentas, monedas }) => {
  const obtenerSimbolo = (codigo: string) => {
    const m = monedas.find((item) => item.codigo === codigo);
    return m ? m.simbolo : codigo;
  };

  const formatearTipo = (tipo: string) => {
    switch (tipo) {
      case 'efectivo': return 'Efectivo';
      case 'banco': return 'Banco';
      case 'billetera_digital': return 'Billetera Digital';
      default: return tipo;
    }
  };

  return (
    <div className="overflow-x-auto my-4">
      <table className="min-w-full divide-y divide-gray-200 border border-gray-200 rounded-lg">
        <thead className="bg-gray-50">
          <tr>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Nombre</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Tipo</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Moneda</th>
            <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase">Saldo Actual</th>
          </tr>
        </thead>
        <tbody className="bg-white divide-y divide-gray-200">
          {cuentas.map((cuenta) => (
            <tr key={cuenta.id}>
              <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">{cuenta.nombre}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{formatearTipo(cuenta.tipo)}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{cuenta.moneda}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm font-semibold text-right text-gray-900">
                {formatearMontoLocal(cuenta.saldo_actual, obtenerSimbolo(cuenta.moneda))}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};