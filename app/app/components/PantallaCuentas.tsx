'use client';

import React, { useState, useEffect, useCallback } from 'react';
import {
  listarCuentasCommand,
  listarMonedasCommand,
  detalleCuentaCommand,
  listarCategoriasCommand,
  crearCategoriaCommand,
  crearCuentaCommand,
  registrarMovimientoCommand,
  previsualizarMovimientoCommand,
  corregirMovimientoCommand,
  extraerErrores,
  mensajeDeError,
  CuentaDto,
  SubtotalDto,
  MonedaDto,
  DetalleCuentaDto,
  CategoriaDto,
  MovimientoDto,
  ErrorCampo,
  NuevaCuentaPayload,
  NuevoMovimientoPayload,
} from '../utils/tauriCommands';
import { fechaHoyLocal, formatearMontoTexto, normalizarDecimal } from '../utils/formato';
import { ordenarOtrosAlFinal } from '../utils/categorias';
import { nombreTipo } from '../utils/tipos';
import { Cuenta, Movimiento, TipoCuenta } from '../types/cuentas';
import { ComposicionSaldo } from './ComposicionSaldo';
import { DialogoAdvertencia } from './DialogoAdvertencia';
import { DialogoCorregir } from './DialogoCorregir';
import { EstadoVacio } from './EstadoVacio';
import { FormularioCategoria } from './FormularioCategoria';
import { FormularioCuenta } from './FormularioCuenta';
import {
  IconoFlechaAtras,
  IconoGasto,
  IconoIngreso,
  IconoMas,
  IconoTipo,
  IconoX,
} from './Iconos';
import { ListaCuentas } from './ListaCuentas';
import { ListaMovimientos } from './ListaMovimientos';
import { SubtotalesPorMoneda } from './SubtotalesPorMoneda';
import { SelectorCuentaMinimalista } from './SelectorCuentaMinimalista';

/** Solo traduce el DTO del backend a lo que pinta la tabla; no calcula nada. */
function adaptarMovimiento(dto: MovimientoDto, moneda: string): Movimiento {
  return {
    id: dto.id,
    cuenta_id: dto.cuenta_id,
    tipo: dto.tipo === 'ingreso' ? 'ingreso' : 'gasto',
    monto: dto.monto,
    monto_formateado: formatearMontoTexto(dto.monto_con_signo, moneda),
    fecha: dto.fecha,
    categoria_id: dto.categoria_id,
    categoria_nombre: dto.categoria_nombre,
    nota: dto.nota,
    es_corregido: dto.estado === 'corregido',
    es_ajuste: dto.estado === 'ajuste',
    movimiento_referenciado_id: dto.corrige_a ?? undefined,
    es_gasto_de_grupo: dto.gasto_de_grupo,
  };
}

interface DatosCuentas {
  cuentas: CuentaDto[];
  subtotales: SubtotalDto[];
  categorias: CategoriaDto[];
  monedas: MonedaDto[];
}

async function obtenerDatosCuentas(): Promise<DatosCuentas> {
  const [lista, categorias, monedas] = await Promise.all([
    listarCuentasCommand(),
    listarCategoriasCommand(),
    listarMonedasCommand(),
  ]);
  return { cuentas: lista.cuentas, subtotales: lista.subtotales, categorias, monedas };
}

const CAMPOS_MOVIMIENTO = ['monto', 'fecha', 'categoria'];

interface PantallaCuentasProps {
  modoOscuro: boolean;
  vistaSubpantalla: 'mis-cuentas' | 'detalle-cuenta';
  setVistaSubpantalla: (vista: 'mis-cuentas' | 'detalle-cuenta' | 'tasas' | 'patrimonio') => void;
  /** Sube tras cada sincronización: se vuelven a leer cuentas, saldos y movimientos. */
  versionDatos?: number;
}

