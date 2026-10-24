import React from 'react';
import { ComposicionSaldo as ComposicionTipo } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';

interface Props {
  composicion: ComposicionTipo;
  simbolo: string;
  saldoActual: string;
}

export const ComposicionSaldo: React.FC<Props> = ({ composicion, simbolo, saldoActual }) => {
  return (
    <section aria-labelledby="titulo-composicion" className="mb-6">
      <h3 id="titulo-composicion" className="mb-3 text-xs font-semibold uppercase tracking-wider text-suave">
        Composición del Saldo
      </h3>
      <div className="grid grid-cols-2 gap-2 sm:gap-3 sm:grid-cols-5">
        {/* Saldo Inicial */}
        <div className="rounded-2xl border border-borde bg-superficie p-2.5 shadow-sm sm:p-3.5">
          <span className="block text-[11px] font-medium text-suave sm:text-xs">Saldo Inicial</span>
          <span
            title={formatearMontoLocal(composicion.saldo_inicial, simbolo)}
            className="mt-1 block truncate text-xs font-semibold tabular-nums text-texto sm:mt-1.5 sm:text-base"
          >
            {formatearMontoLocal(composicion.saldo_inicial, simbolo)}
          </span>
        </div>

        {/* Ingresos */}
        <div className="rounded-2xl border border-exito-borde bg-exito-suave p-2.5 shadow-sm sm:p-3.5">
          <span className="block text-[11px] font-bold text-exito-texto sm:text-xs">Ingresos</span>
          <span
            title={`+ ${formatearMontoLocal(composicion.total_ingresos, simbolo)}`}
            className="mt-1 block truncate text-xs font-extrabold tabular-nums text-exito sm:mt-1.5 sm:text-base"
          >
            + {formatearMontoLocal(composicion.total_ingresos, simbolo)}
          </span>
        </div>

        {/* Gastos */}
        <div className="rounded-2xl border border-peligro-borde bg-peligro-suave p-2.5 shadow-sm sm:p-3.5">
          <span className="block text-[11px] font-bold text-peligro-texto sm:text-xs">Gastos</span>
          <span
            title={`− ${formatearMontoLocal(composicion.total_gastos, simbolo)}`}
            className="mt-1 block truncate text-xs font-extrabold tabular-nums text-peligro sm:mt-1.5 sm:text-base"
          >
            − {formatearMontoLocal(composicion.total_gastos, simbolo)}
          </span>
        </div>

        {/* Ajustes */}
        <div className="rounded-2xl border border-info-borde bg-info-suave p-2.5 shadow-sm sm:p-3.5">
          <span className="block text-[11px] font-bold text-info-texto sm:text-xs">Ajustes</span>
          <span
            title={formatearMontoLocal(composicion.total_ajustes, simbolo)}
            className="mt-1 block truncate text-xs font-extrabold tabular-nums text-info sm:mt-1.5 sm:text-base"
          >
            {formatearMontoLocal(composicion.total_ajustes, simbolo)}
          </span>
        </div>

        {/* Saldo Actual */}
        <div className="col-span-2 rounded-2xl bg-linear-to-br from-grad-a to-grad-b p-3 text-white shadow-md ring-1 ring-white/20 sm:col-span-1 sm:p-3.5">
          <span className="block text-[11px] font-medium text-white/80 sm:text-xs">Saldo Actual</span>
          <span
            title={formatearMontoLocal(saldoActual, simbolo)}
            className="mt-1 block truncate text-sm font-extrabold tabular-nums tracking-tight sm:mt-1.5 sm:text-lg"
          >
            {formatearMontoLocal(saldoActual, simbolo)}
          </span>
        </div>
      </div>
    </section>
  );
};