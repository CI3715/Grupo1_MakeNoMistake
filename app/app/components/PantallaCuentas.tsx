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
import { Movimiento } from '../types/cuentas';
import { DialogoAdvertencia } from './DialogoAdvertencia';
import { DialogoCorregir } from './DialogoCorregir';
import { ListaMovimientos } from './ListaMovimientos';
import { FormularioCategoria } from './FormularioCategoria';
import { FormularioCuenta } from './FormularioCuenta';

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
}

export function PantallaCuentas({
  modoOscuro,
  vistaSubpantalla,
  setVistaSubpantalla,
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
  }, [aplicarDatos]);

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
  }, [cuentaActivaId]);

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

    const catsFiltradas = categorias.filter((c) => c.tipo === tipo);
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

  const cuentaSeleccionada = cuentas.find((c) => c.id === cuentaActivaId);
  const nombreCuentaReal = cuentaSeleccionada?.nombre ?? detalleCuenta?.cuenta.nombre ?? 'Cargando...';
  const monedaCuentaReal = cuentaSeleccionada?.moneda ?? detalleCuenta?.cuenta.moneda ?? '';

  const movimientosAdaptados: Movimiento[] = (detalleCuenta?.movimientos ?? []).map((m) =>
    adaptarMovimiento(m, monedaCuentaReal)
  );
  const categoriasFiltradasModal = categorias.filter((c) => c.tipo === tipoMov);
  const otrosErroresMov = Object.entries(erroresMov)
    .filter(([campo]) => !CAMPOS_MOVIMIENTO.includes(campo))
    .map(([, mensaje]) => mensaje);

  const bgCard = modoOscuro
    ? 'bg-[#151C28] border-gray-800/80 text-white'
    : 'bg-white border-gray-200 text-slate-900';

  const estiloInput = modoOscuro
    ? { backgroundColor: '#0F1420', color: '#ffffff', borderColor: '#374151', colorScheme: 'dark' }
    : { backgroundColor: '#ffffff', color: '#0f172a', borderColor: '#e2e8f0', colorScheme: 'light' };

  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <>
      {vistaSubpantalla === 'mis-cuentas' ? (
        <div className="max-w-5xl mx-auto space-y-6">
          <div className="flex justify-between items-center">
            <div>
              <h2 className="text-2xl font-bold tracking-tight">Mis cuentas</h2>
              <p className={`text-xs mt-1 ${textMuted}`}>
                Selecciona una cuenta para abrir su detalle correspondiente
              </p>
            </div>
            <button
              onClick={() => abrirModalCuenta()}
              className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold shadow-md transition cursor-pointer"
            >
              + Nueva Cuenta
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {cuentas.length === 0 ? (
              <div className={`p-8 rounded-2xl border text-center col-span-2 ${bgCard}`}>
                <p className="text-sm font-medium">No existen cuentas registradas en la base de datos.</p>
                <button
                  onClick={() => abrirModalCuenta()}
                  className="mt-3 text-xs text-[#9333EA] underline font-semibold cursor-pointer"
                >
                  Crear la primera cuenta
                </button>
              </div>
            ) : (
              cuentas.map((c) => (
                <div
                  key={c.id}
                  onClick={() => seleccionarCuenta(c.id)}
                  className={`p-6 rounded-2xl cursor-pointer transition-all border shadow-sm ${bgCard} ${
                    c.id === cuentaActivaId
                      ? 'border-purple-500/80 ring-1 ring-purple-500/30'
                      : 'hover:border-purple-400'
                  }`}
                >
                  <div className="flex justify-between items-start">
                    <div>
                      <h3 className="text-lg font-bold">{c.nombre}</h3>
                      <p className={`text-xs mt-1 ${textMuted}`}>Tipo: {c.tipo}</p>
                    </div>
                    <span className="bg-purple-950/60 text-purple-300 border border-purple-800/40 text-[10px] font-bold px-2.5 py-1 rounded-md">
                      {c.moneda}
                    </span>
                  </div>

                  <div className="mt-8 flex justify-between items-baseline">
                    <span className={`text-xs font-medium ${textMuted}`}>Saldo actual</span>
                    <span className="text-2xl font-extrabold text-[#9333EA]">{formatearMontoTexto(c.saldo_actual, c.moneda)}</span>
                  </div>
                </div>
              ))
            )}
          </div>

          {subtotales.length > 0 && (
            <div className={`p-5 rounded-2xl border shadow-sm ${bgCard}`}>
              <h3 className={`text-[11px] font-bold uppercase tracking-wider mb-3 ${textMuted}`}>
                Subtotal por moneda
              </h3>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                {subtotales.map((s) => (
                  <div key={s.moneda} className="p-3 rounded-xl bg-gray-500/10 border border-gray-500/10">
                    <span className="text-xs text-gray-400 font-bold block uppercase">{s.moneda}</span>
                    <span className="text-sm font-extrabold">{formatearMontoTexto(s.total, s.moneda)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="w-full space-y-6">
          <div className="flex justify-between items-center flex-wrap gap-3">
            <div>
              <h2 className="text-2xl font-bold tracking-tight">
                {nombreCuentaReal} ({monedaCuentaReal})
              </h2>
              <p className={`text-xs mt-0.5 ${textMuted}`}>ID de la cuenta: {cuentaActivaId}</p>
            </div>

            <div className="flex gap-2 items-center flex-wrap">
              <button
                onClick={() => abrirModalMovimientoConTipo('gasto')}
                className="bg-rose-600 hover:bg-rose-700 text-white px-3.5 py-1.5 rounded-xl text-xs font-semibold shadow transition cursor-pointer"
              >
                − Nuevo Gasto
              </button>
              <button
                onClick={() => abrirModalMovimientoConTipo('ingreso')}
                className="bg-emerald-600 hover:bg-emerald-700 text-white px-3.5 py-1.5 rounded-xl text-xs font-semibold shadow transition cursor-pointer"
              >
                + Nuevo Ingreso
              </button>

              {cuentas.length > 0 && (
                <select
                  value={cuentaActivaId}
                  onChange={(e) => setCuentaActivaId(e.target.value)}
                  style={estiloInput}
                  className="border px-3 py-1.5 rounded-xl text-xs font-medium cursor-pointer ml-2 focus:outline-none"
                >
                  {cuentas.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.nombre} ({c.moneda})
                    </option>
                  ))}
                </select>
              )}
            </div>
          </div>

          {detalleCuenta && (
            <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
              <h3 className={`text-[11px] font-bold uppercase tracking-wider mb-3 ${textMuted}`}>
                Composición del Saldo
              </h3>
              <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-center">
                <div className={`p-3 rounded-xl ${modoOscuro ? 'bg-slate-800/60' : 'bg-gray-100/70'}`}>
                  <span className={`block text-xs ${textMuted}`}>Saldo Inicial</span>
                  <span className="font-bold text-xs mt-1 block">{formatearMontoTexto(detalleCuenta.composicion.saldo_inicial, monedaCuentaReal)}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-emerald-950/30 text-emerald-400 border-emerald-900/30'
                      : 'bg-emerald-50 text-emerald-600 border-emerald-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Ingresos</span>
                  <span className="font-bold text-xs mt-1 block">{formatearMontoTexto(detalleCuenta.composicion.total_ingresos, monedaCuentaReal)}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-rose-950/30 text-rose-400 border-rose-900/30'
                      : 'bg-rose-50 text-rose-600 border-rose-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Gastos</span>
                  <span className="font-bold text-xs mt-1 block">{formatearMontoTexto(detalleCuenta.composicion.total_gastos, monedaCuentaReal)}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-blue-950/30 text-blue-400 border-blue-900/30'
                      : 'bg-blue-50 text-blue-600 border-blue-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Ajustes</span>
                  <span className="font-bold text-xs mt-1 block">{formatearMontoTexto(detalleCuenta.composicion.total_ajustes, monedaCuentaReal)}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-purple-950/40 text-purple-300 border-purple-900/40'
                      : 'bg-purple-50 text-[#9333EA] border-purple-100'
                  } font-bold`}
                >
                  <span className="block text-xs font-medium">Saldo Actual</span>
                  <span className="text-xs font-extrabold mt-1 block">{formatearMontoTexto(detalleCuenta.cuenta.saldo_actual, monedaCuentaReal)}</span>
                </div>
              </div>
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2 space-y-6">
              <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
                <h3 className="text-base font-bold mb-4">Historial de Movimientos</h3>
                <div className="w-full overflow-x-auto">
                  <div className="min-w-[620px]">
                    <ListaMovimientos
                      movimientos={movimientosAdaptados}
                      onCorregir={(m) => {
                        setErrorCorreccion(null);
                        setMovimientoAEditar(m);
                      }}
                      modoOscuro={modoOscuro}
                    />
                  </div>
                </div>
              </div>
            </div>

            <div className={`border rounded-2xl p-5 shadow-sm h-fit ${bgCard}`}>
              <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
              <FormularioCategoria
                onAgregar={handleCrearCategoria}
                errores={erroresCat}
                modoOscuro={modoOscuro}
              />
            </div>
          </div>
        </div>
      )}

      <DialogoAdvertencia
        visible={mostrarAdvertencia}
        montoResultante={saldoAdvertencia}
        onConfirmar={() => {
          setMostrarAdvertencia(false);
          handleAgregarMovimiento(null, true);
        }}
        onCancelar={() => setMostrarAdvertencia(false)}
      />

      <DialogoCorregir
        key={movimientoAEditar?.id ?? 'cerrado'}
        movimiento={movimientoAEditar}
        onConfirmar={handleCorregirMovimiento}
        onCancelar={() => setMovimientoAEditar(null)}
        error={errorCorreccion}
      />

      {mostrarModalMovimiento && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
          <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-lg font-bold">
                {tipoMov === 'gasto' ? 'Registrar Gasto' : 'Registrar Ingreso'}
              </h3>
              <span
                className={`text-xs px-2.5 py-1 rounded-md font-bold uppercase ${
                  tipoMov === 'gasto'
                    ? 'bg-rose-950/60 text-rose-300 border border-rose-800/40'
                    : 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/40'
                }`}
              >
                {tipoMov}
              </span>
            </div>

            {otrosErroresMov.length > 0 && (
              <div className="mb-4 p-3 rounded-xl border border-rose-800/60 bg-rose-950/40 text-rose-300 text-xs space-y-1">
                {otrosErroresMov.map((m, i) => (
                  <p key={i}>{m}</p>
                ))}
              </div>
            )}

            <form onSubmit={handleAgregarMovimiento} className="space-y-4">
              <div className="flex gap-4 text-xs font-medium">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="tipo"
                    checked={tipoMov === 'gasto'}
                    onChange={() => {
                      setTipoMov('gasto');
                      const cats = categorias.filter((c) => c.tipo === 'gasto');
                      setCatMovId(cats.length > 0 ? cats[0].id : '');
                    }}
                    className="accent-[#9333EA]"
                  />
                  Gasto
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="tipo"
                    checked={tipoMov === 'ingreso'}
                    onChange={() => {
                      setTipoMov('ingreso');
                      const cats = categorias.filter((c) => c.tipo === 'ingreso');
                      setCatMovId(cats.length > 0 ? cats[0].id : '');
                    }}
                    className="accent-[#9333EA]"
                  />
                  Ingreso
                </label>
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Monto</label>
                <input
                  type="text"
                  placeholder="Ej: 50,00"
                  value={montoMov}
                  onChange={(e) => setMontoMov(e.target.value)}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
                />
                {erroresMov.monto && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.monto}</p>}
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Fecha</label>
                <input
                  type="date"
                  value={fechaMov}
                  onChange={(e) => setFechaMov(e.target.value)}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
                />
                {erroresMov.fecha && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.fecha}</p>}
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Categoría</label>
                <select
                  value={catMovId}
                  onChange={(e) => setCatMovId(e.target.value)}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer"
                >
                  {categoriasFiltradasModal.length === 0 ? (
                    <option value="">No hay categorías disponibles...</option>
                  ) : (
                    <option value="">Seleccione categoría...</option>
                  )}
                  {categoriasFiltradasModal.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.nombre}
                    </option>
                  ))}
                </select>
                {erroresMov.categoria && (
                  <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.categoria}</p>
                )}
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Nota (opcional)</label>
                <input
                  type="text"
                  placeholder="Ej. Mercado del fin de semana"
                  value={notaMov}
                  onChange={(e) => setNotaMov(e.target.value)}
                  style={estiloInput}
                  className="w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
                />
              </div>

              <div className="flex gap-3 justify-end pt-2">
                <button
                  type="button"
                  onClick={() => {
                    setMostrarModalMovimiento(false);
                    setErroresMov({});
                  }}
                  className={`px-4 py-2 rounded-xl text-xs font-semibold border ${
                    modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'
                  }`}
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer"
                >
                  Guardar
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {mostrarModalCuenta && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
          <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
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