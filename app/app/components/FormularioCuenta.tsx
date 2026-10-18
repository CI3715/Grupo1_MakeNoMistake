import React, { useId, useState } from 'react';
import { Moneda, NuevaCuenta, TipoCuenta, ErrorCampo } from '../types/cuentas';
import { normalizarSaldo } from '../utils/formato';
import { TIPOS } from '../utils/tipos';
import { IconoTipo } from './Iconos';

interface Props {
  monedas: Moneda[];
  onCrear: (datos: NuevaCuenta) => Promise<void>;
  onCancelar: () => void;
  errores: ErrorCampo[];
}

const CAMPO = 'w-full rounded-xl border bg-superficie px-3.5 py-2.5 text-base outline-none transition placeholder:text-suave/60';
const CAMPO_OK = 'border-borde focus:border-acento focus:ring-4 focus:ring-acento/15';
const CAMPO_ERROR = 'border-peligro focus:ring-4 focus:ring-peligro/15';

const CAJA_SALDO = 'flex items-center gap-2 rounded-xl border bg-superficie px-3.5 py-2.5 transition';
const CAJA_SALDO_OK = 'border-borde focus-within:border-acento focus-within:ring-4 focus-within:ring-acento/15';
const CAJA_SALDO_ERROR = 'border-peligro focus-within:ring-4 focus-within:ring-peligro/15';

/** Aspecto de una opción (tipo o moneda) mostrada como botón de radio. */
const OPCION =
  'flex cursor-pointer flex-col items-center gap-1 rounded-xl border border-borde bg-superficie px-2 py-3 text-sm text-suave transition hover:border-suave/40 peer-checked:border-acento peer-checked:bg-acento-suave peer-checked:text-acento-texto peer-focus-visible:ring-4 peer-focus-visible:ring-acento/25';

/**
 * Formulario "Nueva cuenta". Se dibuja plano porque va dentro del panel de MisCuentas,
 * que ya pone el título, el borde y la sombra.
 */
export const FormularioCuenta: React.FC<Props> = ({ monedas, onCrear, onCancelar, errores }) => {
  const id = useId();
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoCuenta>('efectivo');
  const [moneda, setMoneda] = useState(monedas[0]?.codigo || 'VES');
  const [saldoInicial, setSaldoInicial] = useState('0.00');
  const [enviando, setEnviando] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setEnviando(true);
    try {
      await onCrear({
        nombre,
        tipo,
        moneda,
        saldo_inicial: normalizarSaldo(saldoInicial),
      });
    } finally {
      setEnviando(false);
    }
  };

  const getError = (campo: string) => {
    const mensajes = errores.filter((e) => e.campo === campo).map((e) => e.mensaje);
    return mensajes.length > 0 ? mensajes.join(' ') : null;
  };

  const erroresGenerales = errores.filter((e) => e.campo === null);
  const errorNombre = getError('nombre');
  const errorTipo = getError('tipo');
  const errorMoneda = getError('moneda');
  const errorSaldo = getError('saldo_inicial');
  const simbolo = monedas.find((m) => m.codigo === moneda)?.simbolo ?? moneda;

  return (
    <form onSubmit={handleSubmit} className="px-6 py-6">
      {erroresGenerales.length > 0 && (
        <div
          role="alert"
          className="mb-5 rounded-xl border border-peligro-borde bg-peligro-suave p-3 text-sm text-peligro"
        >
          {erroresGenerales.map((e, i) => (
            <p key={i}>{e.mensaje}</p>
          ))}
        </div>
      )}

      {/* Nombre */}
      <div className="mb-5">
        <label htmlFor={`${id}-nombre`} className="mb-1.5 block text-sm font-medium">
          Nombre
        </label>
        <input
          id={`${id}-nombre`}
          type="text"
          autoFocus
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          aria-invalid={errorNombre ? true : undefined}
          aria-describedby={errorNombre ? `${id}-nombre-error` : undefined}
          className={`${CAMPO} ${errorNombre ? CAMPO_ERROR : CAMPO_OK}`}
          placeholder="Ej. Cartera, Cuenta nómina"
        />
        {errorNombre && (
          <p id={`${id}-nombre-error`} className="mt-1.5 text-sm text-peligro">
            {errorNombre}
          </p>
        )}
      </div>

      {/* Tipo */}
      <fieldset className="mb-5">
        <legend className="mb-1.5 text-sm font-medium">Tipo de cuenta</legend>
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
                {t.nombre}
              </span>
            </label>
          ))}
        </div>
        {errorTipo && <p className="mt-1.5 text-sm text-peligro">{errorTipo}</p>}
      </fieldset>

      {/* Moneda */}
      <fieldset className="mb-5">
        <legend className="mb-1.5 text-sm font-medium">Moneda</legend>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {monedas.map((m) => (
            <label key={m.codigo} className="relative" title={m.nombre}>
              <input
                type="radio"
                name={`${id}-moneda`}
                value={m.codigo}
                checked={moneda === m.codigo}
                onChange={() => setMoneda(m.codigo)}
                className="peer sr-only"
              />
              <span className={OPCION}>
                <span className="font-semibold">{m.codigo}</span>
                <span className="text-xs opacity-80">{m.simbolo}</span>
              </span>
            </label>
          ))}
        </div>
        {errorMoneda && <p className="mt-1.5 text-sm text-peligro">{errorMoneda}</p>}
      </fieldset>

      {/* Saldo inicial */}
      <div className="mb-6">
        <label htmlFor={`${id}-saldo`} className="mb-1.5 block text-sm font-medium">
          Saldo inicial
        </label>
        <div className={`${CAJA_SALDO} ${errorSaldo ? CAJA_SALDO_ERROR : CAJA_SALDO_OK}`}>
          <span className="text-suave" aria-hidden="true">
            {simbolo}
          </span>
          <input
            id={`${id}-saldo`}
            type="text"
            inputMode="decimal"
            value={saldoInicial}
            onChange={(e) => setSaldoInicial(e.target.value)}
            aria-invalid={errorSaldo ? true : undefined}
            aria-describedby={`${id}-saldo-ayuda${errorSaldo ? ` ${id}-saldo-error` : ''}`}
            className="w-full bg-transparent text-base tabular-nums outline-none placeholder:text-suave/60"
            placeholder="0,00"
          />
        </div>
        <p id={`${id}-saldo-ayuda`} className="mt-1.5 text-xs text-suave">
          Puedes usar coma o punto para los decimales.
        </p>
        {errorSaldo && (
          <p id={`${id}-saldo-error`} className="mt-1 text-sm text-peligro">
            {errorSaldo}
          </p>
        )}
      </div>

      <div className="flex justify-end gap-3 border-t border-borde pt-5">
        <button
          type="button"
          onClick={onCancelar}
          className="h-10 rounded-xl border border-borde px-4 text-sm text-suave transition hover:bg-superficie-2 hover:text-texto focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento"
        >
          Cancelar
        </button>
        <button
          type="submit"
          disabled={enviando}
          className="h-10 rounded-xl bg-acento px-5 text-sm font-medium text-acento-fg transition hover:brightness-110 active:scale-[0.98] disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento"
        >
          {enviando ? 'Guardando…' : 'Guardar cuenta'}
        </button>
      </div>
    </form>
  );
};
