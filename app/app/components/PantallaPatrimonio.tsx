'use client';

import React, { useState, useEffect } from 'react';
import {
  listarMonedasCommand,
  consultarPatrimonioCommand,
  cambiarMonedaBaseCommand,
  crearMonedaCommand,
  extraerErrores,
  mensajeDeError,
  MonedaDto,
  PatrimonioDto,
} from '../utils/tauriCommands';
import { fechaHoyLocal, formatearMontoTexto, formatearFechaDDMMYYYY } from '../utils/formato';

interface PantallaPatrimonioProps {
  modoOscuro?: boolean;
}

interface ResultadoPatrimonio {
  /** Clave (fecha + número de recarga) a la que corresponde este resultado. */
  clave: string;
  datos: PatrimonioDto | null;
  error: string | null;
}

const MONEDA_VACIA = { codigo: '', nombre: '', simbolo: '', decimales: '2' };

export const PantallaPatrimonio: React.FC<PantallaPatrimonioProps> = ({ modoOscuro = true }) => {
  const [fechaConsulta, setFechaConsulta] = useState<string>(fechaHoyLocal);
  const [recarga, setRecarga] = useState<number>(0);
  const [monedas, setMonedas] = useState<MonedaDto[]>([]);
  const [resultado, setResultado] = useState<ResultadoPatrimonio | null>(null);
  const [errorBase, setErrorBase] = useState<string | null>(null);

  const [mostrarCrearMoneda, setMostrarCrearMoneda] = useState<boolean>(false);
  const [nuevaMoneda, setNuevaMoneda] = useState(MONEDA_VACIA);
  const [erroresFormMoneda, setErroresFormMoneda] = useState<Record<string, string>>({});
  const [mensajeExitoMoneda, setMensajeExitoMoneda] = useState<string | null>(null);

  // Monedas del catálogo (de fábrica + propias).
  useEffect(() => {
    let vigente = true;
    listarMonedasCommand()
      .then((lista) => {
        if (vigente) setMonedas(lista);
      })
      .catch((err) => console.error('Error al cargar monedas:', err));
    return () => {
      vigente = false;
    };
  }, [recarga]);

  // Patrimonio a la fecha de consulta. Se vuelve a pedir al cambiar la fecha o al
  // recargar (cambio de moneda base, moneda nueva).
  useEffect(() => {
    let vigente = true;
    const clave = `${fechaConsulta}|${recarga}`;
    consultarPatrimonioCommand(fechaConsulta)
      .then((datos) => {
        if (vigente) setResultado({ clave, datos, error: null });
      })
      .catch((err) => {
        if (vigente) {
          setResultado({
            clave,
            datos: null,
            error: mensajeDeError(err, 'Error al consultar el patrimonio.'),
          });
        }
      });
    return () => {
      vigente = false;
    };
  }, [fechaConsulta, recarga]);

  const cargando = resultado === null || resultado.clave !== `${fechaConsulta}|${recarga}`;
  const datosPatrimonio = resultado?.datos ?? null;
  const errorBackend = resultado?.error ?? null;
  const monedaBase = datosPatrimonio?.moneda_base ?? '';

  const handleCambiarMonedaBase = async (nuevaBase: string) => {
    setErrorBase(null);
    try {
      await cambiarMonedaBaseCommand(nuevaBase);
      setRecarga((r) => r + 1);
    } catch (err) {
      setErrorBase(mensajeDeError(err, 'No se pudo cambiar la moneda base.'));
    }
  };

  const handleCrearMoneda = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresFormMoneda({});

    try {
      await crearMonedaCommand(nuevaMoneda);
      setMostrarCrearMoneda(false);
      setNuevaMoneda(MONEDA_VACIA);
      setMensajeExitoMoneda('Moneda creada exitosamente.');
      setRecarga((r) => r + 1);
      setTimeout(() => setMensajeExitoMoneda(null), 4000);
    } catch (err) {
      const mapa: Record<string, string> = {};
      extraerErrores(err, 'No se pudo crear la moneda.').forEach((item) => {
        mapa[item.campo ?? 'general'] = item.mensaje;
      });
      setErroresFormMoneda(mapa);
    }
  };

  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800 text-white' : 'bg-white border-gray-200 text-slate-900';

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#d1d5db', colorScheme: 'light' };

  return (
    <div className="w-full space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-gray-200 dark:border-gray-800 pb-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight">Resumen de Patrimonio</h1>
          <p className="text-xs text-gray-400 mt-0.5">
            Estado de activos y total consolidado según tasas registradas.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <label htmlFor="fecha-consulta" className="text-xs font-semibold text-gray-400">
            Fecha de consulta:
          </label>
          <input
            id="fecha-consulta"
            type="date"
            value={fechaConsulta}
            max={fechaHoyLocal()}
            onChange={(e) => setFechaConsulta(e.target.value)}
            style={estiloInput}
            className="border px-3 py-1.5 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none"
          />
        </div>
      </div>

      {mensajeExitoMoneda && (
        <div className="p-3 bg-emerald-950/70 border border-emerald-800/80 text-emerald-300 rounded-xl text-xs font-semibold">
          ✓ {mensajeExitoMoneda}
        </div>
      )}

      {errorBase && (
        <div className="p-3 bg-rose-950/40 border border-rose-800/60 text-rose-300 rounded-xl text-xs">
          {errorBase}
        </div>
      )}

      <div className={`flex flex-wrap items-center justify-between gap-4 p-4 rounded-2xl border shadow-sm ${bgCard}`}>
        <div className="flex items-center gap-3">
          <label htmlFor="moneda-base" className="text-xs font-semibold text-gray-400">
            Moneda Base:
          </label>
          <select
            id="moneda-base"
            value={monedaBase}
            onChange={(e) => handleCambiarMonedaBase(e.target.value)}
            style={estiloInput}
            className="border px-3 py-1.5 rounded-xl text-xs font-medium focus:ring-2 focus:ring-purple-500 focus:outline-none cursor-pointer"
          >
            {monedas.map((m) => (
              <option key={m.codigo} value={m.codigo}>
                {m.codigo} - {m.nombre}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={() => setMostrarCrearMoneda(true)}
          className="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-xl text-xs font-semibold transition cursor-pointer"
        >
          + Crear moneda propia
        </button>
      </div>

      {mostrarCrearMoneda && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm">
          <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
            <h3 className="text-base font-bold mb-4">Nueva Moneda Personalizada</h3>
            {erroresFormMoneda.general && (
              <p className="mb-3 text-xs text-rose-400 font-semibold">{erroresFormMoneda.general}</p>
            )}

            <form onSubmit={handleCrearMoneda} className="space-y-4">
              <div>
                <label htmlFor="codigo-moneda" className="block text-xs font-semibold text-gray-400 mb-1">
                  Código (3 a 5 letras mayúsculas)
                </label>
                <input
                  id="codigo-moneda"
                  type="text"
                  maxLength={5}
                  value={nuevaMoneda.codigo}
                  onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, codigo: e.target.value.toUpperCase() })}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
                  placeholder="Ej. VES, BTC"
                />
                {erroresFormMoneda.codigo && <span className="text-[11px] text-rose-400 mt-1 block">{erroresFormMoneda.codigo}</span>}
              </div>

              <div>
                <label htmlFor="nombre-moneda" className="block text-xs font-semibold text-gray-400 mb-1">
                  Nombre
                </label>
                <input
                  id="nombre-moneda"
                  type="text"
                  value={nuevaMoneda.nombre}
                  onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, nombre: e.target.value })}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
                  placeholder="Ej. Bolívar Soberano"
                />
                {erroresFormMoneda.nombre && <span className="text-[11px] text-rose-400 mt-1 block">{erroresFormMoneda.nombre}</span>}
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label htmlFor="simbolo-moneda" className="block text-xs font-semibold text-gray-400 mb-1">
                    Símbolo
                  </label>
                  <input
                    id="simbolo-moneda"
                    type="text"
                    value={nuevaMoneda.simbolo}
                    onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, simbolo: e.target.value })}
                    style={estiloInput}
                    className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
                    placeholder="Ej. Bs."
                  />
                  {erroresFormMoneda.simbolo && <span className="text-[11px] text-rose-400 mt-1 block">{erroresFormMoneda.simbolo}</span>}
                </div>
                <div>
                  <label htmlFor="decimales-moneda" className="block text-xs font-semibold text-gray-400 mb-1">
                    Decimales (0 a 8)
                  </label>
                  <input
                    id="decimales-moneda"
                    type="text"
                    inputMode="numeric"
                    value={nuevaMoneda.decimales}
                    onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, decimales: e.target.value })}
                    style={estiloInput}
                    className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
                  />
                  {erroresFormMoneda.decimales && <span className="text-[11px] text-rose-400 mt-1 block">{erroresFormMoneda.decimales}</span>}
                </div>
              </div>

              <div className="flex justify-end gap-3 pt-2">
                <button
                  type="button"
                  onClick={() => setMostrarCrearMoneda(false)}
                  className="px-4 py-2 rounded-xl text-xs font-semibold border border-gray-700 hover:bg-gray-800 text-white cursor-pointer"
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  className="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer"
                >
                  Guardar Moneda
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {cargando ? (
        <div className="p-8 text-center text-xs text-gray-400">Cargando patrimonio...</div>
      ) : errorBackend ? (
        <div className="p-4 bg-rose-950/40 border border-rose-800/60 text-rose-300 rounded-2xl text-xs">
          {errorBackend}
        </div>
      ) : datosPatrimonio ? (
        <div className="space-y-6">
          {datosPatrimonio.total !== null ? (
            <div className="p-6 rounded-2xl bg-gradient-to-r from-purple-700 to-indigo-800 text-white shadow-lg border border-purple-600/30">
              <h2 className="text-xs uppercase tracking-wider text-purple-200 font-bold">
                Patrimonio Total Consolidado ({datosPatrimonio.moneda_base})
              </h2>
              <div className="text-3xl font-extrabold mt-2">
                {formatearMontoTexto(datosPatrimonio.total, datosPatrimonio.moneda_base)}
              </div>

              {datosPatrimonio.tasas_usadas.length > 0 && (
                <div className="mt-4 pt-4 border-t border-purple-500/30 text-xs text-purple-100 space-y-1">
                  <p className="font-semibold mb-1">Tasas aplicadas para la consolidación:</p>
                  {datosPatrimonio.tasas_usadas.map((t) => (
                    <div
                      key={`${t.moneda_origen}-${t.moneda_destino}-${t.fecha}`}
                      className="flex items-center gap-2"
                    >
                      <span>
                        • 1 {t.moneda_origen} = {formatearMontoTexto(t.valor, t.moneda_destino)} (
                        {t.fuente} — {formatearFechaDDMMYYYY(t.fecha)})
                      </span>
                      {t.inversa && (
                        <span className="bg-purple-900/80 text-purple-200 px-1.5 py-0.5 rounded text-[10px] font-bold">
                          Inversa
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className={`p-5 rounded-2xl border ${modoOscuro ? 'bg-amber-950/40 border-amber-800/70 text-amber-200' : 'bg-amber-50 border-amber-300 text-amber-900'}`}>
              <h3 className="font-bold text-sm mb-1">No se puede calcular el Total Consolidado</h3>
              <ul className="list-disc list-inside text-xs space-y-1 font-semibold">
                {datosPatrimonio.faltantes.map((item) => (
                  <li key={`${item.moneda_origen}-${item.moneda_destino}`}>
                    Falta una tasa de {item.moneda_origen} a {item.moneda_destino} con fecha igual o anterior al{' '}
                    {formatearFechaDDMMYYYY(datosPatrimonio.fecha_consulta)}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
            <h3 className="text-sm font-bold mb-3">Subtotales por Moneda</h3>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              {datosPatrimonio.subtotales.length === 0 ? (
                <p className="text-xs text-gray-400 col-span-full">No hay subtotales disponibles.</p>
              ) : (
                datosPatrimonio.subtotales.map((sub) => (
                  <div key={sub.moneda} className="p-3 rounded-xl bg-gray-500/10 border border-gray-500/10">
                    <span className="text-xs text-gray-400 font-bold block uppercase">{sub.moneda}</span>
                    <span className="text-sm font-extrabold">
                      {formatearMontoTexto(sub.total, sub.moneda)}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>

          {datosPatrimonio.cuentas.length > 0 && (
            <div className={`rounded-2xl shadow-sm border overflow-hidden ${bgCard}`}>
              <div className="p-4 border-b border-gray-200 dark:border-gray-800 font-bold text-sm">
                Saldos por Cuenta
              </div>
              <div className="divide-y divide-gray-200 dark:divide-gray-800">
                {datosPatrimonio.cuentas.map((cta) => (
                  <div key={cta.id} className="p-4 flex justify-between items-center">
                    <div>
                      <span className="font-bold text-sm">{cta.nombre}</span>
                      <span className="ml-2 text-xs font-mono text-gray-400">({cta.moneda})</span>
                    </div>
                    <div className="text-right">
                      <div className="font-bold text-sm">{formatearMontoTexto(cta.saldo, cta.moneda)}</div>
                      {cta.en_moneda_base !== null && cta.moneda !== datosPatrimonio.moneda_base && (
                        <div className="text-[11px] text-gray-400">
                          ≈ {formatearMontoTexto(cta.en_moneda_base, datosPatrimonio.moneda_base)}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
};