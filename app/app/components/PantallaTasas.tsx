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

export const PantallaTasas: React.FC<PantallaTasasProps> = ({ modoOscuro = true }) => {
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

  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800 text-white' : 'bg-white border-gray-200 text-slate-900';

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#d1d5db', colorScheme: 'light' };

  return (
    <div className="w-full space-y-6">
      <div className="border-b border-gray-200 dark:border-gray-800 pb-4">
        <h1 className="text-xl font-bold tracking-tight">Tasas de Cambio</h1>
        <p className="text-xs text-gray-400 mt-1">
          Registra equivalencias entre monedas con su fecha y fuente correspondiente.
        </p>
      </div>

      {mensajeExito && (
        <div className="p-3 bg-emerald-950/70 border border-emerald-800/80 text-emerald-300 rounded-xl text-xs font-semibold">
          ✓ {mensajeExito}
        </div>
      )}

      <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
        <h2 className="text-sm font-bold mb-4">Registrar Tasa</h2>
        {erroresForm.general && (
          <p className="mb-3 text-xs text-rose-400 font-semibold">{erroresForm.general}</p>
        )}
        {erroresForm.moneda && (
          <p className="mb-3 text-xs text-rose-400 font-semibold">{erroresForm.moneda}</p>
        )}

        <form onSubmit={handleRegistrar} className="space-y-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <label htmlFor="moneda-origen" className="block text-xs font-semibold text-gray-400 mb-1">
                Moneda Origen
              </label>
              <select
                id="moneda-origen"
                value={origen}
                onChange={(e) => setOrigen(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none cursor-pointer"
              >
                {monedas.map((m) => (
                  <option key={m.codigo} value={m.codigo}>
                    {m.codigo} - {m.nombre}
                  </option>
                ))}
              </select>
              {erroresForm.moneda_origen && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.moneda_origen}</span>}
            </div>

            <div>
              <label htmlFor="moneda-destino" className="block text-xs font-semibold text-gray-400 mb-1">
                Moneda Destino
              </label>
              <select
                id="moneda-destino"
                value={destino}
                onChange={(e) => setDestino(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none cursor-pointer"
              >
                {monedas.map((m) => (
                  <option key={m.codigo} value={m.codigo}>
                    {m.codigo} - {m.nombre}
                  </option>
                ))}
              </select>
              {erroresForm.moneda_destino && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.moneda_destino}</span>}
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div>
              <label htmlFor="valor-tasa" className="block text-xs font-semibold text-gray-400 mb-1">
                Valor (1 Origen = X Destino)
              </label>
              <input
                id="valor-tasa"
                type="text"
                inputMode="decimal"
                placeholder="Ej. 887,00"
                value={valor}
                onChange={(e) => setValor(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
              />
              {erroresForm.valor && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.valor}</span>}
            </div>

            <div>
              <label htmlFor="fecha-tasa" className="block text-xs font-semibold text-gray-400 mb-1">
                Fecha
              </label>
              <input
                id="fecha-tasa"
                type="date"
                max={fechaHoyLocal()}
                value={fecha}
                onChange={(e) => setFecha(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
              />
              {erroresForm.fecha && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.fecha}</span>}
            </div>

            <div>
              <label htmlFor="fuente-tasa" className="block text-xs font-semibold text-gray-400 mb-1">
                Fuente
              </label>
              <input
                id="fuente-tasa"
                type="text"
                placeholder="Ej. BCV, Casa de Cambio"
                value={fuente}
                onChange={(e) => setFuente(e.target.value)}
                style={estiloInput}
                className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
              />
              {erroresForm.fuente && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.fuente}</span>}
            </div>
          </div>

          <div className="flex justify-end pt-2">
            <button
              type="submit"
              className="bg-purple-600 hover:bg-purple-700 text-white px-5 py-2 rounded-xl text-xs font-semibold transition cursor-pointer"
            >
              Guardar Tasa
            </button>
          </div>
        </form>
      </div>

      <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
        <h2 className="text-sm font-bold mb-4">Historial de Tasas por Par</h2>

        {errorEliminar && <p className="text-xs text-rose-400 text-center pb-3">{errorEliminar}</p>}

        {cargando ? (
          <p className="text-xs text-gray-400 text-center py-6">Cargando tasas de cambio...</p>
        ) : errorBackend ? (
          <p className="text-xs text-rose-400 text-center py-6">{errorBackend}</p>
        ) : historiales.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">No hay tasas de cambio registradas aún.</p>
        ) : (
          <div className="space-y-4">
            {historiales.map((h) => (
              <div key={`${h.moneda_origen}-${h.moneda_destino}`} className="border border-gray-700/60 rounded-xl overflow-hidden">
                <div className="bg-gray-500/10 px-4 py-2 font-bold text-xs uppercase tracking-wider text-purple-400">
                  Par {h.moneda_origen} → {h.moneda_destino}
                </div>
                <div className="divide-y divide-gray-800">
                  {h.tasas.map((t) => (
                    <div key={t.id} className="p-3 flex items-center justify-between text-xs gap-2">
                      <div>
                        <span className="font-extrabold text-sm">{t.valor}</span>
                        <span className="text-gray-400 ml-2">
                          ({formatearFechaDDMMYYYY(t.fecha)} — {t.fuente})
                        </span>
                      </div>
                      <button
                        onClick={() => setTasaAEliminar(t.id)}
                        className="text-rose-400 hover:text-rose-300 font-semibold text-[11px] px-2.5 py-1 bg-rose-950/40 hover:bg-rose-900/60 rounded-lg transition cursor-pointer"
                        title="Eliminar tasa"
                      >
                        Eliminar
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
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className={`w-full max-w-sm p-6 rounded-2xl border shadow-xl ${bgCard}`}>
            <h3 className="text-sm font-bold mb-2">¿Eliminar Tasa?</h3>
            <p className="text-xs text-gray-400 mb-5">
              Esta acción eliminará la tasa seleccionada del historial.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setTasaAEliminar(null)}
                className="px-4 py-2 rounded-xl text-xs font-semibold border border-gray-700 hover:bg-gray-800 text-gray-300 cursor-pointer"
              >
                Cancelar
              </button>
              <button
                onClick={confirmarEliminacion}
                className="bg-rose-600 hover:bg-rose-700 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer"
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