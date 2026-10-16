'use client';

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ErrorCampo, ListaCuentas as DatosLista, Moneda, NuevaCuenta } from '../types/cuentas';
import { EstadoVacio } from './EstadoVacio';
import { FormularioCuenta } from './FormularioCuenta';
import { ListaCuentas } from './ListaCuentas';
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

  return (
    <div className="mx-auto w-full max-w-3xl">
      <header className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-800">Mis cuentas</h1>
        {!mostrarFormulario && !cargando && !errorCarga && (
          <button
            onClick={abrirFormulario}
            className="rounded-md bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
          >
            Nueva cuenta
          </button>
        )}
      </header>

      {cargando && <p className="text-sm text-gray-500">Cargando tus cuentas…</p>}

      {errorCarga && (
        <p role="alert" className="rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700">
          {errorCarga}
        </p>
      )}

      {!cargando && !errorCarga && mostrarFormulario && (
        <FormularioCuenta
          monedas={monedas}
          onCrear={crear}
          onCancelar={cerrarFormulario}
          errores={errores}
        />
      )}

      {!cargando && !errorCarga && !mostrarFormulario && (
        lista.cuentas.length === 0 ? (
          <EstadoVacio onCrearCuenta={abrirFormulario} />
        ) : (
          <>
            <ListaCuentas cuentas={lista.cuentas} monedas={monedas} />
            <SubtotalesPorMoneda subtotales={lista.subtotales} monedas={monedas} />
          </>
        )
      )}
    </div>
  );
}
