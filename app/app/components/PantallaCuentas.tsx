'use client';

import React, { useState, useEffect, useCallback } from 'react';
import {
  listarCuentasCommand,
  detalleCuentaCommand,
  listarCategoriasCommand,
  crearCategoriaCommand,
  registrarMovimientoCommand,
  previsualizarMovimientoCommand,
  crearCuentaCommand,
  listarMonedasCommand,
  normalizarTipoCategoria,
  CuentaDto,
  DetalleCuentaDto,
  CategoriaDto,
  MovimientoDto,
  ErrorCampo,
  ErroresDto,
} from '../utils/tauriCommands';
import { Movimiento } from '../types/cuentas';
import { invoke } from '@tauri-apps/api/core';
import { DialogoAdvertencia } from './DialogoAdvertencia';
import { DialogoCorregir } from './DialogoCorregir';
import { ListaMovimientos } from './ListaMovimientos';
import { FormularioCategoria } from './FormularioCategoria';
import { FormularioCuenta } from './FormularioCuenta';

async function corregirMovimientoCommand(
  movimientoId: string,
  montoCorrecto: string
): Promise<MovimientoDto> {
  return await invoke<MovimientoDto>('corregir_movimiento', {
    datos: {
      movimiento_id: movimientoId,
      monto_correcto: montoCorrecto,
    },
  });
}

function adaptarMovimiento(dto: MovimientoDto): Movimiento {
  return {
    id: dto.id,
    cuenta_id: (dto as any).cuenta_id || '',
    tipo: dto.tipo as 'ingreso' | 'gasto',
    monto: String(dto.monto),
    monto_formateado: (dto as any).monto_con_signo || String(dto.monto),
    fecha: dto.fecha,
    categoria_id: (dto as any).categoria_id || '',
    categoria_nombre: dto.categoria || (dto as any).categoria_nombre || '',
    nota: (dto as any).nota || dto.descripcion || '',
    es_corregido: (dto as any).estado === 'corregido',
    es_ajuste: (dto as any).estado === 'ajuste',
    movimiento_referenciado_id: (dto as any).corrige_a,
  };
}

type NuevaCuentaDatos = Parameters<typeof crearCuentaCommand>[0];

interface DatosCuentas {
  lista: CuentaDto[];
  categorias: CategoriaDto[];
}

async function obtenerCuentasYCategorias(): Promise<DatosCuentas> {
  let resLista: CuentaDto[] | { cuentas?: CuentaDto[] } | null = null;
  try {
    resLista = await listarCuentasCommand();
  } catch (e) {
    console.error('Error al listar cuentas:', e);
  }
  let resCat: CategoriaDto[] | null = null;
  try {
    resCat = await listarCategoriasCommand();
  } catch (e) {
    console.error('Error al listar categorías:', e);
  }
  const cuentasArray = Array.isArray(resLista) ? resLista : (resLista && 'cuentas' in resLista ? resLista.cuentas : []) || [];
  return {
    lista: cuentasArray,
    categorias: resCat || [],
  };
}

