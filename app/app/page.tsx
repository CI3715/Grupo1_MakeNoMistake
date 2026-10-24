'use client';

import React, { useCallback, useEffect, useState } from 'react';
// Activa la simulación interactiva con mockIPC si corre en el navegador puro
import './utils/mock-dev';

import {
  listarMonedasCommand,
  listarCuentasCommand,
  crearCuentaCommand,
  detalleCuentaCommand,
  listarCategoriasCommand,
  crearCategoriaCommand,
  registrarMovimientoCommand,
  previsualizarMovimientoCommand,
  corregirMovimientoCommand,
} from './utils/tauriCommands';

import {
  Moneda,
  ListaCuentas as DatosLista,
  Cuenta,
  NuevaCuenta,
  ErrorCampo,
  Categoria,
  DetalleCuentaData,
  TipoMovimiento,
} from './types/cuentas';

import { MisCuentas } from './components/MisCuentas';
import { DetalleCuenta } from './components/DetalleCuenta';
import { SelectorApariencia } from './components/SelectorApariencia';
import { IconoLogo } from './components/Iconos';

const MENSAJE_INESPERADO = 'Ocurrió un problema inesperado. Intenta de nuevo.';
const MENSAJE_CARGA = 'No se pudieron cargar tus cuentas. Cierra y vuelve a abrir la aplicación.';

function aErrores(e: unknown): ErrorCampo[] {
  if (e && typeof e === 'object' && 'errores' in e) {
    const errores = (e as { errores: unknown }).errores;
    if (Array.isArray(errores) && errores.length > 0) {
      return errores as ErrorCampo[];
    }
  }
  return [{ campo: null, mensaje: typeof e === 'string' ? e : MENSAJE_INESPERADO }];
}

