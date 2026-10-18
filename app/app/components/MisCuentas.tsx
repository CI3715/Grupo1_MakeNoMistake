'use client';

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ErrorCampo, ListaCuentas as DatosLista, Moneda, NuevaCuenta } from '../types/cuentas';
import { EstadoVacio } from './EstadoVacio';
import { FormularioCuenta } from './FormularioCuenta';
import { IconoLogo, IconoMas } from './Iconos';
import { ListaCuentas } from './ListaCuentas';
import { SelectorApariencia } from './SelectorApariencia';
import { SubtotalesPorMoneda } from './SubtotalesPorMoneda';

const MENSAJE_INESPERADO = 'Ocurrió un problema inesperado. Intenta de nuevo.';

/** Extrae los errores por campo que devuelve el backend (`{ errores: [...] }`). */
function aErrores(e: unknown): ErrorCampo[] {
  if (e && typeof e === 'object' && 'errores' in e) {
    const errores = (e as { errores: unknown }).errores;
    if (Array.isArray(errores) && errores.length > 0) {
      return errores as ErrorCampo[];
    }
  }
  return [{ campo: null, mensaje: MENSAJE_INESPERADO }];
}

interface Datos {
  monedas: Moneda[];
  lista: DatosLista;
}

/** Lee monedas y cuentas del backend local. No toca el estado de React. */
async function obtenerDatos(): Promise<Datos> {
  const [monedas, lista] = await Promise.all([
    invoke<Moneda[]>('listar_monedas'),
    invoke<DatosLista>('listar_cuentas'),
  ]);
  return { monedas, lista };
}

const MENSAJE_CARGA = 'No se pudieron cargar tus cuentas. Cierra y vuelve a abrir la aplicación.';

/**
 * Pantalla principal "Mis cuentas". Todo el flujo es local: solo usa `invoke`
 * hacia el backend de Tauri, sin ninguna petición de red.
 *
 * Diseño de escritorio: cabecera con degradado, tarjetas de saldo por moneda a la
 * izquierda (superpuestas a la cabecera) y un panel con la lista de cuentas a la derecha.
 */
export default function MisCuentas() {
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [lista, setLista] = useState<DatosLista>({ cuentas: [], subtotales: [] });
  const [cargando, setCargando] = useState(true);
  const [errorCarga, setErrorCarga] = useState<string | null>(null);
  const [mostrarFormulario, setMostrarFormulario] = useState(false);
  const [errores, setErrores] = useState<ErrorCampo[]>([]);

  const aplicar = useCallback((datos: Datos) => {
    setMonedas(datos.monedas);
    setLista(datos.lista);
    setErrorCarga(null);
  }, []);

  useEffect(() => {
    let activo = true;
    obtenerDatos()
      .then((datos) => {
        if (activo) aplicar(datos);
      })
      .catch(() => {
        if (activo) setErrorCarga(MENSAJE_CARGA);
      })
      .finally(() => {
        if (activo) setCargando(false);
      });
    return () => {
      activo = false;
    };
  }, [aplicar]);

  const abrirFormulario = () => {
    setErrores([]);
    setMostrarFormulario(true);
  };

  const cerrarFormulario = () => {
    setErrores([]);
    setMostrarFormulario(false);
  };

  const crear = async (datos: NuevaCuenta) => {
    try {
      await invoke('crear_cuenta', { datos });
    } catch (e) {
      setErrores(aErrores(e));
      return;
    }
    cerrarFormulario();
    try {
      aplicar(await obtenerDatos());
    } catch {
      setErrorCarga(MENSAJE_CARGA);
    }
  };

  const total = lista.cuentas.length;
  const listo = !cargando && !errorCarga;

  return (
    <div className="pb-12">
      {/* Cabecera con degradado */}
      <div className="relative rounded-b-[2.5rem] bg-linear-to-br from-grad-a to-grad-b pb-28 text-white">
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 overflow-hidden rounded-b-[2.5rem]">
          <div className="absolute -right-24 -top-32 size-96 rounded-full bg-white/10" />
          <div className="absolute -bottom-44 left-1/3 size-96 rounded-full bg-white/5" />
        </div>

        <div className="relative mx-auto w-full max-w-5xl px-5 pt-6 sm:px-8">
          <header className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-2.5">
              <span className="grid size-9 place-items-center rounded-xl bg-white/15 ring-1 ring-white/25">
                <IconoLogo width={20} height={20} />
              </span>
              <span className="text-base font-semibold tracking-tight">Cuentas Claras</span>
            </div>
            <SelectorApariencia />
          </header>

          <div className="mt-10">
            <h1 className="text-4xl font-semibold tracking-tight">Mis cuentas</h1>
            <p className="mt-2 text-white/75">Guardadas solo en este dispositivo.</p>
          </div>
        </div>
      </div>

      {/* Contenido, superpuesto a la cabecera */}
      <div className="relative mx-auto -mt-16 w-full max-w-5xl px-5 sm:px-8">
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
                Aquí verás el saldo de cada moneda en cuanto crees una cuenta.
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
                <h2 id="titulo-panel" className="text-lg font-semibold tracking-tight">
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
                  onClick={abrirFormulario}
                  className="inline-flex h-10 items-center gap-2 rounded-xl bg-acento px-4 text-sm font-medium text-acento-fg transition hover:brightness-110 active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento"
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
                className="m-6 rounded-xl border border-peligro-borde bg-peligro-suave p-4 text-sm text-peligro"
              >
                {errorCarga}
              </p>
            )}

            {listo && mostrarFormulario && (
              <FormularioCuenta
                monedas={monedas}
                onCrear={crear}
                onCancelar={cerrarFormulario}
                errores={errores}
              />
            )}

            {listo &&
              !mostrarFormulario &&
              (total === 0 ? (
                <EstadoVacio onCrearCuenta={abrirFormulario} />
              ) : (
                <ListaCuentas cuentas={lista.cuentas} monedas={monedas} />
              ))}
          </section>
        </div>
      </div>
    </div>
  );
}