function extraerErrores(err: unknown): ErrorCampo[] | null {
  if (err && typeof err === 'object' && 'errores' in err) {
    const errores = (err as ErroresDto).errores;
    if (Array.isArray(errores)) return errores;
  }
  return null;
}

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
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuentaDto | null>(null);
  const [categorias, setCategorias] = useState<CategoriaDto[]>([]);
  const [monedasDisponibles, setMonedasDisponibles] = useState<
    Array<{ codigo: string; nombre: string; simbolo: string }>
  >([
    { codigo: 'VES', nombre: 'Bolívares', simbolo: 'Bs.' },
    { codigo: 'USD', nombre: 'Dólares', simbolo: '$' },
    { codigo: 'EUR', nombre: 'Euros', simbolo: '€' },
    { codigo: 'USDT', nombre: 'Tether', simbolo: '₮' },
  ]);

  const [mostrarModalCuenta, setMostrarModalCuenta] = useState(false);
  const [mostrarModalMovimiento, setMostrarModalMovimiento] = useState(false);

  const [tipoMov, setTipoMov] = useState<'gasto' | 'ingreso'>('gasto');
  const [montoMov, setMontoMov] = useState('');
  const [fechaMov, setFechaMov] = useState(new Date().toISOString().split('T')[0]);
  const [catMovId, setCatMovId] = useState('');
  const [notaMov, setNotaMov] = useState('');

  const [movimientoAEditar, setMovimientoAEditar] = useState<Movimiento | null>(null);
  const [mostrarAdvertencia, setMostrarAdvertencia] = useState(false);
  const [saldoAdvertencia, setSaldoAdvertencia] = useState('');

  const [erroresCuenta, setErroresCuenta] = useState<ErrorCampo[]>([]);
  const [erroresMov, setErroresMov] = useState<Record<string, string>>({});
  const [erroresCat, setErroresCat] = useState<Record<string, string>>({});
  const [filtroCat, setFiltroCat] = useState<'todas' | 'gasto' | 'ingreso'>('todas');

  const aplicarDatos = useCallback((datos: DatosCuentas) => {
    setCuentas(datos.lista);
    setCategorias(datos.categorias);
    setCuentaActivaId((previa) => previa || (datos.lista.length > 0 ? datos.lista[0].id : ''));
  }, []);

  const cargarCuentasYCategorias = useCallback(async () => {
    try {
      aplicarDatos(await obtenerCuentasYCategorias());
    } catch (e) {
      console.error('Error al consultar SQLite:', e);
    }
  }, [aplicarDatos]);

  const cargarDetalleCuenta = useCallback(async (id: string) => {
    if (!id) return;
    try {
      setDetalleCuenta(await detalleCuentaCommand(id));
    } catch (e) {
      console.error('Error al cargar detalle de cuenta:', e);
    }
  }, []);

  useEffect(() => {
    let vigente = true;
    obtenerCuentasYCategorias()
      .then((datos) => {
        if (vigente) aplicarDatos(datos);
      })
      .catch((e) => console.error('Error al consultar SQLite:', e));
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
      .catch((e) => console.error('Error al cargar detalle de cuenta:', e));
    return () => {
      vigente = false;
    };
  }, [cuentaActivaId]);

  const cargarMonedas = async () => {
    try {
      const res = await listarMonedasCommand();
      if (Array.isArray(res) && res.length > 0) {
        setMonedasDisponibles(
          res.map((m: any) => ({
            codigo: m.codigo,
            nombre: m.nombre,
            simbolo: m.simbolo,
          }))
        );
      }
    } catch (e) {
      console.error('Error al cargar monedas:', e);
    }
  };

  useEffect(() => {
    cargarMonedas();
  }, []);

  const seleccionarCuenta = (id: string) => {
    setCuentaActivaId(id);
    setVistaSubpantalla('detalle-cuenta');
  };

  const abrirModalMovimientoConTipo = (tipo: 'gasto' | 'ingreso') => {
    setTipoMov(tipo);
    setMontoMov('');
    setNotaMov('');
    setFechaMov(new Date().toISOString().split('T')[0]);
    setErroresMov({});

    const catsFiltradas = categorias.filter((c) => normalizarTipoCategoria(c.tipo) === tipo);
    setCatMovId(catsFiltradas.length > 0 ? catsFiltradas[0].id : '');

    setMostrarModalMovimiento(true);
  };

  const handleCrearCuenta = async (datos: NuevaCuentaDatos) => {
    setErroresCuenta([]);
    try {
      const nueva = await crearCuentaCommand(datos);
      setMostrarModalCuenta(false);
      await cargarCuentasYCategorias();
      if (nueva && nueva.id) {
        setCuentaActivaId(nueva.id);
      }
    } catch (err) {
      const errores = extraerErrores(err);
      if (errores) {
        setErroresCuenta(errores);
      } else {
        setErroresCuenta([{ campo: 'general', mensaje: typeof err === 'string' ? err : 'Error al crear la cuenta.' }]);
      }
    }
  };

  const handleAgregarMovimiento = async (e: React.FormEvent | null, confirmado = false) => {
    e?.preventDefault();
    setErroresMov({});

    if (!catMovId) {
      setErroresMov({ categoria: 'Elige una categoría.' });
      return;
    }

    if (!montoMov.trim()) {
      setErroresMov({ monto: 'Ingresa un monto válido.' });
      return;
    }

    const montoLimpio = parseFloat(montoMov.trim().replace(',', '.'));

    if (isNaN(montoLimpio) || montoLimpio <= 0) {
      setErroresMov({ monto: 'El monto debe ser un número positivo.' });
      return;
    }

    if (tipoMov === 'gasto' && !confirmado) {
      try {
        const prev = await previsualizarMovimientoCommand({
          cuenta_id: cuentaActivaId,
          tipo: tipoMov,
          monto: montoLimpio,
          fecha: fechaMov,
          categoria_id: catMovId,
          nota: notaMov,
        });
        if (prev?.deja_saldo_negativo) {
          setSaldoAdvertencia(prev.saldo_resultante);
          setMostrarAdvertencia(true);
          return;
        }
      } catch {}
    }

    try {
      await registrarMovimientoCommand({
        cuenta_id: cuentaActivaId,
        tipo: tipoMov,
        monto: montoLimpio,
        fecha: fechaMov,
        categoria_id: catMovId,
        nota: notaMov,
      });

      setMontoMov('');
      setNotaMov('');
      setMostrarAdvertencia(false);
      setMostrarModalMovimiento(false);

      await cargarDetalleCuenta(cuentaActivaId);
      await cargarCuentasYCategorias();
    } catch (err) {
      const errores = extraerErrores(err);
      if (errores) {
        const mapa: Record<string, string> = {};
        errores.forEach((item) => {
          mapa[item.campo || 'monto'] = item.mensaje;
        });
        setErroresMov(mapa);
      } else {
        setErroresMov({ monto: typeof err === 'string' ? err : 'Error al registrar el movimiento.' });
      }
    }
  };

  const handleCorregirMovimiento = async (montoCorrecto: string) => {
    if (!movimientoAEditar) return;
    const montoLimpio = montoCorrecto.trim().replace(',', '.');

    try {
      await corregirMovimientoCommand(movimientoAEditar.id, montoLimpio);
      setMovimientoAEditar(null);
      await cargarDetalleCuenta(cuentaActivaId);
      await cargarCuentasYCategorias();
    } catch (err) {
      console.error('Error al corregir movimiento:', err);
    }
  };

  const handleCrearCategoria = async (nombre: string, tipo: 'gasto' | 'ingreso') => {
    setErroresCat({});
    try {
      await crearCategoriaCommand({ nombre, tipo });
      await cargarCuentasYCategorias();
    } catch (err) {
      const errores = extraerErrores(err);
      if (errores) {
        const mapa: Record<string, string> = {};
        errores.forEach((item) => {
          mapa[item.campo || 'nombre'] = item.mensaje;
        });
        setErroresCat(mapa);
      } else {
        const msg = typeof err === 'string' ? err : (err as any)?.message || 'Error al crear la categoría.';
        setErroresCat({ general: msg });
      }
    }
  };

  const cuentaSeleccionada = cuentas.find((c) => c.id === cuentaActivaId);
  const nombreCuentaReal = cuentaSeleccionada?.nombre || (detalleCuenta as any)?.cuenta?.nombre || 'Cargando...';
  const monedaCuentaReal = cuentaSeleccionada?.moneda || (detalleCuenta as any)?.cuenta?.moneda || '';

  const movimientosAdaptados: Movimiento[] = (detalleCuenta?.movimientos || []).map(adaptarMovimiento);
  const categoriasFiltradasModal = categorias.filter((c) => normalizarTipoCategoria(c.tipo) === tipoMov);
  const categoriasVisibles = categorias.filter((c) => {
    if (filtroCat === 'todas') return true;
    return normalizarTipoCategoria(c.tipo) === filtroCat;
  });

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
              onClick={() => setMostrarModalCuenta(true)}
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
                  onClick={() => setMostrarModalCuenta(true)}
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
                    <span className="text-2xl font-extrabold text-[#9333EA]">{c.saldo_actual}</span>
                  </div>
                </div>
              ))
            )}
          </div>
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
                  <span className="font-bold text-xs mt-1 block">{(detalleCuenta as any)?.composicion?.saldo_inicial ?? 0}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-emerald-950/30 text-emerald-400 border-emerald-900/30'
                      : 'bg-emerald-50 text-emerald-600 border-emerald-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Ingresos</span>
                  <span className="font-bold text-xs mt-1 block">+ {(detalleCuenta as any)?.composicion?.total_ingresos ?? 0}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-rose-950/30 text-rose-400 border-rose-900/30'
                      : 'bg-rose-50 text-rose-600 border-rose-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Gastos</span>
                  <span className="font-bold text-xs mt-1 block">− {(detalleCuenta as any)?.composicion?.total_gastos ?? 0}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-blue-950/30 text-blue-400 border-blue-900/30'
                      : 'bg-blue-50 text-blue-600 border-blue-100'
                  }`}
                >
                  <span className="block text-xs font-medium">Ajustes</span>
                  <span className="font-bold text-xs mt-1 block">{(detalleCuenta as any)?.composicion?.total_ajustes ?? 0}</span>
                </div>
                <div
                  className={`p-3 rounded-xl border ${
                    modoOscuro
                      ? 'bg-purple-950/40 text-purple-300 border-purple-900/40'
                      : 'bg-purple-50 text-[#9333EA] border-purple-100'
                  } font-bold`}
                >
                  <span className="block text-xs font-medium">Saldo Actual</span>
                  <span className="text-xs font-extrabold mt-1 block">{detalleCuenta.saldo_actual}</span>
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
                      onCorregir={(m) => setMovimientoAEditar(m)}
                      modoOscuro={modoOscuro}
                    />
                  </div>
                </div>
              </div>
            </div>

            <div className={`border rounded-2xl p-5 shadow-sm h-fit ${bgCard}`}>
              <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
              <FormularioCategoria
                onAgregar={(nombre, tipo) => handleCrearCategoria(nombre, tipo)}
                errores={erroresCat}
                modoOscuro={modoOscuro}
              />

              <div className="mt-6 pt-5 border-t border-gray-700/40">
                <div className="flex justify-between items-center mb-3">
                  <h4 className="text-xs font-bold uppercase tracking-wider">
                    Categorías ({categorias.length})
                  </h4>
                  <div className="flex gap-1">
                    <button
                      type="button"
                      onClick={() => setFiltroCat('todas')}
                      className={`text-[10px] px-2 py-0.5 rounded-md font-semibold transition cursor-pointer ${
                        filtroCat === 'todas'
                          ? 'bg-[#9333EA] text-white'
                          : modoOscuro
                          ? 'text-gray-400 hover:text-white'
                          : 'text-gray-500 hover:text-gray-900'
                      }`}
                    >
                      Todas
                    </button>
                    <button
                      type="button"
                      onClick={() => setFiltroCat('gasto')}
                      className={`text-[10px] px-2 py-0.5 rounded-md font-semibold transition cursor-pointer ${
                        filtroCat === 'gasto'
                          ? 'bg-rose-600 text-white'
                          : modoOscuro
                          ? 'text-gray-400 hover:text-white'
                          : 'text-gray-500 hover:text-gray-900'
                      }`}
                    >
                      Gastos
                    </button>
                    <button
                      type="button"
                      onClick={() => setFiltroCat('ingreso')}
                      className={`text-[10px] px-2 py-0.5 rounded-md font-semibold transition cursor-pointer ${
                        filtroCat === 'ingreso'
                          ? 'bg-emerald-600 text-white'
                          : modoOscuro
                          ? 'text-gray-400 hover:text-white'
                          : 'text-gray-500 hover:text-gray-900'
                      }`}
                    >
                      Ingresos
                    </button>
                  </div>
                </div>

                <div className="max-h-60 overflow-y-auto space-y-1.5 pr-1">
                  {categoriasVisibles.length === 0 ? (
                    <p className={`text-xs ${textMuted} italic py-2`}>No hay categorías en esta vista.</p>
                  ) : (
                    categoriasVisibles.map((cat) => (
                      <div
                        key={cat.id}
                        className={`flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs ${
                          modoOscuro ? 'bg-slate-800/60' : 'bg-gray-100/70'
                        }`}
                      >
                        <span className="font-medium truncate mr-2">{cat.nombre}</span>
                        <div className="flex items-center gap-1 shrink-0">
                          <span
                            className={`text-[9px] font-bold px-1.5 py-0.5 rounded ${
                              normalizarTipoCategoria(cat.tipo) === 'ingreso'
                                ? 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/40'
                                : 'bg-rose-950/60 text-rose-300 border border-rose-800/40'
                            }`}
                          >
                            {normalizarTipoCategoria(cat.tipo)}
                          </span>
                          {cat.de_fabrica && (
                            <span className="text-[9px] text-gray-400 bg-gray-700/30 px-1 py-0.5 rounded">
                              fábrica
                            </span>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
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
        movimiento={movimientoAEditar}
        onConfirmar={handleCorregirMovimiento}
        onCancelar={() => setMovimientoAEditar(null)}
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

            <form onSubmit={handleAgregarMovimiento} className="space-y-4">
              <div className="flex gap-4 text-xs font-medium">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="tipo"
                    checked={tipoMov === 'gasto'}
                    onChange={() => {
                      setTipoMov('gasto');
                      const cats = categorias.filter((c) => normalizarTipoCategoria(c.tipo) === 'gasto');
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
                      const cats = categorias.filter((c) => normalizarTipoCategoria(c.tipo) === 'ingreso');
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
              monedas={monedasDisponibles}
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