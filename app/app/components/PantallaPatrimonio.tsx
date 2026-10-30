'use client';

import React, { useState, useEffect } from 'react';
import {
  listarMonedasCommand,
  consultarPatrimonioCommand,
  cambiarMonedaBaseCommand,
  crearMonedaCommand,
  MonedaDto,
  RespuestaPatrimonioDto,
} from '../utils/tauriCommands';
import { formatearMontoTexto, formatearFechaDDMMYYYY } from '../utils/formato';

interface PantallaPatrimonioProps {
  modoOscuro?: boolean;
}

const obtenerFechaHoyLocal = (): string => {
  const hoy = new Date();
  const a = hoy.getFullYear();
  const m = String(hoy.getMonth() + 1).padStart(2, '0');
  const d = String(hoy.getDate()).padStart(2, '0');
  return `${a}-${m}-${d}`;
};

export const PantallaPatrimonio: React.FC<PantallaPatrimonioProps> = ({ modoOscuro = true }) => {
  const [fechaConsulta, setFechaConsulta] = useState<string>(obtenerFechaHoyLocal);
  const [monedaBase, setMonedaBase] = useState<string>('USD');
  const [monedas, setMonedas] = useState<MonedaDto[]>([]);
  const [datosPatrimonio, setDatosPatrimonio] = useState<RespuestaPatrimonioDto | null>(null);
  const [cargando, setCargando] = useState<boolean>(true);
  const [errorBackend, setErrorBackend] = useState<string | null>(null);

  const [mostrarCrearMoneda, setMostrarCrearMoneda] = useState<boolean>(false);
  const [nuevaMoneda, setNuevaMoneda] = useState({
    codigo: '',
    nombre: '',
    simbolo: '',
    decimales: 2,
  });
  const [erroresFormMoneda, setErroresFormMoneda] = useState<Record<string, string>>({});
  const [mensajeExitoMoneda, setMensajeExitoMoneda] = useState<string | null>(null);

  const cargarMonedas = async () => {
    try {
      const res = await listarMonedasCommand();
      setMonedas(res || []);
    } catch (err) {
      console.error('Error al cargar monedas:', String(err));
    }
  };

  const consultarPatrimonio = async () => {
    setCargando(true);
    setErrorBackend(null);
    try {
      const fechaLimpia = fechaConsulta.trim().split('T')[0];
      const res = await consultarPatrimonioCommand(fechaLimpia);
      setDatosPatrimonio(res);
      if (res && res.moneda_base) {
        setMonedaBase(res.moneda_base);
      }
    } catch (err: any) {
      const errStr = typeof err === 'object' ? JSON.stringify(err) : String(err);
      if (typeof err === 'object' && err !== null && err.errores) {
        setErrorBackend(err.errores.map((e: any) => e.mensaje).join(', '));
      } else {
        setErrorBackend(errStr || 'Error al consultar el patrimonio.');
      }
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => {
    cargarMonedas();
  }, []);

  useEffect(() => {
    consultarPatrimonio();
  }, [fechaConsulta]);

  const handleCambiarMonedaBase = async (nuevaBase: string) => {
    try {
      await cambiarMonedaBaseCommand(nuevaBase);
      setMonedaBase(nuevaBase);
      await consultarPatrimonio();
    } catch (err) {
      console.error('Error al cambiar moneda base:', String(err));
    }
  };

  const handleCrearMoneda = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresFormMoneda({});

    try {
      await crearMonedaCommand(nuevaMoneda);
      setMostrarCrearMoneda(false);
      setNuevaMoneda({ codigo: '', nombre: '', simbolo: '', decimales: 2 });
      setMensajeExitoMoneda('Moneda creada exitosamente.');
      await cargarMonedas();
      await consultarPatrimonio();
      setTimeout(() => setMensajeExitoMoneda(null), 4000);
    } catch (err: any) {
      if (typeof err === 'object' && err !== null && err.errores) {
        const mapa: Record<string, string> = {};
        err.errores.forEach((item: any) => {
          mapa[item.campo || 'general'] = item.mensaje;
        });
        setErroresFormMoneda(mapa);
      } else {
        setErroresFormMoneda({ general: String(err) });
      }
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
            max={obtenerFechaHoyLocal()}
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
                </div>
                <div>
                  <label htmlFor="decimales-moneda" className="block text-xs font-semibold text-gray-400 mb-1">
                    Decimales (0 a 8)
                  </label>
                  <input
                    id="decimales-moneda"
                    type="number"
                    min={0}
                    max={8}
                    value={nuevaMoneda.decimales}
                    onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, decimales: parseInt(e.target.value) || 0 })}
                    style={estiloInput}
                    className="w-full border px-3 py-2 rounded-xl text-xs focus:ring-2 focus:ring-purple-500 focus:outline-none"
                  />
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
          {datosPatrimonio.total !== null && datosPatrimonio.total !== undefined ? (
            <div className="p-6 rounded-2xl bg-gradient-to-r from-purple-700 to-indigo-800 text-white shadow-lg border border-purple-600/30">
              <h2 className="text-xs uppercase tracking-wider text-purple-200 font-bold">
                Patrimonio Total Consolidado ({datosPatrimonio.moneda_base})
              </h2>
              <div className="text-3xl font-extrabold mt-2">
                {formatearMontoTexto(datosPatrimonio.total, datosPatrimonio.moneda_base)}
              </div>

              {datosPatrimonio.tasas_usadas && datosPatrimonio.tasas_usadas.length > 0 && (
                <div className="mt-4 pt-4 border-t border-purple-500/30 text-xs text-purple-100 space-y-1">
                  <p className="font-semibold mb-1">Tasas aplicadas para la consolidación:</p>
                  {datosPatrimonio.tasas_usadas.map((t, idx) => {
                    const parLabel = t.par || (t.moneda_origen && t.moneda_destino ? `${t.moneda_origen} → ${t.moneda_destino}` : 'Tasa');
                    const esInversa = Boolean(t.inversa ?? t.es_inversa);
                    return (
                      <div key={idx} className="flex items-center gap-2">
                        <span>
                          • {parLabel}: {t.valor} ({t.fuente} — {formatearFechaDDMMYYYY(t.fecha)})
                        </span>
                        {esInversa && (
                          <span className="bg-purple-900/80 text-purple-200 px-1.5 py-0.5 rounded text-[10px] font-bold">
                            Inversa
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          ) : (
            <div className={`p-5 rounded-2xl border ${modoOscuro ? 'bg-amber-950/40 border-amber-800/70 text-amber-200' : 'bg-amber-50 border-amber-300 text-amber-900'}`}>
              <h3 className="font-bold text-sm mb-1">No se puede calcular el Total Consolidado</h3>
              <p className="text-xs mb-2">Faltan las siguientes tasas de cambio con fecha igual o anterior al {formatearFechaDDMMYYYY(fechaConsulta)}:</p>
              <ul className="list-disc list-inside text-xs space-y-1 font-semibold">
                {(datosPatrimonio.faltantes || []).map((item, idx) => (
                  <li key={idx}>
                    {typeof item === 'object' && item !== null
                      ? `Falta una tasa de ${item.moneda_origen} a ${item.moneda_destino} con fecha igual o anterior al ${formatearFechaDDMMYYYY(fechaConsulta)}.`
                      : String(item)}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Subtotales por Moneda con parseo flexible */}
          {datosPatrimonio.subtotales && (
            <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
              <h3 className="text-sm font-bold mb-3">Subtotales por Moneda</h3>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                {(() => {
                  const subt = datosPatrimonio.subtotales;
                  let listaSubtotales: { moneda: string; monto: any }[] = [];

                  if (Array.isArray(subt)) {
                    listaSubtotales = subt.map((item: any) => {
                      if (Array.isArray(item)) {
                        return { moneda: String(item[0]), monto: item[1] };
                      }
                      return {
                        moneda: item.moneda || item.codigo || item.moneda_codigo || 'Divisa',
                        monto: item.monto ?? item.total ?? item.saldo ?? 0,
                      };
                    });
                  } else if (typeof subt === 'object' && subt !== null) {
                    listaSubtotales = Object.entries(subt).map(([m, monto]) => ({
                      moneda: m,
                      monto,
                    }));
                  }

                  if (listaSubtotales.length === 0) {
                    return (
                      <p className="text-xs text-gray-400 col-span-full">
                        No hay subtotales disponibles.
                      </p>
                    );
                  }

                  return listaSubtotales.map(({ moneda, monto }, idx) => (
                    <div
                      key={moneda || idx}
                      className="p-3 rounded-xl bg-gray-500/10 border border-gray-500/10"
                    >
                      <span className="text-xs text-gray-400 font-bold block uppercase">
                        {moneda}
                      </span>
                      <span className="text-sm font-extrabold">
                        {formatearMontoTexto(monto, moneda)}
                      </span>
                    </div>
                  ));
                })()}
              </div>
            </div>
          )}

          {datosPatrimonio.cuentas && (
            <div className={`rounded-2xl shadow-sm border overflow-hidden ${bgCard}`}>
              <div className="p-4 border-b border-gray-200 dark:border-gray-800 font-bold text-sm">
                Saldos por Cuenta
              </div>
              <div className="divide-y divide-gray-200 dark:divide-gray-800">
                {datosPatrimonio.cuentas.map((cta) => {
                  const saldoCuenta = cta.saldo ?? (cta as any).saldo_nativo;
                  return (
                    <div key={cta.id} className="p-4 flex justify-between items-center">
                      <div>
                        <span className="font-bold text-sm">{cta.nombre}</span>
                        <span className="ml-2 text-xs font-mono text-gray-400">({cta.moneda})</span>
                      </div>
                      <div className="text-right">
                        <div className="font-bold text-sm">{formatearMontoTexto(saldoCuenta, cta.moneda)}</div>
                        {Boolean(cta.en_moneda_base) && cta.moneda !== datosPatrimonio.moneda_base && (
                          <div className="text-[11px] text-gray-400">
                            ≈ {formatearMontoTexto(cta.en_moneda_base, datosPatrimonio.moneda_base)}
                          </div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
};