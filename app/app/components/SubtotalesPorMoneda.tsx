import React from 'react';
import { Subtotal, Moneda } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';

interface Props {
  subtotales: Subtotal[];
  monedas: Moneda[];
}

export const SubtotalesPorMoneda: React.FC<Props> = ({ subtotales, monedas }) => {
  const obtenerSimbolo = (codigo: string) => {
    const m = monedas.find((item) => item.codigo === codigo);
    return m ? m.simbolo : codigo;
  };

  return (
    <div className="bg-gray-50 p-4 rounded-md border border-gray-200 my-4">
      <h4 className="text-sm font-semibold text-gray-600 uppercase mb-2">Subtotales por Moneda</h4>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {subtotales.map((sub) => (
          <div key={sub.moneda} className="bg-white p-3 rounded shadow-sm border border-gray-100">
            <span className="text-xs text-gray-400 block font-bold">{sub.moneda}</span>
            <span className="text-md font-semibold text-gray-800">
              {formatearMontoLocal(sub.total, obtenerSimbolo(sub.moneda))}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};