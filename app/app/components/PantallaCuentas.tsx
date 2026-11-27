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
import type { DisenoEscritorio } from '../utils/apariencia';

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
  diseno?: DisenoEscritorio;
}

export function PantallaCuentas({
  modoOscuro,
  vistaSubpantalla,
  setVistaSubpantalla,
  versionDatos = 0,
  diseno = 'barra-superior',
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
        {diseno !== 'dividida' && (
          <button
            type="button"
            onClick={() => setVistaSubpantalla('mis-cuentas')}
            className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-2xl border border-borde bg-superficie px-4 text-xs font-semibold text-texto shadow-suave transition hover:border-acento hover:bg-superficie-2 focus-visible:outline-2 focus-visible:outline-acento sm:w-auto sm:text-sm"
          >
            <IconoFlechaAtras width={16} height={16} />
            <span>Volver a Mis cuentas</span>
          </button>
        )}

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
              className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-xl bg-texto px-4 text-xs font-semibold text-fondo shadow-xs transition hover:opacity-90 active:scale-[0.98] sm:text-sm"
            >
              <IconoGasto width={15} height={15} />
              <span>Nuevo Gasto</span>
            </button>
            {/* Nuevo Ingreso: Acción complementaria periódica -> Puro borde (outline) minimalista (sin verde chillón) */}
            <button
              type="button"
              onClick={() => abrirModalMovimientoConTipo('ingreso')}
              title="Registrar nuevo ingreso"
              className="inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-xl border-2 border-borde-fuerte bg-transparent px-4 text-xs font-semibold text-texto transition hover:border-texto hover:bg-superficie-2 active:scale-[0.98] sm:text-sm"
            >
              <IconoIngreso width={15} height={15} />
              <span>Nuevo Ingreso</span>
            </button>
          </div>
        </div>
      </div>

      {/* Tarjeta de cabecera y composición de la cuenta */}
      <div className="rounded-3xl border border-borde bg-superficie p-4 shadow-suave sm:p-6">
        <div className="mb-5 flex flex-wrap items-center justify-between gap-3 border-b border-borde pb-4">
          <div className="flex items-center gap-3">
            <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-acento-suave text-acento-texto shadow-sm sm:size-12">
              <IconoTipo tipo={tipoCuentaReal} width={22} height={22} />
            </span>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="truncate text-lg font-bold tracking-tight text-texto sm:text-2xl">
                  {nombreCuentaReal} ({monedaCuentaReal})
                </h2>
                <span className="rounded-full bg-acento-suave px-2.5 py-0.5 text-xs font-bold text-acento-texto">
                  {monedaCuentaReal}
                </span>
              </div>
              <p className="mt-0.5 text-xs text-suave">
                {nombreTipo(tipoCuentaReal)} · ID de la cuenta:{' '}
                <span className="font-mono text-[11px] opacity-80">{cuentaActivaId}</span>
              </p>
            </div>
          </div>
        </div>

        {detalleCuenta && (
          <ComposicionSaldo
            composicion={detalleCuenta.composicion}
            simbolo={simboloCuentaReal}
            saldoActual={detalleCuenta.cuenta.saldo_actual}
          />
        )}

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
          <div className="space-y-4 lg:col-span-2">
            <div>
              <h3 className="text-base font-semibold tracking-tight text-texto">
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

          <div className="h-fit rounded-2xl border border-borde bg-superficie-2/40 p-5">
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
      {diseno === 'dividida' ? (
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 items-start">
          {/* Panel Izquierdo: Subtotales y Cuentas (4 cols) */}
          <div className="space-y-6 lg:col-span-4">
            {subtotales.length > 0 && (
              <aside className="min-w-0">
                <SubtotalesPorMoneda
                  subtotales={subtotales}
                  monedas={monedas}
                  cuentas={cuentasAdaptadas}
                />
              </aside>
            )}
            <section
              aria-labelledby="titulo-panel-cuentas-dividida"
              className="min-w-0 overflow-hidden rounded-3xl border border-borde bg-superficie shadow-suave"
            >
              <div className="flex items-center justify-between gap-4 border-b border-borde px-4 py-3.5 sm:px-5">
                <div>
                  <h2
                    id="titulo-panel-cuentas-dividida"
                    className="text-base font-semibold tracking-tight text-texto"
                  >
                    Mis Cuentas
                  </h2>
                  <p className="text-xs text-suave">{cuentas.length} registradas</p>
                </div>
                <button
                  type="button"
                  onClick={() => abrirModalCuenta()}
                  className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-xl bg-acento px-3 text-xs font-semibold text-acento-fg shadow-xs transition hover:brightness-110 active:scale-[0.98]"
                >
                  <IconoMas width={14} height={14} />
                  <span>Nueva</span>
                </button>
              </div>

              {cuentasAdaptadas.length === 0 ? (
                <EstadoVacio onCrearCuenta={() => abrirModalCuenta()} />
              ) : (
                <ListaCuentas
                  cuentas={cuentasAdaptadas}
                  monedas={monedas}
                  onSeleccionarCuenta={(id) => setCuentaActivaId(id)}
                />
              )}
            </section>
          </div>

          {/* Panel Derecho: Detalle de Cuenta (8 cols) */}
          <div className="min-w-0 lg:col-span-8">{bloqueDetalleCuenta}</div>
        </div>
      ) : vistaSubpantalla === 'mis-cuentas' ? (
        <div className="grid items-start gap-6 md:grid-cols-[17.5rem_1fr]">
          {/* Tarjetas laterales de saldo por moneda */}
          <aside className="min-w-0">
            {subtotales.length > 0 ? (
              <SubtotalesPorMoneda
                subtotales={subtotales}
                monedas={monedas}
                cuentas={cuentasAdaptadas}
              />
            ) : (
              <div className="rounded-3xl border border-dashed border-borde bg-superficie p-5 text-sm text-suave shadow-suave">
                Aquí verás el saldo de cada moneda en cuanto crees tu primera cuenta.
              </div>
            )}
          </aside>

          {/* Panel principal de cuentas */}
          <section
            aria-labelledby="titulo-panel-cuentas"
            className="min-w-0 overflow-hidden rounded-3xl border border-borde bg-superficie shadow-suave"
          >
            <div className="flex items-center justify-between gap-4 border-b border-borde px-4 py-3.5 sm:px-6 sm:py-4">
              <div>
                <h2
                  id="titulo-panel-cuentas"
                  className="text-lg font-semibold tracking-tight text-texto"
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
                className="inline-flex h-10 cursor-pointer items-center gap-2 rounded-xl bg-acento px-4 text-xs font-semibold text-acento-fg shadow-sm transition hover:brightness-110 active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento sm:text-sm"
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
