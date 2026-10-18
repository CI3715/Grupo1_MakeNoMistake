import React from 'react';
import { Cuenta, Moneda } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';
import { nombreTipo } from '../utils/tipos';
import { IconoTipo } from './Iconos';

interface Props {
  cuentas: Cuenta[];
  monedas: Moneda[];
}

export const ListaCuentas: React.FC<Props> = ({ cuentas, monedas }) => {
  const obtenerSimbolo = (codigo: string) => {
    const m = monedas.find((item) => item.codigo === codigo);
    return m ? m.simbolo : codigo;
  };

  return (
    <ul aria-label="Lista de cuentas" className="divide-y divide-borde">
      {cuentas.map((cuenta) => (
        <li
          key={cuenta.id}
          className="flex items-center gap-4 px-6 py-4 transition-colors hover:bg-superficie-2/60"
        >
          <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-acento-suave text-acento-texto">
            <IconoTipo tipo={cuenta.tipo} />
          </span>
          <div className="min-w-0 flex-1">
            <p className="truncate font-medium">{cuenta.nombre}</p>
            <p className="text-sm text-suave">
              {nombreTipo(cuenta.tipo)}
              <span className="sm:hidden"> · {cuenta.moneda}</span>
            </p>
          </div>
          <span className="hidden shrink-0 rounded-full bg-superficie-2 px-2.5 py-1 text-xs font-medium text-suave sm:inline-block">
            {cuenta.moneda}
          </span>
          <p className="min-w-28 shrink-0 text-right font-semibold tabular-nums">
            {formatearMontoLocal(cuenta.saldo_actual, obtenerSimbolo(cuenta.moneda))}
          </p>
        </li>
      ))}
    </ul>
  );
};
