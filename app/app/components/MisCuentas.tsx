'use client';

import React from 'react';
import { Moneda, ListaCuentas as DatosLista, NuevaCuenta, ErrorCampo } from '../types/cuentas';
import { EstadoVacio } from './EstadoVacio';
import { FormularioCuenta } from './FormularioCuenta';
import { IconoMas } from './Iconos';
import { ListaCuentas } from './ListaCuentas';
import { SelectorApariencia } from './SelectorApariencia';
import { SubtotalesPorMoneda } from './SubtotalesPorMoneda';

interface Props {
  monedas: Moneda[];
  lista: DatosLista;
  cargando: boolean;
  errorCarga: string | null;
  mostrarFormulario: boolean;
  errores: ErrorCampo[];
  onAbrirFormulario: () => void;
  onCerrarFormulario: () => void;
  onCrearCuenta: (datos: NuevaCuenta) => Promise<void>;
  onSeleccionarCuenta: (id: string) => void;
}

export const MisCuentas: React.FC<Props> = ({
  monedas,
  lista,
  cargando,
  errorCarga,
  mostrarFormulario,
  errores,
  onAbrirFormulario,
  onCerrarFormulario,
  onCrearCuenta,
  onSeleccionarCuenta,
}) => {
  const total = lista.cuentas.length;
  const listo = !cargando && !errorCarga;

  return (
    <div className="grid items-start gap-6 md:grid-cols-[17.5rem_1fr]">
      {/* Saldo por moneda */}
      <aside className="min-w-0">
        {cargando && (
          <div className="space-y-4" aria-hidden="true">
            <div className="h-40 animate-pulse rounded-3xl bg-superficie ring-1 ring-borde" />
            <div className="hidden h-40 animate-pulse rounded-3xl bg-superficie ring-1 ring-borde md:block" />
          </div>
        )}
        {listo && total > 0 && (
          <SubtotalesPorMoneda subtotales={lista.subtotales} monedas={monedas} cuentas={lista.cuentas} />
        )}
        {listo && total === 0 && (
          <div className="rounded-3xl border border-dashed border-borde bg-superficie p-5 text-sm text-suave shadow-suave">
            Aquí verás el saldo de cada moneda en cuanto crees tu primera cuenta.
          </div>
        )}
      </aside>

      {/* Panel de cuentas */}
      <section
        aria-labelledby="titulo-panel"
        className="min-w-0 overflow-hidden rounded-3xl border border-borde bg-superficie shadow-suave"
      >
        <div className="flex items-center justify-between gap-4 border-b border-borde px-6 py-4">
          <div>
            <h2 id="titulo-panel" className="text-lg font-semibold tracking-tight text-texto">
              {mostrarFormulario ? 'Nueva cuenta' : 'Cuentas'}
            </h2>
            {!mostrarFormulario && listo && (
              <p className="text-sm tabular-nums text-suave">
                {total} {total === 1 ? 'cuenta' : 'cuentas'}
              </p>
            )}
          </div>
          {!mostrarFormulario && listo && (
            <button
              onClick={onAbrirFormulario}
              className="inline-flex h-10 items-center gap-2 rounded-xl bg-acento px-4 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento sm:text-sm"
            >
              <IconoMas width={16} height={16} />
              Nueva cuenta
            </button>
          )}
        </div>

        {cargando && (
          <div className="space-y-px" aria-busy="true">
            <p className="sr-only">Cargando tus cuentas…</p>
            {[0, 1, 2].map((i) => (
              <div key={i} className="flex items-center gap-4 px-6 py-4" aria-hidden="true">
                <div className="size-11 animate-pulse rounded-2xl bg-superficie-2" />
                <div className="flex-1 space-y-2">
                  <div className="h-3.5 w-1/3 animate-pulse rounded bg-superficie-2" />
                  <div className="h-3 w-1/5 animate-pulse rounded bg-superficie-2" />
                </div>
                <div className="h-4 w-24 animate-pulse rounded bg-superficie-2" />
              </div>
            ))}
          </div>
        )}

        {errorCarga && (
          <p
            role="alert"
            className="m-6 rounded-2xl border border-peligro-borde bg-peligro-suave p-4 text-sm text-peligro"
          >
            {errorCarga}
          </p>
        )}

        {listo && mostrarFormulario && (
          <FormularioCuenta
            monedas={monedas}
            onCrear={onCrearCuenta}
            onCancelar={onCerrarFormulario}
            errores={errores}
          />
        )}

        {listo &&
          !mostrarFormulario &&
          (total === 0 ? (
            <EstadoVacio onCrearCuenta={onAbrirFormulario} />
          ) : (
            <ListaCuentas
              cuentas={lista.cuentas}
              monedas={monedas}
              onSeleccionarCuenta={onSeleccionarCuenta}
            />
          ))}
      </section>
    </div>
  );
};
export default MisCuentas;