'use client';

import React, { useState, useEffect } from 'react';
import {
  listarMonedasCommand,
  listarTasasCommand,
  registrarTasaCommand,
  eliminarTasaCommand,
  extraerErrores,
  mensajeDeError,
  MonedaDto,
  HistorialParDto,
} from '../utils/tauriCommands';
import { fechaHoyLocal, formatearFechaDDMMYYYY, normalizarDecimal } from '../utils/formato';
import { IconoCheckCirculo, IconoPapelera } from './Iconos';

interface PantallaTasasProps {
  modoOscuro?: boolean;
}

interface CargaTasas {
  /** Número de recarga al que corresponde este resultado. */
  recarga: number;
  monedas: MonedaDto[];
  historiales: HistorialParDto[];
  error: string | null;
}

const CAMPO =
  'w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs font-medium text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15';

export const PantallaTasas: React.FC<PantallaTasasProps> = () => {
  const [recarga, setRecarga] = useState<number>(0);
  const [carga, setCarga] = useState<CargaTasas | null>(null);
  const [mensajeExito, setMensajeExito] = useState<string | null>(null);
  const [errorEliminar, setErrorEliminar] = useState<string | null>(null);

  const [tasaAEliminar, setTasaAEliminar] = useState<string | null>(null);

  const [origen, setOrigen] = useState<string>('USD');
  const [destino, setDestino] = useState<string>('VES');
  const [valor, setValor] = useState<string>('');
  const [fecha, setFecha] = useState<string>(fechaHoyLocal);
  const [fuente, setFuente] = useState<string>('');
  const [erroresForm, setErroresForm] = useState<Record<string, string>>({});

  // Carga de datos: el estado solo se actualiza dentro de los callbacks de la promesa.
  useEffect(() => {
    let vigente = true;
    Promise.all([listarMonedasCommand(), listarTasasCommand()])
      .then(([monedas, historiales]) => {
        if (vigente) setCarga({ recarga, monedas, historiales, error: null });
      })
      .catch((err) => {
        if (vigente) {
          setCarga({
            recarga,
            monedas: [],
            historiales: [],
            error: mensajeDeError(err, 'Error al cargar las tasas de cambio.'),
          });
        }
      });
    return () => {
      vigente = false;
    };
  }, [recarga]);

  const cargando = carga === null || carga.recarga !== recarga;
  const monedas = carga?.monedas ?? [];
  const historiales = carga?.historiales ?? [];
  const errorBackend = carga?.error ?? null;

  const avisarExito = (mensaje: string) => {
    setMensajeExito(mensaje);
    setTimeout(() => setMensajeExito(null), 4000);
  };

  const handleRegistrar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresForm({});
    setMensajeExito(null);

    try {
      // Solo se envía lo escrito: el dominio valida valor, fecha, fuente y el par.
      await registrarTasaCommand({
        moneda_origen: origen,
        moneda_destino: destino,
        valor: normalizarDecimal(valor),
        fecha,
        fuente: fuente.trim(),
      });
      setValor('');
      setFuente('');
      setRecarga((r) => r + 1);
      avisarExito('Tasa registrada exitosamente.');
    } catch (err) {
      const mapa: Record<string, string> = {};
      extraerErrores(err, 'No se pudo registrar la tasa.').forEach((item) => {
        mapa[item.campo ?? 'general'] = item.mensaje;
      });
      setErroresForm(mapa);
    }
  };

  const confirmarEliminacion = async () => {
    if (!tasaAEliminar) return;
    setErrorEliminar(null);
    try {
      await eliminarTasaCommand(tasaAEliminar);
      setTasaAEliminar(null);
      setRecarga((r) => r + 1);
      avisarExito('Tasa eliminada correctamente.');
    } catch (err) {
      setErrorEliminar(mensajeDeError(err, 'No se pudo eliminar la tasa.'));
      setTasaAEliminar(null);
    }
  };

  return (
    <div className="w-full space-y-6">
      {mensajeExito && (
        <div className="flex items-center gap-2 rounded-2xl border border-exito-borde bg-exito-suave p-3.5 text-xs font-semibold text-exito-texto shadow-sm">
          <IconoCheckCirculo className="h-4 w-4 shrink-0 text-exito" />
          <span>{mensajeExito}</span>
        </div>
      )}

      <div className="rounded-3xl border border-borde bg-superficie p-5 text-texto shadow-suave sm:p-6">
        <div className="mb-4 border-b border-borde pb-3">
          <h2 className="text-base font-bold tracking-tight text-texto">Registrar Tasa</h2>
          <p className="mt-0.5 text-xs text-suave">
            Registra equivalencias entre monedas con su fecha y fuente correspondiente.
          </p>
        </div>

        {erroresForm.general && (
          <p className="mb-3 text-xs font-semibold text-peligro">{erroresForm.general}</p>
        )}
        {erroresForm.moneda && (
          <p className="mb-3 text-xs font-semibold text-peligro">{erroresForm.moneda}</p>
        )}

        <form onSubmit={handleRegistrar} className="space-y-4">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div>
              <label
                htmlFor="moneda-origen"
                className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
              >
                Moneda Origen
              </label>
              <select
                id="moneda-origen"
                value={origen}
                onChange={(e) => setOrigen(e.target.value)}
                className={`${CAMPO} cursor-pointer`}
              >
                {monedas.map((m) => (
                  <option key={m.codigo} value={m.codigo} className="bg-superficie text-texto">
                    {m.codigo} - {m.nombre}
                  </option>
                ))}
              </select>
              {erroresForm.moneda_origen && (
                <span className="mt-1 block text-[11px] font-medium text-peligro">
                  {erroresForm.moneda_origen}
                </span>
              )}
            </div>

            <div>
              <label
                htmlFor="moneda-destino"
                className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
              >
                Moneda Destino
              </label>
              <select
                id="moneda-destino"
                value={destino}
                onChange={(e) => setDestino(e.target.value)}
                className={`${CAMPO} cursor-pointer`}
              >
                {monedas.map((m) => (
                  <option key={m.codigo} value={m.codigo} className="bg-superficie text-texto">
                    {m.codigo} - {m.nombre}
                  </option>
                ))}
              </select>
              {erroresForm.moneda_destino && (
                <span className="mt-1 block text-[11px] font-medium text-peligro">
                  {erroresForm.moneda_destino}
                </span>
              )}
            </div>
          </div>

          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div>
              <label
                htmlFor="valor-tasa"
                className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
              >
                Valor (1 Origen = X Destino)
              </label>
              <input
                id="valor-tasa"
                type="text"
                inputMode="decimal"
                placeholder="Ej. 887,00"
                value={valor}
                onChange={(e) => setValor(e.target.value)}
                className={CAMPO}
              />
              {erroresForm.valor && (
                <span className="mt-1 block text-[11px] font-medium text-peligro">
                  {erroresForm.valor}
                </span>
              )}
            </div>

            <div>
              <label
                htmlFor="fecha-tasa"
                className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
              >
                Fecha
              </label>
              <input
                id="fecha-tasa"
                type="date"
                max={fechaHoyLocal()}
                value={fecha}
                onChange={(e) => setFecha(e.target.value)}
                className={CAMPO}
              />
              {erroresForm.fecha && (
                <span className="mt-1 block text-[11px] font-medium text-peligro">
                  {erroresForm.fecha}
                </span>
              )}
            </div>

            <div>
              <label
                htmlFor="fuente-tasa"
                className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
              >
                Fuente
              </label>
              <input
                id="fuente-tasa"
                type="text"
                placeholder="Ej. BCV, Casa de Cambio"
                value={fuente}
                onChange={(e) => setFuente(e.target.value)}
                className={CAMPO}
              />
              {erroresForm.fuente && (
                <span className="mt-1 block text-[11px] font-medium text-peligro">
                  {erroresForm.fuente}
                </span>
              )}
            </div>
          </div>

          <div className="flex justify-end pt-2">
            <button
              type="submit"
              className="cursor-pointer rounded-xl bg-acento px-5 py-2.5 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
            >
              Guardar Tasa
            </button>
          </div>
        </form>
      </div>

      <div className="rounded-3xl border border-borde bg-superficie p-5 text-texto shadow-suave sm:p-6">
        <h2 className="mb-4 text-base font-bold tracking-tight text-texto">
          Historial de Tasas por Par
        </h2>

        {errorEliminar && (
          <p className="pb-3 text-center text-xs font-medium text-peligro">{errorEliminar}</p>
        )}

        {cargando ? (
          <p className="py-6 text-center text-xs text-suave">Cargando tasas de cambio...</p>
        ) : errorBackend ? (
          <p className="py-6 text-center text-xs font-medium text-peligro">{errorBackend}</p>
        ) : historiales.length === 0 ? (
          <p className="py-6 text-center text-xs text-suave">
            No hay tasas de cambio registradas aún.
          </p>
        ) : (
          <div className="space-y-4">
            {historiales.map((h) => (
              <div
                key={`${h.moneda_origen}-${h.moneda_destino}`}
                className="overflow-hidden rounded-2xl border border-borde"
              >
                <div className="bg-acento-suave px-4 py-2.5 text-xs font-bold uppercase tracking-wider text-acento-texto">
                  Par {h.moneda_origen} → {h.moneda_destino}
                </div>
                <div className="divide-y divide-borde">
                  {h.tasas.map((t) => (
                    <div
                      key={t.id}
                      className="flex items-center justify-between gap-2 p-3.5 text-xs transition hover:bg-superficie-2/50"
                    >
                      <div>
                        <span className="text-sm font-extrabold tabular-nums text-texto">
                          {t.valor}
                        </span>
                        <span className="ml-2 text-suave">
                          ({formatearFechaDDMMYYYY(t.fecha)} — {t.fuente})
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={() => setTasaAEliminar(t.id)}
                        className="flex cursor-pointer items-center gap-1 rounded-lg border border-peligro-borde bg-peligro-suave px-2.5 py-1 text-[11px] font-semibold text-peligro transition hover:brightness-110"
                        title="Eliminar tasa"
                      >
                        <IconoPapelera className="h-3 w-3 shrink-0" />
                        <span>Eliminar</span>
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {tasaAEliminar && (
        <div
          role="dialog"
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"
        >
          <div className="w-full max-w-sm rounded-3xl border border-borde bg-superficie p-6 text-texto shadow-2xl">
            <h3 className="mb-2 text-base font-bold text-texto">¿Eliminar Tasa?</h3>
            <p className="mb-5 text-xs text-suave">
              Esta acción eliminará la tasa seleccionada del historial.
            </p>
            <div className="flex justify-end gap-3 border-t border-borde pt-4">
              <button
                type="button"
                onClick={() => setTasaAEliminar(null)}
                className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
              >
                Cancelar
              </button>
              <button
                type="button"
                onClick={confirmarEliminacion}
                className="cursor-pointer rounded-xl bg-peligro px-4 py-2 text-xs font-semibold text-white shadow-sm transition hover:brightness-110"
              >
                Eliminar
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