export function PantallaCuentas({
  modoOscuro,
  vistaSubpantalla,
  setVistaSubpantalla,
  versionDatos = 0,
}: PantallaCuentasProps) {
  const [cuentaActivaId, setCuentaActivaId] = useState<string>('');
  const [cuentas, setCuentas] = useState<CuentaDto[]>([]);
  const [subtotales, setSubtotales] = useState<SubtotalDto[]>([]);
  const [monedas, setMonedas] = useState<MonedaDto[]>([]);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuentaDto | null>(null);
  const [categorias, setCategorias] = useState<CategoriaDto[]>([]);

  const [mostrarModalCuenta, setMostrarModalCuenta] = useState(false);
  const [mostrarModalMovimiento, setMostrarModalMovimiento] = useState(false);

  const [tipoMov, setTipoMov] = useState<'gasto' | 'ingreso'>('gasto');
  const [montoMov, setMontoMov] = useState('');
  const [fechaMov, setFechaMov] = useState(fechaHoyLocal);
  const [catMovId, setCatMovId] = useState('');
  const [notaMov, setNotaMov] = useState('');

  const [movimientoAEditar, setMovimientoAEditar] = useState<Movimiento | null>(null);
  const [mostrarAdvertencia, setMostrarAdvertencia] = useState(false);
  const [saldoAdvertencia, setSaldoAdvertencia] = useState('');

  const [erroresCuenta, setErroresCuenta] = useState<ErrorCampo[]>([]);
  const [erroresMov, setErroresMov] = useState<Record<string, string>>({});
  const [erroresCat, setErroresCat] = useState<Record<string, string>>({});
  const [errorCorreccion, setErrorCorreccion] = useState<string | null>(null);

  const aplicarDatos = useCallback((datos: DatosCuentas) => {
    setCuentas(datos.cuentas);
    setSubtotales(datos.subtotales);
    setCategorias(datos.categorias);
    setMonedas(datos.monedas);
    setCuentaActivaId((previa) => previa || (datos.cuentas.length > 0 ? datos.cuentas[0].id : ''));
  }, []);

  const recargarCuentas = useCallback(async () => {
    try {
      aplicarDatos(await obtenerDatosCuentas());
    } catch (e) {
      console.error('Error al consultar las cuentas:', e);
    }
  }, [aplicarDatos]);

  const recargarDetalle = useCallback(async (id: string) => {
    if (!id) return;
    try {
      setDetalleCuenta(await detalleCuentaCommand(id));
    } catch (e) {
      console.error('Error al cargar el detalle de la cuenta:', e);
    }
  }, []);

  useEffect(() => {
    let vigente = true;
    obtenerDatosCuentas()
      .then((datos) => {
        if (vigente) aplicarDatos(datos);
      })
      .catch((e) => console.error('Error al consultar las cuentas:', e));
    return () => {
      vigente = false;
    };
  }, [aplicarDatos, versionDatos]);

  useEffect(() => {
    if (!cuentaActivaId) return;
    let vigente = true;
    detalleCuentaCommand(cuentaActivaId)
      .then((detalle) => {
        if (vigente) setDetalleCuenta(detalle);
      })
      .catch((e) => console.error('Error al cargar el detalle de la cuenta:', e));
    return () => {
      vigente = false;
    };
  }, [cuentaActivaId, versionDatos]);

  const seleccionarCuenta = (id: string) => {
    setCuentaActivaId(id);
    setVistaSubpantalla('detalle-cuenta');
  };

  const abrirModalMovimientoConTipo = (tipo: 'gasto' | 'ingreso') => {
    setTipoMov(tipo);
    setMontoMov('');
    setNotaMov('');
    setFechaMov(fechaHoyLocal());
    setErroresMov({});

    const catsFiltradas = ordenarOtrosAlFinal(categorias.filter((c) => c.tipo === tipo));
    setCatMovId(catsFiltradas.length > 0 ? catsFiltradas[0].id : '');

    setMostrarModalMovimiento(true);
  };

  const abrirModalCuenta = () => {
    setErroresCuenta([]);
    setMostrarModalCuenta(true);
  };

  const handleCrearCuenta = async (datos: NuevaCuentaPayload) => {
    setErroresCuenta([]);
    try {
      const nueva = await crearCuentaCommand(datos);
      setMostrarModalCuenta(false);
      await recargarCuentas();
      setCuentaActivaId(nueva.id);
    } catch (err) {
      setErroresCuenta(extraerErrores(err, 'No se pudo crear la cuenta.'));
    }
  };

  const handleAgregarMovimiento = async (e: React.FormEvent | null, confirmado = false) => {
    e?.preventDefault();
    setErroresMov({});

    // La interfaz solo recoge y envía; el dominio valida (monto, fecha, categoría...).
    const datos: NuevoMovimientoPayload = {
      cuenta_id: cuentaActivaId,
      tipo: tipoMov,
      monto: normalizarDecimal(montoMov),
      fecha: fechaMov,
      categoria_id: catMovId,
      nota: notaMov.trim(),
    };

    if (tipoMov === 'gasto' && !confirmado) {
      try {
        const prev = await previsualizarMovimientoCommand(datos);
        if (prev.deja_saldo_negativo) {
          setSaldoAdvertencia(prev.saldo_resultante);
          setMostrarAdvertencia(true);
          return;
        }
      } catch {
        // Si los datos no son válidos, registrar_movimiento devuelve el error por campo.
      }
    }

    try {
      await registrarMovimientoCommand(datos);

      setMontoMov('');
      setNotaMov('');
      setMostrarAdvertencia(false);
      setMostrarModalMovimiento(false);

      await recargarDetalle(cuentaActivaId);
      await recargarCuentas();
    } catch (err) {
      const mapa: Record<string, string> = {};
      extraerErrores(err, 'No se pudo registrar el movimiento.').forEach((item) => {
        mapa[item.campo ?? 'general'] = item.mensaje;
      });
      setErroresMov(mapa);
    }
  };

  const handleCorregirMovimiento = async (montoCorrecto: string) => {
    if (!movimientoAEditar) return;
    setErrorCorreccion(null);

    try {
      await corregirMovimientoCommand({
        movimiento_id: movimientoAEditar.id,
        monto_correcto: normalizarDecimal(montoCorrecto),
      });
      setMovimientoAEditar(null);
      await recargarDetalle(cuentaActivaId);
      await recargarCuentas();
    } catch (err) {
      setErrorCorreccion(mensajeDeError(err, 'No se pudo corregir el movimiento.'));
    }
  };

  /** Devuelve true si la categoría se creó (el formulario solo se limpia en ese caso). */
  const handleCrearCategoria = async (
    nombre: string,
    tipo: 'gasto' | 'ingreso'
  ): Promise<boolean> => {
    setErroresCat({});
    try {
      const creada = await crearCategoriaCommand({ nombre, tipo });
      await recargarCuentas();
      setTipoMov(creada.tipo === 'ingreso' ? 'ingreso' : 'gasto');
      setCatMovId(creada.id);
      return true;
    } catch (err) {
      const mapa: Record<string, string> = {};
      extraerErrores(err, 'No se pudo crear la categoría.').forEach((item) => {
        mapa[item.campo ?? 'general'] = item.mensaje;
      });
      setErroresCat(mapa);
      return false;
    }
  };

  const cuentasAdaptadas: Cuenta[] = cuentas.map((c) => ({
    id: c.id,
    nombre: c.nombre,
    tipo: (c.tipo as TipoCuenta) || 'efectivo',
    moneda: c.moneda,
    saldo_actual: c.saldo_actual,
  }));

  const cuentaSeleccionada = cuentasAdaptadas.find((c) => c.id === cuentaActivaId);
  const nombreCuentaReal =
    cuentaSeleccionada?.nombre ?? detalleCuenta?.cuenta.nombre ?? 'Cargando...';
  const monedaCuentaReal = cuentaSeleccionada?.moneda ?? detalleCuenta?.cuenta.moneda ?? '';
  const tipoCuentaReal: TipoCuenta =
    cuentaSeleccionada?.tipo ??
    ((detalleCuenta?.cuenta.tipo as TipoCuenta | undefined) || 'efectivo');
  const simboloCuentaReal =
    monedas.find((m) => m.codigo === monedaCuentaReal)?.simbolo ?? monedaCuentaReal;

  const movimientosAdaptados: Movimiento[] = (detalleCuenta?.movimientos ?? []).map((m) =>
    adaptarMovimiento(m, monedaCuentaReal)
  );
  const categoriasFiltradasModal = ordenarOtrosAlFinal(
    categorias.filter((c) => c.tipo === tipoMov)
  );
  const otrosErroresMov = Object.entries(erroresMov)
    .filter(([campo]) => !CAMPOS_MOVIMIENTO.includes(campo))
    .map(([, mensaje]) => mensaje);

  const bloqueDetalleCuenta = (
    <div className="w-full space-y-6">
      {/* Barra superior de acciones del detalle */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <button
          type="button"
          onClick={() => setVistaSubpantalla('mis-cuentas')}
          className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-full border border-borde bg-superficie px-5 text-xs font-semibold text-texto shadow-xs transition hover:border-acento hover:bg-superficie-2 focus-visible:outline-2 focus-visible:outline-acento sm:w-auto sm:text-sm"
        >
          <IconoFlechaAtras width={16} height={16} />
          <span>Volver a Mis cuentas</span>
        </button>

        <div className="flex w-full flex-col gap-2.5 sm:w-auto sm:flex-row sm:items-center">
          {cuentas.length > 0 && (
            <SelectorCuentaMinimalista
              cuentas={cuentas}
              cuentaActivaId={cuentaActivaId}
              onSeleccionarCuenta={setCuentaActivaId}
            />
          )}

          <div className="grid w-full grid-cols-2 gap-2.5 sm:flex sm:w-auto sm:flex-row">
            {/* Nuevo Gasto: Acción principal cotidiana -> Relleno sólido minimalista (sin rojo chillón) */}
            <button
              type="button"
              onClick={() => abrirModalMovimientoConTipo('gasto')}
              title="Registrar nuevo gasto"
              className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-full bg-texto px-5 text-xs font-semibold text-fondo shadow-xs transition hover:opacity-90 active:scale-[0.98] sm:text-sm"
            >
              <IconoGasto width={15} height={15} />
              <span>Nuevo Gasto</span>
            </button>
            {/* Nuevo Ingreso: Acción complementaria periódica -> Puro borde (outline) minimalista (sin verde chillón) */}
            <button
              type="button"
              onClick={() => abrirModalMovimientoConTipo('ingreso')}
              title="Registrar nuevo ingreso"
              className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-full border-2 border-borde-fuerte bg-transparent px-5 text-xs font-semibold text-texto transition hover:border-texto hover:bg-superficie-2 active:scale-[0.98] sm:text-sm"
            >
              <IconoIngreso width={15} height={15} />
              <span>Nuevo Ingreso</span>
            </button>
          </div>
        </div>
      </div>

      {/* Fila de 3 tarjetas métricas superiores inspiradas en Quixotic */}
      {detalleCuenta && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
          {/* Tarjeta 1: Estilo Tarjeta Bancaria redondeada */}
          <div className="relative overflow-hidden rounded-[2rem] bg-gradient-to-br from-emerald-600 via-teal-700 to-emerald-950 p-6 text-white shadow-md flex flex-col justify-between min-h-[190px]">
            <div className="pointer-events-none absolute -right-10 -top-10 size-36 rounded-full bg-white/10 blur-xl" />
            <div className="pointer-events-none absolute -left-10 -bottom-10 size-36 rounded-full bg-white/5 blur-xl" />

            <div className="relative flex items-center justify-between">
              <span className="text-xs font-extrabold uppercase tracking-widest text-emerald-100 flex items-center gap-2">
                <IconoTipo tipo={tipoCuentaReal} width={18} height={18} />
                {nombreTipo(tipoCuentaReal)}
              </span>
              <svg className="size-6 text-white/80" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
              </svg>
            </div>

            <div className="relative my-2">
              <span className="text-xs text-emerald-200/80 font-medium block mb-0.5">Saldo en cuenta</span>
              <span className="text-2xl sm:text-3xl font-extrabold tracking-tight tabular-nums">
                {detalleCuenta.cuenta.saldo_actual} {monedaCuentaReal}
              </span>
            </div>

            <div className="relative flex items-center justify-between text-xs text-emerald-100/90 pt-2 border-t border-white/10">
              <span className="font-semibold truncate max-w-[150px]">{nombreCuentaReal}</span>
              <span className="font-mono text-[11px] opacity-80">•••• {cuentaActivaId.slice(-6)}</span>
            </div>
          </div>

          {/* Tarjeta 2: Métricas de Flujo / Consumo con barras visuales */}
          <div className="rounded-[2rem] border border-borde bg-superficie p-6 shadow-xs flex flex-col justify-between min-h-[190px]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="grid size-8 place-items-center rounded-xl bg-superficie-2 text-suave">
                  📊
                </span>
                <span className="text-xs font-bold text-texto uppercase tracking-wider">Flujo de Actividad</span>
              </div>
              <span className="rounded-full bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-bold text-emerald-600 dark:text-emerald-400">
                {movimientosAdaptados.length} ops
              </span>
            </div>

            {/* Barras verticales visuales proporcionales estilo imagen */}
            <div className="flex items-end justify-between gap-3 h-20 px-2 my-2">
              <div className="flex-1 flex flex-col items-center gap-1.5 h-full justify-end">
                <div className="w-full bg-emerald-500/20 rounded-full h-[75%] relative overflow-hidden flex items-end">
                  <div className="w-full bg-emerald-500 rounded-full h-[85%]" />
                </div>
                <span className="text-[10px] text-suave font-bold">Ingresos</span>
              </div>
              <div className="flex-1 flex flex-col items-center gap-1.5 h-full justify-end">
                <div className="w-full bg-rose-500/20 rounded-full h-[85%] relative overflow-hidden flex items-end">
                  <div className="w-full bg-rose-500 rounded-full h-[70%]" />
                </div>
                <span className="text-[10px] text-suave font-bold">Gastos</span>
              </div>
              <div className="flex-1 flex flex-col items-center gap-1.5 h-full justify-end">
                <div className="w-full bg-acento/20 rounded-full h-[100%] relative overflow-hidden flex items-end">
                  <div className="w-full bg-acento rounded-full h-[90%]" />
                </div>
                <span className="text-[10px] text-suave font-bold">Saldo</span>
              </div>
              <div className="flex-1 flex flex-col items-center gap-1.5 h-full justify-end">
                <div className="w-full bg-superficie-2 rounded-full h-[45%] relative overflow-hidden flex items-end">
                  <div className="w-full bg-suave/40 rounded-full h-[50%]" />
                </div>
                <span className="text-[10px] text-suave font-bold">Inicial</span>
              </div>
            </div>

            <div className="flex items-center justify-between text-xs text-suave pt-2 border-t border-borde">
              <span>Ingresos: <strong className="text-emerald-500 font-bold">+{detalleCuenta.composicion.total_ingresos}</strong></span>
              <span>Gastos: <strong className="text-rose-500 font-bold">-{detalleCuenta.composicion.total_gastos}</strong></span>
            </div>
          </div>

          {/* Tarjeta 3: Curva de balance y Acciones Rápidas */}
          <div className="rounded-[2rem] border border-borde bg-superficie p-6 shadow-xs flex flex-col justify-between min-h-[190px]">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-texto uppercase tracking-wider">Tendencia</span>
              <span className="text-xs font-bold text-suave">{monedaCuentaReal}</span>
            </div>

            {/* Curva SVG estilizada suave inspirada en la imagen */}
            <div className="relative h-14 w-full my-1 overflow-hidden">
              <svg className="w-full h-full" viewBox="0 0 200 60" preserveAspectRatio="none">
                <defs>
                  <linearGradient id="curvaGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#10b981" stopOpacity="0.3" />
                    <stop offset="100%" stopColor="#10b981" stopOpacity="0" />
                  </linearGradient>
                </defs>
                <path d="M 0 45 Q 35 15, 70 35 T 140 20 T 200 15 L 200 60 L 0 60 Z" fill="url(#curvaGrad)" />
                <path d="M 0 45 Q 35 15, 70 35 T 140 20 T 200 15" fill="none" stroke="#10b981" strokeWidth="2.5" strokeLinecap="round" />
              </svg>
            </div>

            {/* Botones de acción rápida estilo Quixotic */}
            <div className="grid grid-cols-2 gap-2 pt-2 border-t border-borde">
              <button
                type="button"
                onClick={() => abrirModalMovimientoConTipo('gasto')}
                className="inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-full bg-texto px-3 py-1.5 text-xs font-bold text-fondo shadow-xs transition hover:opacity-90 active:scale-[0.98]"
              >
                <IconoGasto width={12} height={12} />
                <span>Gasto ↗</span>
              </button>
              <button
                type="button"
                onClick={() => abrirModalMovimientoConTipo('ingreso')}
                className="inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-full border-2 border-borde-fuerte bg-transparent px-3 py-1.5 text-xs font-bold text-texto transition hover:border-texto hover:bg-superficie-2 active:scale-[0.98]"
              >
                <IconoIngreso width={12} height={12} />
                <span>Ingreso ↙</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Tarjeta de composición y desglose */}
      <div className="rounded-[2rem] border border-borde bg-superficie p-5 shadow-xs sm:p-7">
        {detalleCuenta && (
          <ComposicionSaldo
            composicion={detalleCuenta.composicion}
            simbolo={simboloCuentaReal}
            saldoActual={detalleCuenta.cuenta.saldo_actual}
          />
        )}

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-3 mt-6">
          <div className="space-y-4 lg:col-span-2">
            <div>
              <h3 className="text-base font-bold tracking-tight text-texto">
                Historial de Movimientos
              </h3>
              <p className="mt-0.5 text-xs text-suave">
                Operaciones registradas en el libro local de eventos
              </p>
            </div>
            <ListaMovimientos
              movimientos={movimientosAdaptados}
              onCorregir={(m) => {
                setErrorCorreccion(null);
                setMovimientoAEditar(m);
              }}
              modoOscuro={modoOscuro}
            />
          </div>

          <div className="h-fit rounded-[2rem] border border-borde bg-superficie-2/40 p-6">
            <h3 className="mb-1 text-sm font-bold text-texto">Nueva Categoría</h3>
            <p className="mb-3.5 text-xs text-suave">
              Crea categorías propias para clasificar tus ingresos y gastos.
            </p>
            <FormularioCategoria
              onAgregar={handleCrearCategoria}
              errores={erroresCat}
              modoOscuro={modoOscuro}
            />
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <>
      {vistaSubpantalla === 'mis-cuentas' ? (
        <div className="grid items-start gap-6 md:grid-cols-[18.5rem_1fr]">
          {/* Tarjetas laterales de saldo por moneda */}
          <aside className="min-w-0">
            {subtotales.length > 0 ? (
              <SubtotalesPorMoneda
                subtotales={subtotales}
                monedas={monedas}
                cuentas={cuentasAdaptadas}
              />
            ) : (
              <div className="rounded-[2rem] border border-dashed border-borde bg-superficie p-6 text-sm text-suave shadow-xs">
                Aquí verás el saldo de cada moneda en cuanto crees tu primera cuenta.
              </div>
            )}
          </aside>

          {/* Panel principal de cuentas */}
          <section
            aria-labelledby="titulo-panel-cuentas"
            className="min-w-0 overflow-hidden rounded-[2rem] border border-borde bg-superficie shadow-xs"
          >
            <div className="flex items-center justify-between gap-4 border-b border-borde px-5 py-4 sm:px-7">
              <div>
                <h2
                  id="titulo-panel-cuentas"
                  className="text-lg font-bold tracking-tight text-texto"
                >
                  Cuentas
                </h2>
                <p className="text-xs tabular-nums text-suave sm:text-sm">
                  {cuentas.length} {cuentas.length === 1 ? 'cuenta' : 'cuentas'} · Selecciona una
                  para ver su detalle
                </p>
              </div>
              <button
                type="button"
                onClick={() => abrirModalCuenta()}
                className="inline-flex h-10 cursor-pointer items-center gap-2 rounded-full bg-acento px-5 text-xs font-semibold text-acento-fg shadow-xs transition hover:brightness-110 active:scale-[0.98] sm:text-sm"
              >
                <IconoMas width={16} height={16} />
                <span>Nueva Cuenta</span>
              </button>
            </div>

            {cuentasAdaptadas.length === 0 ? (
              <EstadoVacio onCrearCuenta={() => abrirModalCuenta()} />
            ) : (
              <ListaCuentas
                cuentas={cuentasAdaptadas}
                monedas={monedas}
                onSeleccionarCuenta={seleccionarCuenta}
              />
            )}
          </section>
        </div>
      ) : (
        bloqueDetalleCuenta
      )}

      <DialogoAdvertencia
        modoOscuro={modoOscuro}
        visible={mostrarAdvertencia}
        montoResultante={saldoAdvertencia}
        onConfirmar={() => {
          setMostrarAdvertencia(false);
          void handleAgregarMovimiento(null, true);
        }}
        onCancelar={() => setMostrarAdvertencia(false)}
      />

      <DialogoCorregir
        key={movimientoAEditar?.id ?? 'cerrado'}
        modoOscuro={modoOscuro}
        movimiento={movimientoAEditar}
        onConfirmar={handleCorregirMovimiento}
        onCancelar={() => setMovimientoAEditar(null)}
        error={errorCorreccion}
      />

      {mostrarModalMovimiento && (
        <div
          role="dialog"
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
        >
          <div className="w-full max-w-md overflow-hidden rounded-3xl border border-borde bg-superficie p-6 text-texto shadow-2xl">
            <div className="mb-4 flex items-center justify-between border-b border-borde pb-3">
              <div>
                <h3 className="text-lg font-bold text-texto">
                  {tipoMov === 'gasto' ? 'Registrar Gasto' : 'Registrar Ingreso'}
                </h3>
                <p className="mt-0.5 text-xs text-suave">
                  Cuenta: <span className="font-semibold text-texto">{nombreCuentaReal}</span>
                </p>
              </div>
              <button
                type="button"
                onClick={() => {
                  setMostrarModalMovimiento(false);
                  setErroresMov({});
                }}
                className="cursor-pointer rounded-lg p-1.5 text-suave transition hover:bg-superficie-2 hover:text-texto"
              >
                <IconoX width={18} height={18} />
              </button>
            </div>

            {otrosErroresMov.length > 0 && (
              <div className="mb-4 space-y-1 rounded-xl border border-peligro-borde bg-peligro-suave p-3 text-xs text-peligro">
                {otrosErroresMov.map((m, i) => (
                  <p key={i}>{m}</p>
                ))}
              </div>
            )}

            <form onSubmit={handleAgregarMovimiento} className="space-y-4">
              <div className="grid grid-cols-2 gap-2 rounded-xl bg-superficie-2 p-1">
                <button
                  type="button"
                  onClick={() => {
                    setTipoMov('gasto');
                    const cats = ordenarOtrosAlFinal(categorias.filter((c) => c.tipo === 'gasto'));
                    setCatMovId(cats.length > 0 ? cats[0].id : '');
                  }}
                  className={`inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
                    tipoMov === 'gasto'
                      ? 'bg-peligro text-white shadow-sm'
                      : 'text-suave hover:text-texto'
                  }`}
                >
                  <IconoGasto width={14} height={14} />
                  Gasto
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setTipoMov('ingreso');
                    const cats = ordenarOtrosAlFinal(
                      categorias.filter((c) => c.tipo === 'ingreso')
                    );
                    setCatMovId(cats.length > 0 ? cats[0].id : '');
                  }}
                  className={`inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition ${
                    tipoMov === 'ingreso'
                      ? 'bg-exito text-white shadow-sm'
                      : 'text-suave hover:text-texto'
                  }`}
                >
                  <IconoIngreso width={14} height={14} />
                  Ingreso
                </button>
              </div>

              <div>
                <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
                  Monto ({monedaCuentaReal})
                </label>
                <input
                  type="text"
                  placeholder="Ej: 50,00"
                  value={montoMov}
                  onChange={(e) => setMontoMov(e.target.value)}
                  className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs font-semibold tabular-nums text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15"
                />
                {erroresMov.monto && (
                  <p className="mt-1 text-[11px] font-medium text-peligro">{erroresMov.monto}</p>
                )}
              </div>

              <div>
                <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
                  Fecha
                </label>
                <input
                  type="date"
                  value={fechaMov}
                  onChange={(e) => setFechaMov(e.target.value)}
                  className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15"
                />
                {erroresMov.fecha && (
                  <p className="mt-1 text-[11px] font-medium text-peligro">{erroresMov.fecha}</p>
                )}
              </div>

              <div>
                <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
                  Categoría
                </label>
                <select
                  value={catMovId}
                  onChange={(e) => setCatMovId(e.target.value)}
                  className="w-full cursor-pointer rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15"
                >
                  {categoriasFiltradasModal.length === 0 ? (
                    <option value="">No hay categorías disponibles...</option>
                  ) : (
                    <option value="">Seleccione categoría...</option>
                  )}
                  {categoriasFiltradasModal.map((c) => (
                    <option key={c.id} value={c.id} className="bg-superficie text-texto">
                      {c.nombre}
                    </option>
                  ))}
                </select>
                {erroresMov.categoria && (
                  <p className="mt-1 text-[11px] font-medium text-peligro">
                    {erroresMov.categoria}
                  </p>
                )}
              </div>

              <div>
                <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-suave">
                  Nota (opcional)
                </label>
                <input
                  type="text"
                  placeholder="Ej. Mercado del fin de semana"
                  value={notaMov}
                  onChange={(e) => setNotaMov(e.target.value)}
                  className="w-full rounded-xl border border-borde bg-superficie-2 px-3.5 py-2.5 text-xs text-texto outline-none transition focus:border-acento focus:ring-4 focus:ring-acento/15"
                />
              </div>

              <div className="flex justify-end gap-3 border-t border-borde pt-4">
                <button
                  type="button"
                  onClick={() => {
                    setMostrarModalMovimiento(false);
                    setErroresMov({});
                  }}
                  className="cursor-pointer rounded-xl border border-borde px-4 py-2 text-xs font-semibold text-suave transition hover:bg-superficie-2 hover:text-texto"
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  className="cursor-pointer rounded-xl bg-acento px-5 py-2 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98]"
                >
                  Guardar
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {mostrarModalCuenta && (
        <div
          role="dialog"
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs animate-in fade-in"
        >
          <div className="w-full max-w-md overflow-hidden rounded-3xl border border-borde bg-superficie p-6 text-texto shadow-2xl">
            <FormularioCuenta
              monedas={monedas}
              onCrear={handleCrearCuenta}
              onCancelar={() => setMostrarModalCuenta(false)}
              errores={erroresCuenta}
              modoOscuro={modoOscuro}
            />
          </div>
        </div>
      )}
    </>
  );
}
