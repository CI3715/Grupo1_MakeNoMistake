'use client';

import React, { useId, useState } from 'react';
import { ErrorCampo, MonedaDto, NuevaCuentaPayload } from '../utils/tauriCommands';
import { normalizarDecimal } from '../utils/formato';
import { TIPOS } from '../utils/tipos';
import { IconoTipo } from './Iconos';
import { MenuDesplegable } from './MenuDesplegable';

const CAMPOS_DEL_FORMULARIO = ['nombre', 'tipo', 'moneda', 'saldo_inicial'];

interface FormularioCuentaProps {
  monedas: MonedaDto[];
  onCrear: (datos: NuevaCuentaPayload) => void | Promise<void>;
  onCancelar: () => void;
  errores?: ErrorCampo[];
  modoOscuro?: boolean;
}

const CAMPO =
  'w-full rounded-xl border bg-superficie px-3.5 py-2.5 text-sm text-texto outline-none transition placeholder:text-suave/60';
const CAMPO_OK = 'border-borde focus:border-acento focus:ring-4 focus:ring-acento/15';
const CAMPO_ERROR = 'border-peligro focus:ring-4 focus:ring-peligro/15';

const CAJA_SALDO =
  'flex items-center gap-2 rounded-xl border bg-superficie px-3.5 py-2.5 transition';
const CAJA_SALDO_OK =
  'border-borde focus-within:border-acento focus-within:ring-4 focus-within:ring-acento/15';
const CAJA_SALDO_ERROR = 'border-peligro focus-within:ring-4 focus-within:ring-peligro/15';

const OPCION =
  'flex cursor-pointer flex-col items-center gap-1.5 rounded-xl border border-borde bg-superficie px-2 py-2.5 text-xs text-suave transition hover:border-suave/40 peer-checked:border-acento peer-checked:bg-acento-suave peer-checked:text-acento-texto peer-focus-visible:ring-4 peer-focus-visible:ring-acento/25';

export const FormularioCuenta: React.FC<FormularioCuentaProps> = ({
  monedas,
  onCrear,
  onCancelar,
  errores = [],
}) => {
  const id = useId();
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState('efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');

  const errorMap = errores.reduce(
    (acc, err) => {
      if (err.campo) acc[err.campo] = err.mensaje;
      return acc;
    },
    {} as Record<string, string>
  );

  // Errores sin campo (o de un campo que este formulario no muestra): se ven arriba.
  const erroresGenerales = errores.filter(
    (err) => !err.campo || !CAMPOS_DEL_FORMULARIO.includes(err.campo)
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // El saldo viaja como texto: la validación (negativo, decimales de más) es del dominio.
    void onCrear({ nombre, tipo, moneda, saldo_inicial: normalizarDecimal(saldoInicial) });
  };

  const simbolo = monedas.find((m) => m.codigo === moneda)?.simbolo ?? moneda;

  return (
    <div className="w-full text-texto">
      <h3 className="mb-4 text-lg font-bold tracking-tight text-texto">Nueva Cuenta</h3>

      {erroresGenerales.length > 0 && (
        <div
          role="alert"
          className="mb-4 space-y-1 rounded-xl border border-peligro-borde bg-peligro-suave p-3 text-xs text-peligro"
        >
          {erroresGenerales.map((err, i) => (
            <p key={i}>{err.mensaje}</p>
          ))}
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label
            htmlFor={`${id}-nombre`}
            className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
          >
            Nombre de la cuenta
          </label>
          <input
            id={`${id}-nombre`}
            type="text"
            autoFocus
            placeholder="Ej. Cartera, Cuenta Banesco"
            value={nombre}
            onChange={(e) => setNombre(e.target.value)}
            className={`${CAMPO} ${errorMap.nombre ? CAMPO_ERROR : CAMPO_OK}`}
          />
          {errorMap.nombre && (
            <p className="mt-1.5 text-xs font-medium text-peligro">{errorMap.nombre}</p>
          )}
        </div>

        <fieldset>
          <legend className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-suave">
            Tipo de cuenta
          </legend>
          <div className="grid grid-cols-3 gap-2">
            {TIPOS.map((t) => (
              <label key={t.valor} className="relative">
                <input
                  type="radio"
                  name={`${id}-tipo`}
                  value={t.valor}
                  checked={tipo === t.valor}
                  onChange={() => setTipo(t.valor)}
                  className="peer sr-only"
                />
                <span className={OPCION}>
                  <IconoTipo tipo={t.valor} />
                  <span className="font-medium">{t.nombre}</span>
                </span>
              </label>
            ))}
          </div>
          {errorMap.tipo && (
            <p className="mt-1.5 text-xs font-medium text-peligro">{errorMap.tipo}</p>
          )}
        </fieldset>

        <div>
          <label
            htmlFor={`${id}-moneda`}
            className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
          >
            Moneda
          </label>
          <MenuDesplegable
            id={`${id}-moneda`}
            valor={moneda}
            alCambiar={setMoneda}
            opciones={monedas.map((m) => ({
              valor: m.codigo,
              etiqueta: `${m.nombre}`,
              secundario: m.simbolo,
            }))}
          />
          {errorMap.moneda && (
            <p className="mt-1.5 text-xs font-medium text-peligro">{errorMap.moneda}</p>
          )}
        </div>

        <div>
          <label
            htmlFor={`${id}-saldo`}
            className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
          >
            Saldo inicial
          </label>
          <div className={`${CAJA_SALDO} ${errorMap.saldo_inicial ? CAJA_SALDO_ERROR : CAJA_SALDO_OK}`}>
            <span className="font-semibold text-suave" aria-hidden="true">
              {simbolo}
            </span>
            <input
              id={`${id}-saldo`}
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={saldoInicial}
              onChange={(e) => setSaldoInicial(e.target.value)}
              className="w-full bg-transparent text-sm font-medium tabular-nums text-texto outline-none placeholder:text-suave/60"
            />
          </div>
          {errorMap.saldo_inicial && (
            <p className="mt-1.5 text-xs font-medium text-peligro">{errorMap.saldo_inicial}</p>
          )}
        </div>

        <div className="flex justify-end gap-3 border-t border-borde pt-4">
          <button
            type="button"
            onClick={onCancelar}
            className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            Cancelar
          </button>
          <button
            type="submit"
            className="cursor-pointer rounded-xl bg-acento px-5 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
          >
            Guardar Cuenta
          </button>
        </div>
      </form>
    </div>
  );
};