export default function Home() {
  const [pantallaActual, setPantallaActual] = useState<'mis-cuentas' | 'detalle-cuenta'>('mis-cuentas');
  const [cuentaActivaId, setCuentaActivaId] = useState<string>('');

  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [lista, setLista] = useState<DatosLista>({ cuentas: [], subtotales: [] });
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuentaData | null>(null);

  const [cargando, setCargando] = useState(true);
  const [errorCarga, setErrorCarga] = useState<string | null>(null);

  const [mostrarFormularioCuenta, setMostrarFormularioCuenta] = useState(false);
  const [erroresCuenta, setErroresCuenta] = useState<ErrorCampo[]>([]);

  // Carga general de cuentas, monedas y catálogo de categorías
  const cargarDatos = useCallback(async () => {
    try {
      const [resMonedas, resCuentas, resCategorias] = await Promise.all([
        listarMonedasCommand(),
        listarCuentasCommand(),
        listarCategoriasCommand().catch(() => []),
      ]);

      const cuentas = (resCuentas.cuentas || []).map((c) => ({
        id: c.id,
        nombre: c.nombre,
        tipo: c.tipo as any,
        moneda: c.moneda,
        saldo_actual: c.saldo_actual,
      }));

      const subtotales = resCuentas.subtotales || [];

      setMonedas(resMonedas || []);
      setLista({ cuentas, subtotales });
      setCategorias(
        (resCategorias || []).map((cat) => ({
          id: cat.id,
          nombre: cat.nombre,
          tipo: cat.tipo as any,
          de_fabrica: cat.de_fabrica,
        }))
      );
      setErrorCarga(null);
    } catch (e) {
      console.error('Error al cargar datos:', e);
      setErrorCarga(MENSAJE_CARGA);
    } finally {
      setCargando(false);
    }
  }, []);

  // Cargar detalle de una cuenta específica
  const cargarDetalle = useCallback(async (id: string) => {
    if (!id) return;
    try {
      const res = await detalleCuentaCommand(id);
      const cuentaFormateada: Cuenta = {
        id: res.cuenta.id,
        nombre: res.cuenta.nombre,
        tipo: res.cuenta.tipo as any,
        moneda: res.cuenta.moneda,
        saldo_actual: res.cuenta.saldo_actual,
      };

      setDetalleCuenta({
        cuenta: cuentaFormateada,
        composicion: res.composicion,
        movimientos: res.movimientos.map((m) => ({
          ...m,
          monto_formateado: m.monto,
        })),
      });
    } catch (e) {
      console.error('Error al cargar detalle de cuenta:', e);
    }
  }, []);

  useEffect(() => {
    cargarDatos();
  }, [cargarDatos]);

  useEffect(() => {
    if (cuentaActivaId) {
      cargarDetalle(cuentaActivaId);
    }
  }, [cuentaActivaId, cargarDetalle]);

  const seleccionarCuenta = (id: string) => {
    setCuentaActivaId(id);
    setPantallaActual('detalle-cuenta');
  };

  const handleCrearCuenta = async (datos: NuevaCuenta) => {
    try {
      await crearCuentaCommand({
        nombre: datos.nombre,
        tipo: datos.tipo,
        moneda: datos.moneda,
        saldo_inicial: datos.saldo_inicial,
      });
      setMostrarFormularioCuenta(false);
      setErroresCuenta([]);
      await cargarDatos();
    } catch (e) {
      setErroresCuenta(aErrores(e));
    }
  };

  const handleRegistrarMovimiento = async (datos: {
    cuenta_id: string;
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => {
    await registrarMovimientoCommand(datos);
    await Promise.all([cargarDetalle(datos.cuenta_id), cargarDatos()]);
  };

  const handlePrevisualizarMovimiento = async (datos: {
    cuenta_id: string;
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => {
    return await previsualizarMovimientoCommand(datos);
  };

  const handleCorregirMovimiento = async (movimientoId: string, montoCorrecto: string) => {
    await corregirMovimientoCommand(movimientoId, montoCorrecto);
    if (cuentaActivaId) {
      await Promise.all([cargarDetalle(cuentaActivaId), cargarDatos()]);
    }
  };

  const handleCrearCategoria = async (nombre: string, tipo: TipoMovimiento) => {
    await crearCategoriaCommand(nombre, tipo);
    const resCat = await listarCategoriasCommand();
    setCategorias(
      resCat.map((cat) => ({
        id: cat.id,
        nombre: cat.nombre,
        tipo: cat.tipo as any,
        de_fabrica: cat.de_fabrica,
      }))
    );
  };

  return (
    <div className="min-h-screen bg-fondo text-texto pb-16 transition-colors duration-200">
      {/* Cabecera con degradado distintivo de Angel */}
      <div className="relative rounded-b-[2.5rem] bg-linear-to-br from-grad-a to-grad-b pb-28 text-white shadow-lg">
        {/* Círculos decorativos con efecto de luz y desenfoque ambiental */}
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 overflow-hidden rounded-b-[2.5rem]">
          <div className="absolute -right-24 -top-32 size-96 rounded-full bg-white/10 blur-2xl" />
          <div className="absolute -bottom-44 left-1/3 size-96 rounded-full bg-white/5 blur-3xl" />
        </div>

        <div className="relative mx-auto w-full max-w-5xl px-5 pt-6 sm:px-8">
          {/* Barra superior de navegación */}
          <header className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <span className="grid size-10 place-items-center rounded-2xl bg-white/15 shadow-sm ring-1 ring-white/25 backdrop-blur-xs">
                <IconoLogo width={22} height={22} />
              </span>
              <div>
                <span className="text-lg font-bold tracking-tight">Cuentas Claras</span>
                <span className="hidden ml-2 rounded-md bg-white/15 px-2 py-0.5 text-[10px] font-semibold tracking-wider text-white/90 sm:inline">
                  LOCAL-FIRST
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2.5">
              {/* Píldora de navegación entre vistas */}
              <nav aria-label="Vistas principales" className="flex rounded-xl bg-white/15 p-1 ring-1 ring-white/20 backdrop-blur-xs">
                <button
                  type="button"
                  onClick={() => setPantallaActual('mis-cuentas')}
                  className={`rounded-lg px-3 py-1.5 text-xs font-semibold transition ${
                    pantallaActual === 'mis-cuentas'
                      ? 'bg-white text-texto shadow-sm'
                      : 'text-white/80 hover:text-white'
                  }`}
                >
                  Mis cuentas
                </button>
                {lista.cuentas.length > 0 && (
                  <button
                    type="button"
                    onClick={() => {
                      if (!cuentaActivaId && lista.cuentas.length > 0) {
                        setCuentaActivaId(lista.cuentas[0].id);
                      }
                      setPantallaActual('detalle-cuenta');
                    }}
                    className={`rounded-lg px-3 py-1.5 text-xs font-semibold transition ${
                      pantallaActual === 'detalle-cuenta'
                        ? 'bg-white text-texto shadow-sm'
                        : 'text-white/80 hover:text-white'
                    }`}
                  >
                    Detalle de cuenta
                  </button>
                )}
              </nav>

              {/* Selector de apariencia (tema y paletas) */}
              <SelectorApariencia />
            </div>
          </header>

          {/* Título de la cabecera */}
          <div className="mt-8">
            <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
              {pantallaActual === 'mis-cuentas' ? 'Mis cuentas' : detalleCuenta?.cuenta.nombre || 'Detalle de cuenta'}
            </h1>
            <p className="mt-1.5 text-sm text-white/80">
              {pantallaActual === 'mis-cuentas'
                ? 'Todas tus cuentas y saldos guardados localmente en este dispositivo.'
                : `Libro de movimientos y desglose de saldo en ${detalleCuenta?.cuenta.moneda || ''}.`}
            </p>
          </div>
        </div>
      </div>

      {/* Contenido principal, superpuesto a la cabecera con margen negativo */}
      <main className="relative mx-auto -mt-16 w-full max-w-5xl px-5 sm:px-8">
        {pantallaActual === 'mis-cuentas' ? (
          <MisCuentas
            monedas={monedas}
            lista={lista}
            cargando={cargando}
            errorCarga={errorCarga}
            mostrarFormulario={mostrarFormularioCuenta}
            errores={erroresCuenta}
            onAbrirFormulario={() => {
              setErroresCuenta([]);
              setMostrarFormularioCuenta(true);
            }}
            onCerrarFormulario={() => {
              setErroresCuenta([]);
              setMostrarFormularioCuenta(false);
            }}
            onCrearCuenta={handleCrearCuenta}
            onSeleccionarCuenta={seleccionarCuenta}
          />
        ) : detalleCuenta ? (
          <DetalleCuenta
            detalle={detalleCuenta}
            cuentas={lista.cuentas}
            monedas={monedas}
            categorias={categorias}
            onVolver={() => setPantallaActual('mis-cuentas')}
            onCambiarCuenta={(id) => setCuentaActivaId(id)}
            onRegistrarMovimiento={handleRegistrarMovimiento}
            onPrevisualizarMovimiento={handlePrevisualizarMovimiento}
            onCorregirMovimiento={handleCorregirMovimiento}
            onCrearCategoria={handleCrearCategoria}
          />
        ) : (
          <div className="rounded-3xl border border-borde bg-superficie p-12 text-center shadow-suave">
            <p className="text-sm font-medium text-suave">Cargando detalle de la cuenta…</p>
          </div>
        )}
      </main>
    </div>
  );
}