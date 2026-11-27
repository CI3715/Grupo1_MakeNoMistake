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
import { IconoCheckCirculo, IconoMas } from './Iconos';
import { MenuDesplegable } from './MenuDesplegable';

interface PantallaPatrimonioProps {
  modoOscuro?: boolean;
  /** Sube tras cada sincronización: el patrimonio se vuelve a leer sin mostrar «cargando». */
  versionDatos?: number;
}

interface ResultadoPatrimonio {
  /** Clave (fecha + número de recarga) a la que corresponde este resultado. */
  clave: string;
  datos: PatrimonioDto | null;
  error: string | null;
}

const MONEDA_VACIA = { codigo: '', nombre: '', simbolo: '', decimales: '2' };

const CAMPO =
  'rounded-xl border border-borde bg-superficie-2 px-3.5 py-2 text-xs font-medium text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15';

export const PantallaPatrimonio: React.FC<PantallaPatrimonioProps> = ({ versionDatos = 0 }) => {
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
  }, [fechaConsulta, recarga, versionDatos]);

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

  return (
    <div className="w-full space-y-6">
      {mensajeExitoMoneda && (
        <div className="flex items-center gap-2 rounded-2xl border border-exito-borde bg-exito-suave p-3.5 text-xs font-semibold text-exito-texto shadow-sm">
          <IconoCheckCirculo className="h-4 w-4 shrink-0 text-exito" />
          <span>{mensajeExitoMoneda}</span>
        </div>
      )}

      {errorBase && (
        <div className="rounded-2xl border border-peligro-borde bg-peligro-suave p-3.5 text-xs text-peligro">
          {errorBase}
        </div>
      )}

      {/* Barra de controles: Moneda Base, Fecha de consulta y Crear moneda propia */}
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-3xl border border-borde bg-superficie p-4 text-texto shadow-suave sm:p-5">
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2.5">
            <label htmlFor="moneda-base" className="text-xs font-semibold text-suave">
              Moneda Base:
            </label>
            <MenuDesplegable
              id="moneda-base"
              valor={monedaBase}
              alCambiar={handleCambiarMonedaBase}
              variante="boton"
              anchoPopup="w-64"
              opciones={monedas.map((m) => ({
                valor: m.codigo,
                etiqueta: `${m.codigo} - ${m.nombre}`,
                secundario: m.simbolo,
              }))}
            />
          </div>

          <div className="flex items-center gap-2.5">
            <label htmlFor="fecha-consulta" className="text-xs font-semibold text-suave">
              Fecha de consulta:
            </label>
            <input
              id="fecha-consulta"
              type="date"
              value={fechaConsulta}
              max={fechaHoyLocal()}
              onChange={(e) => setFechaConsulta(e.target.value)}
              className={CAMPO}
            />
          </div>
        </div>

        <button
          type="button"
          onClick={() => setMostrarCrearMoneda(true)}
          className="inline-flex cursor-pointer items-center gap-1.5 rounded-xl bg-acento px-4 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
        >
          <IconoMas className="h-3.5 w-3.5" />
          <span>Crear moneda propia</span>
        </button>
      </div>

      {mostrarCrearMoneda && (
        <div
          role="dialog"
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"
        >
          <div className="w-full max-w-md rounded-3xl border border-borde bg-superficie p-6 text-texto shadow-2xl">
            <h3 className="mb-4 text-base font-bold text-texto">Nueva Moneda Personalizada</h3>
            {erroresFormMoneda.general && (
              <p className="mb-3 text-xs font-semibold text-peligro">
                {erroresFormMoneda.general}
              </p>
            )}

            <form onSubmit={handleCrearMoneda} className="space-y-4">
              <div>
                <label
                  htmlFor="codigo-moneda"
                  className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
                >
                  Código (3 a 5 letras mayúsculas)
                </label>
                <input
                  id="codigo-moneda"
                  type="text"
                  maxLength={5}
                  value={nuevaMoneda.codigo}
                  onChange={(e) =>
                    setNuevaMoneda({ ...nuevaMoneda, codigo: e.target.value.toUpperCase() })
                  }
                  className={`${CAMPO} w-full`}
                  placeholder="Ej. VES, BTC"
                />
                {erroresFormMoneda.codigo && (
                  <span className="mt-1 block text-[11px] font-medium text-peligro">
                    {erroresFormMoneda.codigo}
                  </span>
                )}
              </div>

              <div>
                <label
                  htmlFor="nombre-moneda"
                  className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
                >
                  Nombre
                </label>
                <input
                  id="nombre-moneda"
                  type="text"
                  value={nuevaMoneda.nombre}
                  onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, nombre: e.target.value })}
                  className={`${CAMPO} w-full`}
                  placeholder="Ej. Bolívar Soberano"
                />
                {erroresFormMoneda.nombre && (
                  <span className="mt-1 block text-[11px] font-medium text-peligro">
                    {erroresFormMoneda.nombre}
                  </span>
                )}
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label
                    htmlFor="simbolo-moneda"
                    className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
                  >
                    Símbolo
                  </label>
                  <input
                    id="simbolo-moneda"
                    type="text"
                    value={nuevaMoneda.simbolo}
                    onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, simbolo: e.target.value })}
                    className={`${CAMPO} w-full`}
                    placeholder="Ej. Bs."
                  />
                  {erroresFormMoneda.simbolo && (
                    <span className="mt-1 block text-[11px] font-medium text-peligro">
                      {erroresFormMoneda.simbolo}
                    </span>
                  )}
                </div>
                <div>
                  <label
                    htmlFor="decimales-moneda"
                    className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave"
                  >
                    Decimales (0 a 8)
                  </label>
                  <input
                    id="decimales-moneda"
                    type="text"
                    inputMode="numeric"
                    value={nuevaMoneda.decimales}
                    onChange={(e) => setNuevaMoneda({ ...nuevaMoneda, decimales: e.target.value })}
                    className={`${CAMPO} w-full`}
                  />
                  {erroresFormMoneda.decimales && (
                    <span className="mt-1 block text-[11px] font-medium text-peligro">
                      {erroresFormMoneda.decimales}
                    </span>
                  )}
                </div>
              </div>

              <div className="flex justify-end gap-3 border-t border-borde pt-4">
                <button
                  type="button"
                  onClick={() => setMostrarCrearMoneda(false)}
                  className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  className="cursor-pointer rounded-xl bg-acento px-5 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110"
                >
                  Guardar Moneda
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {cargando ? (
        <div className="p-8 text-center text-xs text-suave">Cargando patrimonio...</div>
      ) : errorBackend ? (
        <div className="rounded-3xl border border-peligro-borde bg-peligro-suave p-4 text-xs text-peligro">
          {errorBackend}
        </div>
      ) : datosPatrimonio ? (
        <div className="space-y-6">
          {datosPatrimonio.total !== null ? (
            <div className="relative overflow-hidden rounded-3xl bg-linear-to-br from-grad-a to-grad-b p-6 text-white shadow-lg ring-1 ring-white/20">
              <span
                aria-hidden="true"
                className="pointer-events-none absolute -right-12 -top-16 size-56 rounded-full bg-white/10"
              />
              <span
                aria-hidden="true"
                className="pointer-events-none absolute -bottom-20 -left-10 size-52 rounded-full bg-white/5"
              />
              <h2 className="relative text-xs font-bold uppercase tracking-wider text-white/80">
                Patrimonio Total Consolidado ({datosPatrimonio.moneda_base})
              </h2>
              <div className="relative mt-2 text-3xl font-extrabold tabular-nums tracking-tight sm:text-4xl">
                {formatearMontoTexto(datosPatrimonio.total, datosPatrimonio.moneda_base)}
              </div>

              {datosPatrimonio.tasas_usadas.length > 0 && (
                <div className="relative mt-4 space-y-1 border-t border-white/20 pt-4 text-xs text-white/85">
                  <p className="mb-1 font-semibold">Tasas aplicadas para la consolidación:</p>
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
                        <span className="rounded-md bg-white/15 px-1.5 py-0.5 text-[10px] font-bold text-white">
                          Inversa
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="rounded-3xl border border-aviso-borde bg-aviso-suave p-5 text-aviso-texto">
              <h3 className="mb-1 text-sm font-bold">No se puede calcular el Total Consolidado</h3>
              <ul className="list-inside list-disc space-y-1 text-xs font-semibold">
                {datosPatrimonio.faltantes.map((item) => (
                  <li key={`${item.moneda_origen}-${item.moneda_destino}`}>
                    Falta una tasa de {item.moneda_origen} a {item.moneda_destino} con fecha igual o
                    anterior al {formatearFechaDDMMYYYY(datosPatrimonio.fecha_consulta)}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div className="rounded-3xl border border-borde bg-superficie p-5 text-texto shadow-suave sm:p-6">
            <h3 className="mb-3 text-sm font-bold text-texto">Subtotales por Moneda</h3>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              {datosPatrimonio.subtotales.length === 0 ? (
                <p className="col-span-full text-xs text-suave">No hay subtotales disponibles.</p>
              ) : (
                datosPatrimonio.subtotales.map((sub) => (
                  <div
                    key={sub.moneda}
                    className="rounded-2xl border border-borde bg-superficie-2 p-3.5"
                  >
                    <span className="block text-xs font-bold uppercase text-suave">
                      {sub.moneda}
                    </span>
                    <span className="mt-1 block text-sm font-extrabold tabular-nums text-texto">
                      {formatearMontoTexto(sub.total, sub.moneda)}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>

          {datosPatrimonio.cuentas.length > 0 && (
            <div className="overflow-hidden rounded-3xl border border-borde bg-superficie text-texto shadow-suave">
              <div className="border-b border-borde px-5 py-4 text-sm font-bold text-texto">
                Saldos por Cuenta
              </div>
              <div className="divide-y divide-borde">
                {datosPatrimonio.cuentas.map((cta) => (
                  <div
                    key={cta.id}
                    className="flex items-center justify-between px-5 py-4 transition hover:bg-superficie-2/50"
                  >
                    <div>
                      <span className="text-sm font-bold text-texto">{cta.nombre}</span>
                      <span className="ml-2 rounded-full bg-acento-suave px-2 py-0.5 font-mono text-xs font-semibold text-acento-texto">
                        {cta.moneda}
                      </span>
                    </div>
                    <div className="text-right">
                      <div className="text-sm font-bold tabular-nums text-texto">
                        {formatearMontoTexto(cta.saldo, cta.moneda)}
                      </div>
                      {cta.en_moneda_base !== null &&
                        cta.moneda !== datosPatrimonio.moneda_base && (
                          <div className="text-[11px] tabular-nums text-suave">
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
