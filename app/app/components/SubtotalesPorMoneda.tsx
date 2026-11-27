import React, { useState } from 'react';
import { Cuenta, Subtotal, Moneda } from '../types/cuentas';
import { formatearMontoLocal } from '../utils/formato';

interface Props {
  subtotales: Subtotal[];
  monedas: Moneda[];
  /** Solo para mostrar cuántas cuentas hay en cada moneda; los totales vienen del núcleo. */
  cuentas: Cuenta[];
}

/** Una tarjeta de saldo por moneda. Nunca hay un total consolidado (INV-02). */
export const SubtotalesPorMoneda: React.FC<Props> = ({ subtotales, monedas, cuentas }) => {
  const [expandido, setExpandido] = useState(false);
  const buscar = (codigo: string) => monedas.find((item) => item.codigo === codigo);
  const obtenerSimbolo = (codigo: string) => buscar(codigo)?.simbolo ?? codigo;
  const obtenerNombre = (codigo: string) => buscar(codigo)?.nombre ?? codigo;

  const limite = 3;
  const hayMas = subtotales.length > limite;
  const tarjetasVisibles = expandido || !hayMas ? subtotales : subtotales.slice(0, limite);

  return (
    <section aria-labelledby="titulo-subtotales">
      <h2 id="titulo-subtotales" className="sr-only">
        Saldo por moneda
      </h2>
      <ul className="flex snap-x gap-4 overflow-x-auto px-1 pb-4 pt-1 md:flex-col md:overflow-visible md:p-0">
        {tarjetasVisibles.map((sub) => {
          const n = cuentas.filter((c) => c.moneda === sub.moneda).length;
          return (
            <li
              key={sub.moneda}
              className="relative min-w-64 flex-1 snap-start overflow-hidden rounded-3xl bg-linear-to-br from-grad-b to-grad-a p-5 text-white shadow-[0_18px_40px_-18px_rgb(0_0_0/0.55)] ring-1 ring-white/15 md:min-w-0 md:flex-none"
            >
              <span aria-hidden="true" className="pointer-events-none absolute -right-10 -top-12 size-40 rounded-full bg-white/10" />
              <span aria-hidden="true" className="pointer-events-none absolute -bottom-16 -left-8 size-44 rounded-full bg-white/5" />

              <div className="relative flex items-start justify-between gap-3">
                <p className="text-sm leading-snug text-white/80">Saldo en {obtenerNombre(sub.moneda)}</p>
                <span className="shrink-0 rounded-md bg-white/15 px-2 py-0.5 text-xs font-semibold tracking-wider">
                  {sub.moneda}
                </span>
              </div>
              <p className="relative mt-5 text-2xl font-semibold tracking-tight tabular-nums sm:text-3xl">
                {formatearMontoLocal(sub.total, obtenerSimbolo(sub.moneda))}
              </p>
              <p className="relative mt-1 text-sm text-white/75">
                {n} {n === 1 ? 'cuenta' : 'cuentas'}
              </p>
            </li>
          );
        })}
      </ul>

      {hayMas && (
        <button
          type="button"
          onClick={() => setExpandido((v) => !v)}
          className="mt-3 flex w-full cursor-pointer items-center justify-center gap-2 rounded-2xl border border-borde bg-superficie px-4 py-2.5 text-xs font-bold text-texto shadow-xs transition hover:bg-superficie-2 hover:border-borde-fuerte active:scale-[0.98]"
        >
          <span>{expandido ? 'Mostrar menos' : `Ver ${subtotales.length - limite} monedas más`}</span>
          <svg
            className={`size-3.5 transition-transform duration-200 ${expandido ? 'rotate-180' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2.5}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      )}
    </section>
  );
};
