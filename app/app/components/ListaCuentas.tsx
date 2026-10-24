import React from 'react';
import { Cuenta, Moneda } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';
import { nombreTipo } from '../utils/tipos';
import { IconoTipo, IconoFlechaAdelante } from './Iconos';

interface Props {
  cuentas: Cuenta[];
  monedas: Moneda[];
  onSeleccionarCuenta?: (id: string) => void;
}

export const ListaCuentas: React.FC<Props> = ({ cuentas, monedas, onSeleccionarCuenta }) => {
  const obtenerSimbolo = (codigo: string) => {
    const m = monedas.find((item) => item.codigo === codigo);
    return m ? m.simbolo : codigo;
  };

  return (
    <ul aria-label="Lista de cuentas" className="divide-y divide-borde">
      {cuentas.map((cuenta) => (
        <li key={cuenta.id}>
          <button
            type="button"
            onClick={() => onSeleccionarCuenta?.(cuenta.id)}
            className="group flex w-full items-center gap-4 px-6 py-4 text-left transition-colors hover:bg-superficie-2/70 focus-visible:bg-superficie-2/90 focus-visible:outline-none"
          >
            <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-acento-suave text-acento-texto transition group-hover:scale-105">
              <IconoTipo tipo={cuenta.tipo} />
            </span>
            <div className="min-w-0 flex-1">
              <p className="truncate font-medium transition group-hover:text-acento-texto">
                {cuenta.nombre}
              </p>
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
            <span className="text-suave/40 transition group-hover:translate-x-0.5 group-hover:text-suave">
              <IconoFlechaAdelante width={16} height={16} />
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
};
