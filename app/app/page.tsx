'use client';

import React, { useState, useEffect } from 'react';
import {
  listarCuentasCommand,
  crearCuentaCommand,
  detalleCuentaCommand,
  listarCategoriasCommand,
  crearCategoriaCommand,
  registrarMovimientoCommand,
  previsualizarMovimientoCommand,
  CuentaDto,
  DetalleCuentaDto,
  CategoriaDto,
  MovimientoDto,
  ErroresDto,
} from './utils/tauriCommands';
import { invoke } from '@tauri-apps/api/core';
import { DialogoAdvertencia } from './components/DialogoAdvertencia';

// Comando de corrección mapeado a Tauri
async function corregirMovimientoCommand(movimientoId: string, montoCorrecto: string): Promise<MovimientoDto> {
  return await invoke<MovimientoDto>('corregir_movimiento', {
    datos: {
      movimiento_id: movimientoId,
      monto_correcto: montoCorrecto,
    },
  });
}

export default function Home() {
  const [modoOscuro, setModoOscuro] = useState(true);
  const [pantallaActual, setPantallaActual] = useState<'mis-cuentas' | 'detalle-cuenta'>('mis-cuentas');
  const [cuentaActivaId, setCuentaActivaId] = useState<string>('');

  // Estados alimentados desde SQLite
  const [cuentas, setCuentas] = useState<CuentaDto[]>([]);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuentaDto | null>(null);
  const [categorias, setCategorias] = useState<CategoriaDto[]>([]);

  // Modales
  const [mostrarModalCuenta, setMostrarModalCuenta] = useState(false);
  const [nombreNuevaCuenta, setNombreNuevaCuenta] = useState('');
  const [tipoNuevaCuenta, setTipoNuevaCuenta] = useState('banco');
  const [monedaNuevaCuenta, setMonedaNuevaCuenta] = useState('USD');
  const [saldoInicialNuevaCuenta, setSaldoInicialNuevaCuenta] = useState('0.00');

  const [mostrarModalMovimiento, setMostrarModalMovimiento] = useState(false);

  // Formulario Nueva Categoría
  const [nuevaCatNombre, setNuevaCatNombre] = useState('');
  const [nuevaCatTipo, setNuevaCatTipo] = useState<'gasto' | 'ingreso'>('gasto');

  // Formulario Nuevo Movimiento
  const [tipoMov, setTipoMov] = useState<'gasto' | 'ingreso'>('gasto');
  const [montoMov, setMontoMov] = useState('');
  const [fechaMov, setFechaMov] = useState(new Date().toISOString().split('T')[0]);
  const [catMovId, setCatMovId] = useState('');
  const [notaMov, setNotaMov] = useState('');

  // Modal Corregir Movimiento
  const [movimientoAEditar, setMovimientoAEditar] = useState<MovimientoDto | null>(null);
  const [montoCorrectoInput, setMontoCorrectoInput] = useState('');
  const [errorCorreccion, setErrorCorreccion] = useState<string>('');

  // Errores por campo
  const [erroresCuenta, setErroresCuenta] = useState<Record<string, string>>({});
  const [erroresMov, setErroresMov] = useState<Record<string, string>>({});
  const [mostrarAdvertencia, setMostrarAdvertencia] = useState(false);
  const [saldoAdvertencia, setSaldoAdvertencia] = useState('');
  const [erroresCat, setErroresCat] = useState<Record<string, string>>({});

  // Carga de cuentas y categorías. La base guarda el catálogo de fábrica como eventos (ids cat-fabrica-*);
  // el núcleo además devuelve una copia interna (de_fabrica = true) con otros ids que la base no acepta.
  // Solo se muestran las del catálogo guardado, para no repetirlas ni elegir un id inválido.
  const cargarCuentasYCategorias = async () => {
    try {
      const resLista = await listarCuentasCommand();
      const resCat = await listarCategoriasCommand();

      const lista = resLista.cuentas || [];
      setCuentas(lista);

      const delCatalogoGuardado = (resCat || []).filter((c) => !c.de_fabrica);
      setCategorias(delCatalogoGuardado);

      if (lista.length > 0 && !cuentaActivaId) {
        setCuentaActivaId(lista[0].id);
      }
    } catch (e) {
      console.error('Error al consultar SQLite:', e);
    }
  };

  const cargarDetalleCuenta = async (id: string) => {
    if (!id) return;
    try {
      const detalle = await detalleCuentaCommand(id);
      setDetalleCuenta(detalle);
    } catch (e) {
      console.error('Error al cargar detalle de cuenta:', e);
    }
  };

  useEffect(() => {
    cargarCuentasYCategorias();
  }, []);

  useEffect(() => {
    if (cuentaActivaId) {
      cargarDetalleCuenta(cuentaActivaId);
    }
  }, [cuentaActivaId]);

  const seleccionarCuenta = (id: string) => {
    setCuentaActivaId(id);
    setPantallaActual('detalle-cuenta');
  };

  // Abrir modal seleccionando solo entre tus categorías creadas
  const abrirModalMovimientoConTipo = (tipo: 'gasto' | 'ingreso') => {
    setTipoMov(tipo);
    setMontoMov('');
    setNotaMov('');
    setFechaMov(new Date().toISOString().split('T')[0]);
    setErroresMov({});

    const misCats = categorias.filter((c) => c.tipo === tipo && !c.de_fabrica);
    setCatMovId(misCats.length > 0 ? misCats[0].id : '');

    setMostrarModalMovimiento(true);
  };

  // Crear Cuenta
  const handleCrearCuenta = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresCuenta({});

    const saldoLimpio = saldoInicialNuevaCuenta.trim().replace(',', '.');

    try {
      const nueva = await crearCuentaCommand({
        nombre: nombreNuevaCuenta.trim(),
        tipo: tipoNuevaCuenta,
        moneda: monedaNuevaCuenta,
        saldo_inicial: saldoLimpio,
      });

      setNombreNuevaCuenta('');
      setSaldoInicialNuevaCuenta('0.00');
      setMostrarModalCuenta(false);

      await cargarCuentasYCategorias();
      if (nueva && nueva.id) {
        setCuentaActivaId(nueva.id);
      }
    } catch (err: any) {
      const dtoErr = err as ErroresDto;
      if (dtoErr && dtoErr.errores) {
        const mapa: Record<string, string> = {};
        dtoErr.errores.forEach((item) => {
          mapa[item.campo || 'general'] = item.mensaje;
        });
        setErroresCuenta(mapa);
      } else {
        setErroresCuenta({ general: typeof err === 'string' ? err : 'Error al crear la cuenta.' });
      }
    }
  };

  // Registrar Movimiento
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

    const montoLimpio = montoMov.trim().replace(',', '.');

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
        if (prev.deja_saldo_negativo) {
          setSaldoAdvertencia(prev.saldo_resultante);
          setMostrarAdvertencia(true);
          return;
        }
      } catch {
        // Los errores por campo los muestra el propio registro.
      }
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
    } catch (err: any) {
      console.error('Error al registrar movimiento:', err);

      if (err && typeof err === 'object' && err.errores) {
        const mapa: Record<string, string> = {};
        (err as ErroresDto).errores.forEach((item) => {
          mapa[item.campo || 'monto'] = item.mensaje;
        });
        setErroresMov(mapa);
      } else {
        const mensajeStr = typeof err === 'string' ? err : 'Error al registrar el movimiento.';
        setErroresMov({ monto: mensajeStr });
      }
    }
  };

  // Corregir Movimiento
  const handleCorregirMovimientoSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!movimientoAEditar) return;
    setErrorCorreccion('');

    const montoLimpio = montoCorrectoInput.trim().replace(',', '.');

    try {
      await corregirMovimientoCommand(movimientoAEditar.id, montoLimpio);

      setMovimientoAEditar(null);
      setMontoCorrectoInput('');

      await cargarDetalleCuenta(cuentaActivaId);
      await cargarCuentasYCategorias();
    } catch (err: any) {
      const dtoErr = err as ErroresDto;
      if (dtoErr && dtoErr.errores && dtoErr.errores.length > 0) {
        setErrorCorreccion(dtoErr.errores[0].mensaje);
      } else {
        setErrorCorreccion(typeof err === 'string' ? err : 'Error al corregir el movimiento.');
      }
    }
  };

  // Crear Categoría
  const handleAgregarCategoria = async (e: React.FormEvent) => {
    e.preventDefault();
    setErroresCat({});

    try {
      const catCreada = await crearCategoriaCommand(nuevaCatNombre.trim(), nuevaCatTipo);

      if (catCreada) {
        await cargarCuentasYCategorias();
        setTipoMov(catCreada.tipo as 'gasto' | 'ingreso');
        setCatMovId(catCreada.id);
      }

      setNuevaCatNombre('');
    } catch (err: any) {
      const dtoErr = err as ErroresDto;
      if (dtoErr && dtoErr.errores) {
        const mapa: Record<string, string> = {};
        dtoErr.errores.forEach((item) => {
          mapa[item.campo || 'nombre'] = item.mensaje;
        });
        setErroresCat(mapa);
      } else {
        setErroresCat({ nombre: typeof err === 'string' ? err : 'Error al registrar la categoría.' });
      }
    }
  };

  const cuentaSeleccionada = cuentas.find((c) => c.id === cuentaActivaId);
  const nombreCuentaReal = cuentaSeleccionada?.nombre || detalleCuenta?.cuenta.nombre || 'Cargando...';
  const monedaCuentaReal = cuentaSeleccionada?.moneda || detalleCuenta?.cuenta.moneda || '';

  // Filtro que excluye explícitamente cualquier categoría predeterminada
  const misCategoriasFiltradas = categorias.filter((c) => c.tipo === tipoMov && !c.de_fabrica);

  const bgMain = modoOscuro ? 'bg-[#0B0F19] text-white' : 'bg-[#F6F5FA] text-slate-900';
  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800/80 text-white' : 'bg-white border-gray-200 text-slate-900';
  const bgInput = modoOscuro
    ? 'bg-[#0F1420] border-gray-700 text-white placeholder-gray-500 appearance-none bg-[url("data:image/svg+xml;charset=UTF-8,%3csvg%20xmlns=\'http://www.w3.org/2000/svg\'%20viewBox=\'0%200%2024%2024\'%20fill=\'none\'%20stroke=\'%23A1A1AA\'%20stroke-width=\'2\'%20stroke-linecap=\'round\'%20stroke-linejoin=\'round\'%3e%3cpolyline%20points=\'6%209%2012%2015%2018%209\'%3e%3c/polyline%3e%3c/svg%3e")] bg-[length:1.25rem] bg-[right_0.75rem_center] bg-no-repeat pr-9 [&>option]:bg-[#0F1420] [&>option]:text-white'
    : 'bg-white border-gray-200 text-slate-900 placeholder-gray-400 [&>option]:bg-white [&>option]:text-slate-900';
  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <div className={`flex min-h-screen w-full font-sans transition-colors duration-200 ${bgMain}`}>
      {/* MENÚ LATERAL */}
      <aside className="w-[224px] min-h-screen bg-[#211337] text-white flex flex-col justify-between p-6 shrink-0">
        <div>
          <h1 className="text-xl font-bold tracking-tight mb-8 text-white">Cuentas Claras</h1>
          <nav className="space-y-2">
            <button
              onClick={() => setPantallaActual('mis-cuentas')}
              className={`w-full text-left px-4 py-2.5 rounded-xl font-medium text-sm transition ${
                pantallaActual === 'mis-cuentas' ? 'bg-[#9333EA] text-white shadow-lg' : 'text-purple-200 hover:bg-white/10'
              }`}
            >
              Mis cuentas
            </button>
            <button
              onClick={() => setPantallaActual('detalle-cuenta')}
              className={`w-full text-left px-4 py-2.5 rounded-xl font-medium text-sm transition ${
                pantallaActual === 'detalle-cuenta' ? 'bg-[#9333EA] text-white shadow-lg' : 'text-purple-200 hover:bg-white/10'
              }`}
            >
              Detalle de Cuenta
            </button>
          </nav>
        </div>

        <div className="pt-4 border-t border-purple-900/40 flex items-center justify-between text-xs">
          <span className="text-purple-300">{modoOscuro ? 'Oscuro' : 'Claro'}</span>
          <button
            onClick={() => setModoOscuro(!modoOscuro)}
            className="px-3 py-1.5 bg-[#9333EA] text-white rounded-full font-semibold shadow"
          >
            {modoOscuro ? '🌙 Oscuro' : '☀️ Claro'}
          </button>
        </div>
      </aside>

      {/* ÁREA DE CONTENIDO */}
      <main className="flex-1 p-8 overflow-y-auto">
        {pantallaActual === 'mis-cuentas' ? (
          <div className="max-w-5xl mx-auto space-y-6">
            <div className="flex justify-between items-center">
              <div>
                <h2 className="text-2xl font-bold tracking-tight">Mis cuentas</h2>
                <p className={`text-xs mt-1 ${textMuted}`}>Selecciona una cuenta para abrir su detalle correspondiente</p>
              </div>
              <button
                onClick={() => setMostrarModalCuenta(true)}
                className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold shadow-md transition"
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
                    className="mt-3 text-xs text-[#9333EA] underline font-semibold"
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
                      c.id === cuentaActivaId ? 'border-purple-500/80 ring-1 ring-purple-500/30' : 'hover:border-purple-400'
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
                      <span className="text-2xl font-extrabold text-[#9333EA]">
                        {c.saldo_actual}
                      </span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        ) : (
          <div className="max-w-6xl mx-auto space-y-6">
            <div className="flex justify-between items-center">
              <div>
                <h2 className="text-2xl font-bold tracking-tight">
                  {nombreCuentaReal} ({monedaCuentaReal})
                </h2>
                <p className={`text-xs mt-0.5 ${textMuted}`}>ID de la cuenta: {cuentaActivaId}</p>
              </div>

              <div className="flex gap-2 items-center">
                <button
                  onClick={() => abrirModalMovimientoConTipo('gasto')}
                  className="bg-rose-600 hover:bg-rose-700 text-white px-3.5 py-1.5 rounded-xl text-xs font-semibold shadow transition"
                >
                  − Nuevo Gasto
                </button>
                <button
                  onClick={() => abrirModalMovimientoConTipo('ingreso')}
                  className="bg-emerald-600 hover:bg-emerald-700 text-white px-3.5 py-1.5 rounded-xl text-xs font-semibold shadow transition"
                >
                  + Nuevo Ingreso
                </button>

                {cuentas.length > 0 && (
                  <select
                    value={cuentaActivaId}
                    onChange={(e) => setCuentaActivaId(e.target.value)}
                    className={`border px-3 py-1.5 rounded-xl text-xs font-medium cursor-pointer ml-2 ${bgInput}`}
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

            {/* COMPOSICIÓN DEL SALDO REAL DESDE SQLITE */}
            {detalleCuenta && (
              <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
                <h3 className={`text-[11px] font-bold uppercase tracking-wider mb-3 ${textMuted}`}>Composición del Saldo</h3>
                <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-center">
                  <div className={`p-3 rounded-xl ${modoOscuro ? 'bg-slate-800/60' : 'bg-gray-100/70'}`}>
                    <span className={`block text-xs ${textMuted}`}>Saldo Inicial</span>
                    <span className="font-bold text-xs mt-1 block">{detalleCuenta.composicion.saldo_inicial}</span>
                  </div>
                  <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-emerald-950/30 text-emerald-400 border-emerald-900/30' : 'bg-emerald-50 text-emerald-600 border-emerald-100'}`}>
                    <span className="block text-xs font-medium">Ingresos</span>
                    <span className="font-bold text-xs mt-1 block">+ {detalleCuenta.composicion.total_ingresos}</span>
                  </div>
                  <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-rose-950/30 text-rose-400 border-rose-900/30' : 'bg-rose-50 text-rose-600 border-rose-100'}`}>
                    <span className="block text-xs font-medium">Gastos</span>
                    <span className="font-bold text-xs mt-1 block">− {detalleCuenta.composicion.total_gastos}</span>
                  </div>
                  <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-blue-950/30 text-blue-400 border-blue-900/30' : 'bg-blue-50 text-blue-600 border-blue-100'}`}>
                    <span className="block text-xs font-medium">Ajustes</span>
                    <span className="font-bold text-xs mt-1 block">{detalleCuenta.composicion.total_ajustes}</span>
                  </div>
                  <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-purple-950/40 text-purple-300 border-purple-900/40' : 'bg-purple-50 text-[#9333EA] border-purple-100'} font-bold`}>
                    <span className="block text-xs font-medium">Saldo Actual</span>
                    <span className="text-xs font-extrabold mt-1 block">{detalleCuenta.cuenta.saldo_actual}</span>
                  </div>
                </div>
              </div>
            )}

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2 space-y-6">
                <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
                  <h3 className="text-base font-bold mb-4">Historial de Movimientos</h3>
                  <div className="overflow-x-auto">
                    <table className="w-full text-left text-xs">
                      <thead>
                        <tr className={`border-b ${modoOscuro ? 'border-gray-800 text-gray-400' : 'border-gray-200 text-gray-500'}`}>
                          <th className="pb-3 font-semibold">Fecha</th>
                          <th className="pb-3 font-semibold">Categoría</th>
                          <th className="pb-3 font-semibold">Nota</th>
                          <th className="pb-3 font-semibold text-right">Monto</th>
                          <th className="pb-3 font-semibold text-center">Estado</th>
                          <th className="pb-3 font-semibold text-center">Acción</th>
                        </tr>
                      </thead>
                      <tbody className={`divide-y ${modoOscuro ? 'divide-gray-800' : 'divide-gray-200'}`}>
                        {!detalleCuenta?.movimientos || detalleCuenta.movimientos.length === 0 ? (
                          <tr>
                            <td colSpan={6} className="py-6 text-center text-gray-400">
                              No hay movimientos registrados para esta cuenta.
                            </td>
                          </tr>
                        ) : (
                          detalleCuenta.movimientos.map((m) => (
                            <tr key={m.id} className={modoOscuro ? 'hover:bg-slate-800/40' : 'hover:bg-gray-50/50'}>
                              <td className="py-3.5 text-xs">{m.fecha}</td>
                              <td className="py-3.5 font-medium">{m.categoria_nombre}</td>
                              <td className={`py-3.5 ${textMuted}`}>{m.nota || '—'}</td>
                              <td className={`py-3.5 font-bold text-right ${m.tipo === 'gasto' ? 'text-rose-400' : 'text-emerald-400'}`}>
                                {m.monto_con_signo}
                              </td>
                              <td className="py-3.5 text-center">
                                <span className={`text-[11px] px-2 py-0.5 rounded-full ${
                                  m.estado === 'corregido'
                                    ? 'bg-amber-950/60 text-amber-300 border border-amber-800/40'
                                    : m.estado === 'ajuste'
                                    ? 'bg-blue-950/60 text-blue-300 border border-blue-800/40'
                                    : 'text-gray-400'
                                }`}>
                                  {m.estado}
                                </span>
                              </td>
                              <td className="py-3.5 text-center">
                                {m.estado !== 'ajuste' && (
                                  <button
                                    onClick={() => {
                                      setMovimientoAEditar(m);
                                      setMontoCorrectoInput(m.monto || '');
                                      setErrorCorreccion('');
                                    }}
                                    className="text-[11px] text-purple-400 hover:text-purple-300 font-semibold underline"
                                  >
                                    Corregir
                                  </button>
                                )}
                              </td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>

              {/* NUEVA CATEGORÍA */}
              <div className={`border rounded-2xl p-5 shadow-sm h-fit ${bgCard}`}>
                <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
                <form onSubmit={handleAgregarCategoria} className="space-y-3">
                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Nombre</label>
                    <input
                      type="text"
                      placeholder="Ej. Cine, Mascotas"
                      value={nuevaCatNombre}
                      onChange={(e) => setNuevaCatNombre(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                    />
                    {erroresCat.nombre && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCat.nombre}</p>}
                  </div>

                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Tipo</label>
                    <select
                      value={nuevaCatTipo}
                      onChange={(e) => setNuevaCatTipo(e.target.value as 'gasto' | 'ingreso')}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer ${bgInput}`}
                    >
                      <option value="gasto">Gasto</option>
                      <option value="ingreso">Ingreso</option>
                    </select>
                    {erroresCat.tipo && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCat.tipo}</p>}
                  </div>

                  <button type="submit" className="w-full bg-[#9333EA] text-white py-2 rounded-xl text-xs font-semibold hover:bg-purple-600 transition shadow-md">
                    Añadir Categoría
                  </button>
                </form>
              </div>
            </div>
          </div>
        )}

        {/* MODAL REGISTRAR MOVIMIENTO */}
        <DialogoAdvertencia
          visible={mostrarAdvertencia}
          montoResultante={saldoAdvertencia}
          onConfirmar={() => {
            setMostrarAdvertencia(false);
            handleAgregarMovimiento(null, true);
          }}
          onCancelar={() => setMostrarAdvertencia(false)}
        />
        {mostrarModalMovimiento && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
            <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
              <div className="flex justify-between items-center mb-4">
                <h3 className="text-lg font-bold">
                  {tipoMov === 'gasto' ? 'Registrar Gasto' : 'Registrar Ingreso'}
                </h3>
                <span className={`text-xs px-2.5 py-1 rounded-md font-bold uppercase ${
                  tipoMov === 'gasto' ? 'bg-rose-950/60 text-rose-300 border border-rose-800/40' : 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/40'
                }`}>
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
                        const misCats = categorias.filter((c) => c.tipo === 'gasto' && !c.de_fabrica);
                        setCatMovId(misCats.length > 0 ? misCats[0].id : '');
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
                        const misCats = categorias.filter((c) => c.tipo === 'ingreso' && !c.de_fabrica);
                        setCatMovId(misCats.length > 0 ? misCats[0].id : '');
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
                    placeholder="Ej: 50,00 o 50.00"
                    value={montoMov}
                    onChange={(e) => setMontoMov(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                  {erroresMov.monto && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.monto}</p>}
                </div>

                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Fecha</label>
                  <input
                    type="date"
                    value={fechaMov}
                    onChange={(e) => setFechaMov(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                  {erroresMov.fecha && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.fecha}</p>}
                </div>

                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Categoría</label>
                  <select
                    value={catMovId}
                    onChange={(e) => setCatMovId(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 cursor-pointer ${bgInput}`}
                  >
                    {misCategoriasFiltradas.length === 0 ? (
                      <option value="">No has creado categorías de ({tipoMov}) aún...</option>
                    ) : (
                      <option value="">Seleccione categoría...</option>
                    )}
                    {misCategoriasFiltradas.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.nombre}
                      </option>
                    ))}
                  </select>
                  {erroresMov.categoria && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresMov.categoria}</p>}
                </div>

                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Nota (opcional)</label>
                  <input
                    type="text"
                    placeholder="Ej. Salida de fin de semana"
                    value={notaMov}
                    onChange={(e) => setNotaMov(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                </div>

                <div className="flex gap-3 justify-end pt-2">
                  <button
                    type="button"
                    onClick={() => {
                      setMostrarModalMovimiento(false);
                      setErroresMov({});
                    }}
                    className={`px-4 py-2 rounded-xl text-xs font-semibold border ${modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'}`}
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold"
                  >
                    Guardar
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}

        {/* MODAL CORREGIR MOVIMIENTO */}
        {movimientoAEditar && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
            <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
              <h3 className="text-lg font-bold mb-2">Corregir Movimiento</h3>
              <p className={`text-xs mb-4 ${textMuted}`}>
                Indica el monto correcto para esta operación ({movimientoAEditar.categoria_nombre}). El sistema registrará un movimiento de ajuste por la diferencia.
              </p>

              <form onSubmit={handleCorregirMovimientoSubmit} className="space-y-4">
                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Monto Registrado Anteriormente</label>
                  <input
                    type="text"
                    disabled
                    value={movimientoAEditar.monto}
                    className="w-full border px-3 py-2 rounded-xl text-xs bg-gray-800/50 text-gray-400 border-gray-700 cursor-not-allowed"
                  />
                </div>

                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Monto Correcto Real</label>
                  <input
                    type="text"
                    placeholder="Ej. 50.00 o 0 (para anular)"
                    value={montoCorrectoInput}
                    onChange={(e) => setMontoCorrectoInput(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                  {errorCorreccion && <p className="text-rose-400 text-[11px] font-medium mt-1">{errorCorreccion}</p>}
                </div>

                <div className="flex gap-3 justify-end pt-2">
                  <button
                    type="button"
                    onClick={() => {
                      setMovimientoAEditar(null);
                      setErrorCorreccion('');
                    }}
                    className={`px-4 py-2 rounded-xl text-xs font-semibold border ${modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'}`}
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold"
                  >
                    Guardar Ajuste
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}

        {/* MODAL CREAR CUENTA */}
        {mostrarModalCuenta && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
            <div className={`w-full max-w-md p-6 rounded-2xl border shadow-xl ${bgCard}`}>
              <h3 className="text-lg font-bold mb-4">Nueva Cuenta</h3>
              <form onSubmit={handleCrearCuenta} className="space-y-4">
                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Nombre de la cuenta</label>
                  <input
                    type="text"
                    placeholder="Ej. Banesco, BofA, Efectivo"
                    value={nombreNuevaCuenta}
                    onChange={(e) => setNombreNuevaCuenta(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                  {erroresCuenta.nombre && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCuenta.nombre}</p>}
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Tipo</label>
                    <select
                      value={tipoNuevaCuenta}
                      onChange={(e) => setTipoNuevaCuenta(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer ${bgInput}`}
                    >
                      <option value="efectivo">Efectivo</option>
                      <option value="banco">Banco</option>
                      <option value="billetera_digital">Digital</option>
                    </select>
                    {erroresCuenta.tipo && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCuenta.tipo}</p>}
                  </div>
                  <div>
                    <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Moneda</label>
                    <select
                      value={monedaNuevaCuenta}
                      onChange={(e) => setMonedaNuevaCuenta(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer ${bgInput}`}
                    >
                      <option value="USD">USD ($)</option>
                      <option value="VES">VES (Bs.)</option>
                      <option value="EUR">EUR (€)</option>
                      <option value="USDT">USDT (₮)</option>
                    </select>
                    {erroresCuenta.moneda && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCuenta.moneda}</p>}
                  </div>
                </div>

                <div>
                  <label className={`block text-xs font-semibold mb-1 ${textMuted}`}>Saldo Inicial</label>
                  <input
                    type="text"
                    placeholder="0,00 o 0.00"
                    value={saldoInicialNuevaCuenta}
                    onChange={(e) => setSaldoInicialNuevaCuenta(e.target.value)}
                    className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                  />
                  {erroresCuenta.saldo_inicial && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCuenta.saldo_inicial}</p>}
                </div>

                {erroresCuenta.general && <p className="text-rose-400 text-[11px] font-medium mt-1">{erroresCuenta.general}</p>}

                <div className="flex gap-3 justify-end pt-2">
                  <button
                    type="button"
                    onClick={() => {
                      setMostrarModalCuenta(false);
                      setErroresCuenta({});
                    }}
                    className={`px-4 py-2 rounded-xl text-xs font-semibold border ${modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'}`}
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    className="bg-[#9333EA] hover:bg-purple-600 text-white px-4 py-2 rounded-xl text-xs font-semibold"
                  >
                    Crear Cuenta
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}