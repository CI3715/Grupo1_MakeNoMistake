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
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
        {/* Saldo Inicial */}
        <div className="rounded-2xl border border-borde bg-superficie p-3.5 shadow-sm">
          <span className="block text-xs font-medium text-suave">Saldo Inicial</span>
          <span className="mt-1.5 block text-sm font-semibold tabular-nums text-texto sm:text-base">
            {formatearMontoLocal(composicion.saldo_inicial, simbolo)}
          </span>
        </div>

        {/* Ingresos */}
        <div className="rounded-2xl border border-exito-borde bg-exito-suave p-3.5 shadow-sm">
          <span className="block text-xs font-semibold text-exito-texto">Ingresos</span>
          <span className="mt-1.5 block text-sm font-bold tabular-nums text-exito sm:text-base">
            + {formatearMontoLocal(composicion.total_ingresos, simbolo)}
          </span>
        </div>

        {/* Gastos */}
        <div className="rounded-2xl border border-peligro-borde bg-peligro-suave p-3.5 shadow-sm">
          <span className="block text-xs font-semibold text-peligro-texto">Gastos</span>
          <span className="mt-1.5 block text-sm font-bold tabular-nums text-peligro sm:text-base">
            − {formatearMontoLocal(composicion.total_gastos, simbolo)}
          </span>
        </div>

        {/* Ajustes */}
        <div className="rounded-2xl border border-info-borde bg-info-suave p-3.5 shadow-sm">
          <span className="block text-xs font-semibold text-info-texto">Ajustes</span>
          <span className="mt-1.5 block text-sm font-bold tabular-nums text-info sm:text-base">
            {formatearMontoLocal(composicion.total_ajustes, simbolo)}
          </span>
        </div>

        {/* Saldo Actual */}
        <div className="col-span-2 rounded-2xl bg-linear-to-br from-grad-a to-grad-b p-3.5 text-white shadow-md ring-1 ring-white/20 sm:col-span-1">
          <span className="block text-xs font-medium text-white/80">Saldo Actual</span>
          <span className="mt-1.5 block text-base font-extrabold tabular-nums tracking-tight sm:text-lg">
            {formatearMontoLocal(saldoActual, simbolo)}
          </span>
        </div>
      </div>
    </section>
  );
};