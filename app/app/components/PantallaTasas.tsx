'use client';

import React, { useState, useEffect } from 'react';
import {
  listarMonedasCommand,
  listarTasasCommand,
  registrarTasaCommand,
  eliminarTasaCommand,
  MonedaDto,
  HistorialParDto,
} from '../utils/tauriCommands';
import { formatearFechaDDMMYYYY } from '../utils/formato';

interface PantallaTasasProps {
  modoOscuro?: boolean;
}

const obtenerFechaHoyLocal = (): string => {
  const hoy = new Date();
  const a = hoy.getFullYear();
  const m = String(hoy.getMonth() + 1).padStart(2, '0');
  const d = String(hoy.getDate()).padStart(2, '0');
  return `${a}-${m}-${d}`;
};

export const PantallaTasas: React.FC<PantallaTasasProps> = ({ modoOscuro = true }) => {
  const [monedas, setMonedas] = useState<MonedaDto[]>([]);
  const [historiales, setHistoriales] = useState<HistorialParDto[]>([]);
  const [cargando, setCargando] = useState<boolean>(true);
  const [errorBackend, setErrorBackend] = useState<string | null>(null);
  const [mensajeExito, setMensajeExito] = useState<string | null>(null);

  const [tasaAEliminar, setTasaAEliminar] = useState<string | null>(null);

  const [origen, setOrigen] = useState<string>('USD');
  const [destino, setDestino] = useState<string>('VES');
  const [valor, setValor] = useState<string>('');
  const [fecha, setFecha] = useState<string>(obtenerFechaHoyLocal);
  const [fuente, setFuente] = useState<string>('');
  const [erroresForm, setErroresForm] = useState<Record<string, string>>({});

  const cargarDatos = async () => {
    setCargando(true);
    setErrorBackend(null);
    try {
      const [resMonedas, resTasas] = await Promise.all([
        listarMonedasCommand(),
        listarTasasCommand(),
      ]);

      setMonedas(Array.isArray(resMonedas) ? resMonedas : []);

      const historialesArray = Array.isArray(resTasas)
        ? resTasas
        : (resTasas as any)?.tasas || (resTasas as any)?.historiales || [];

      setHistoriales(Array.isArray(historialesArray) ? historialesArray : []);
    } catch (err: any) {
      setErrorBackend(typeof err === 'string' ? err : 'Error al cargar las tasas de cambio.');
      setHistoriales([]);
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => {
    cargarDatos();
  }, []);

  const handleRegistrar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresForm({});
    setMensajeExito(null);

    const valNum = parseFloat(valor);
    const nuevosErrores: Record<string, string> = {};

    if (!valor || isNaN(valNum) || valNum <= 0) {
      nuevosErrores.valor = 'El valor debe ser mayor a 0.';
    }
    if (!fecha) nuevosErrores.fecha = 'La fecha es obligatoria.';
    if (!fuente.trim()) nuevosErrores.fuente = 'La fuente es obligatoria.';
    if (origen === destino) nuevosErrores.destino = 'Las monedas deben ser distintas.';

    if (Object.keys(nuevosErrores).length > 0) {
      setErroresForm(nuevosErrores);
      return;
    }

    try {
      await registrarTasaCommand({
        moneda_origen: origen,
        moneda_destino: destino,
        valor: valNum,
        fecha,
        fuente: fuente.trim(),
      });
      setValor('');
      setFuente('');
      setMensajeExito('Tasa registrada exitosamente.');
      await cargarDatos();
      setTimeout(() => setMensajeExito(null), 4000);
    } catch (err: any) {
      if (typeof err === 'object' && err !== null && err.errores) {
        const mapa: Record<string, string> = {};
        err.errores.forEach((item: any) => {
          mapa[item.campo || 'general'] = item.mensaje;
        });
        setErroresForm(mapa);
      } else {
        setErroresForm({ general: String(err) });
      }
    }
  };

  const confirmarEliminacion = async () => {
    if (!tasaAEliminar) return;
    try {
      await eliminarTasaCommand(tasaAEliminar);
      setMensajeExito('Tasa eliminada correctamente.');
      setTasaAEliminar(null);
      await cargarDatos();
      setTimeout(() => setMensajeExito(null), 4000);
    } catch (err) {
      setErrorBackend('Error al eliminar la tasa: ' + String(err));
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
              {erroresForm.destino && <span className="text-[11px] text-rose-400 mt-1 block">{erroresForm.destino}</span>}
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div>
              <label htmlFor="valor-tasa" className="block text-xs font-semibold text-gray-400 mb-1">
                Valor (1 Origen = X Destino)
              </label>
              <input
                id="valor-tasa"
                type="number"
                step="any"
                placeholder="Ej. 887.00"
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
                max={obtenerFechaHoyLocal()}
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

        {cargando ? (
          <p className="text-xs text-gray-400 text-center py-6">Cargando tasas de cambio...</p>
        ) : errorBackend ? (
          <p className="text-xs text-rose-400 text-center py-6">{errorBackend}</p>
        ) : !Array.isArray(historiales) || historiales.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">No hay tasas de cambio registradas aún.</p>
        ) : (
          <div className="space-y-4">
            {historiales.map((h) => (
              <div key={h.par || Math.random().toString()} className="border border-gray-700/60 rounded-xl overflow-hidden">
                <div className="bg-gray-500/10 px-4 py-2 font-bold text-xs uppercase tracking-wider text-purple-400">
                  Par {h.par || `${(h as any).moneda_origen}/${(h as any).moneda_destino}`}
                </div>
                <div className="divide-y divide-gray-800">
                  {Array.isArray(h.tasas) && h.tasas.map((t) => (
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