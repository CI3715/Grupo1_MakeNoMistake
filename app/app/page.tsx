'use client';

import React, { useState } from 'react';

export default function Home() {
  const [modoOscuro, setModoOscuro] = useState(true);
  const [pantallaActual, setPantallaActual] = useState<'mis-cuentas' | 'detalle-cuenta'>('detalle-cuenta');
  const [cuentaActivaId, setCuentaActivaId] = useState('c1');

  // Estado de Categorías
  const [categorias, setCategorias] = useState({
    ingreso: ['Salario', 'Ventas', 'Transferencia'],
    gasto: ['Alimentación', 'Supermercado', 'Servicios', 'Transporte', 'Ocio'],
  });

  const [nuevaCatNombre, setNuevaCatNombre] = useState('');
  const [nuevaCatTipo, setNuevaCatTipo] = useState<'gasto' | 'ingreso'>('gasto');

  // Estado de Cuentas
  const cuentas = [
    { id: 'c1', nombre: 'Billetera Dólares', tipo: 'Billetera', moneda: 'USD', simbolo: '$', saldo: '1.750,00' },
    { id: 'c2', nombre: 'Cuenta Banco Bs.', tipo: 'Banco', moneda: 'VES', simbolo: 'Bs.', saldo: '12.500,00' },
  ];

  // Estado de Movimientos
  const [movimientos, setMovimientos] = useState([
    { id: 'm1', fecha: '2026-10-19', categoria: 'Salario', nota: 'Pago quincena', monto: '+ $ 800,00', tipo: 'ingreso', relacion: 'Normal' },
    { id: 'm2', fecha: '2026-10-19', categoria: 'Alimentación', nota: 'Mercado (se anotó mal)', monto: '− $ 500,00', tipo: 'gasto', relacion: 'Corregido' },
    { id: 'm3', fecha: '2026-10-19', categoria: 'Alimentación', nota: 'Ajuste corrección m2', monto: '$ + 450,00', tipo: 'ingreso', relacion: 'Ajuste (Ref: m2)' },
  ]);

  // Formulario de Movimientos
  const [tipoMov, setTipoMov] = useState<'gasto' | 'ingreso'>('gasto');
  const [montoMov, setMontoMov] = useState('');
  const [fechaMov, setFechaMov] = useState('2026-10-20');
  const [catMov, setCatMov] = useState('');
  const [notaMov, setNotaMov] = useState('');

  const handleAgregarCategoria = (e: React.FormEvent) => {
    e.preventDefault();
    if (!nuevaCatNombre.trim()) return;
    setCategorias((prev) => ({
      ...prev,
      [nuevaCatTipo]: [...prev[nuevaCatTipo], nuevaCatNombre.trim()],
    }));
    setNuevaCatNombre('');
  };

  const handleAgregarMovimiento = (e: React.FormEvent) => {
    e.preventDefault();
    if (!montoMov || !catMov) return;
    const nuevo = {
      id: `m_${Date.now()}`,
      fecha: fechaMov,
      categoria: catMov,
      nota: notaMov || '—',
      monto: `${tipoMov === 'gasto' ? '− $' : '+ $'} ${montoMov}`,
      tipo: tipoMov,
      relacion: 'Normal',
    };
    setMovimientos([nuevo, ...movimientos]);
    setMontoMov('');
    setNotaMov('');
  };

  // Clases CSS reactivas con forzado de appearance-none en selects
  const bgMain = modoOscuro ? 'bg-[#0B0F19] text-white' : 'bg-[#F6F5FA] text-slate-900';
  const bgCard = modoOscuro ? 'bg-[#151C28] border-gray-800/80 text-white' : 'bg-white border-gray-200 text-slate-900';
  
  const bgInput = modoOscuro
    ? 'bg-[#0F1420] border-gray-700 text-white placeholder-gray-500 appearance-none bg-[url("data:image/svg+xml;charset=UTF-8,%3csvg%20xmlns=\'http://www.w3.org/2000/svg\'%20viewBox=\'0%200%2024%2024\'%20fill=\'none\'%20stroke=\'%23A1A1AA\'%20stroke-width=\'2\'%20stroke-linecap=\'round\'%20stroke-linejoin=\'round\'%3e%3cpolyline%20points=\'6%209%2012%2015%2018%209\'%3e%3c/polyline%3e%3c/svg%3e")] bg-[length:1.25rem] bg-[right_0.75rem_center] bg-no-repeat pr-9 [&>option]:bg-[#0F1420] [&>option]:text-white'
    : 'bg-white border-gray-200 text-slate-900 placeholder-gray-400 [&>option]:bg-white [&>option]:text-slate-900';

  const textMuted = modoOscuro ? 'text-gray-400' : 'text-gray-500';

  return (
    <div className={`flex min-h-screen w-full font-sans transition-colors duration-200 ${bgMain}`}>
      
      {/* SIDEBAR LATERAL OSCURO FIXO */}
      <aside className="w-[224px] min-h-screen bg-[#211337] text-white flex flex-col justify-between p-6 shrink-0">
        <div>
          <h1 className="text-xl font-bold tracking-tight mb-8 text-white">Cuentas Claras</h1>
          <nav className="space-y-2">
            <button
              onClick={() => setPantallaActual('mis-cuentas')}
              className={`w-full text-left px-4 py-2.5 rounded-xl font-medium text-sm transition ${
                pantallaActual === 'mis-cuentas'
                  ? 'bg-[#9333EA] text-white shadow-lg shadow-purple-900/50'
                  : 'text-purple-200 hover:bg-white/10'
              }`}
            >
              Mis cuentas
            </button>
            <button
              onClick={() => setPantallaActual('detalle-cuenta')}
              className={`w-full text-left px-4 py-2.5 rounded-xl font-medium text-sm transition ${
                pantallaActual === 'detalle-cuenta'
                  ? 'bg-[#9333EA] text-white shadow-lg shadow-purple-900/50'
                  : 'text-purple-200 hover:bg-white/10'
              }`}
            >
              Detalle de Cuenta
            </button>
          </nav>
        </div>

        {/* Control de Modo Oscuro / Claro */}
        <div className="pt-4 border-t border-purple-900/40 flex items-center justify-between text-xs">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded-full bg-black text-white flex items-center justify-center font-bold text-[10px]">
              N
            </div>
            <span className="text-purple-300">{modoOscuro ? 'Oscuro' : 'Claro'}</span>
          </div>
          <button
            onClick={() => setModoOscuro(!modoOscuro)}
            className="px-3 py-1.5 bg-[#9333EA] hover:bg-purple-600 text-white rounded-full font-semibold shadow transition"
          >
            {modoOscuro ? '🌙 Oscuro' : '☀️ Claro'}
          </button>
        </div>
      </aside>

      {/* ÁREA DE CONTENIDO PRINCIPAL */}
      <main className="flex-1 p-8 overflow-y-auto">
        {pantallaActual === 'mis-cuentas' ? (
          
          /* VISTA: MIS CUENTAS */
          <div className="max-w-5xl mx-auto space-y-6">
            <div className="flex justify-between items-center">
              <div>
                <h2 className="text-2xl font-bold tracking-tight">Mis cuentas</h2>
                <p className={`text-xs mt-1 ${textMuted}`}>Listado de cuentas y saldos actuales</p>
              </div>
              <button className="bg-[#9333EA] text-white px-5 py-2.5 rounded-xl text-xs font-bold hover:bg-purple-600 shadow-md">
                + Nueva cuenta
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {cuentas.map((c, index) => (
                <div
                  key={c.id}
                  onClick={() => {
                    setCuentaActivaId(c.id);
                    setPantallaActual('detalle-cuenta');
                  }}
                  className={`p-6 rounded-2xl cursor-pointer transition-all border shadow-sm ${bgCard} ${
                    index === 0 ? 'border-purple-500/80 ring-1 ring-purple-500/30' : 'hover:border-purple-400'
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
                      {c.simbolo} {c.saldo}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (

          /* VISTA: DETALLE DE CUENTA */
          <div className="max-w-6xl mx-auto space-y-6">
            <div>
              <h2 className="text-2xl font-bold tracking-tight">Billetera Dólares (USD)</h2>
              <p className={`text-xs mt-0.5 ${textMuted}`}>Detalle de cuenta y movimientos</p>
            </div>

            {/* COMPOSICIÓN DEL SALDO */}
            <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
              <h3 className={`text-[11px] font-bold uppercase tracking-wider mb-3 ${textMuted}`}>Composición del Saldo</h3>
              <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-center">
                <div className={`p-3 rounded-xl ${modoOscuro ? 'bg-slate-800/60' : 'bg-gray-100/70'}`}>
                  <span className={`block text-xs ${textMuted}`}>Saldo Inicial</span>
                  <span className="font-bold text-xs mt-1 block">$ 1.000,00</span>
                </div>
                <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-emerald-950/30 text-emerald-400 border-emerald-900/30' : 'bg-emerald-50 text-emerald-600 border-emerald-100'}`}>
                  <span className="block text-xs font-medium">Ingresos</span>
                  <span className="font-bold text-xs mt-1 block">+ $ 800,00</span>
                </div>
                <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-rose-950/30 text-rose-400 border-rose-900/30' : 'bg-rose-50 text-rose-600 border-rose-100'}`}>
                  <span className="block text-xs font-medium">Gastos</span>
                  <span className="font-bold text-xs mt-1 block">− $ 500,00</span>
                </div>
                <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-blue-950/30 text-blue-400 border-blue-900/30' : 'bg-blue-50 text-blue-600 border-blue-100'}`}>
                  <span className="block text-xs font-medium">Ajustes</span>
                  <span className="font-bold text-xs mt-1 block">$ + 450,00</span>
                </div>
                <div className={`p-3 rounded-xl border ${modoOscuro ? 'bg-purple-950/40 text-purple-300 border-purple-900/40' : 'bg-purple-50 text-[#9333EA] border-purple-100'} font-bold`}>
                  <span className="block text-xs font-medium">Saldo Actual</span>
                  <span className="text-xs font-extrabold mt-1 block">$ 1.750,00</span>
                </div>
              </div>
            </div>

            {/* HISTORIAL Y FORMULARIOS */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              
              {/* HISTORIAL Y NUEVA CATEGORÍA */}
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
                          <th className="pb-3 font-semibold text-center">Estado / Relación</th>
                          <th className="pb-3 font-semibold text-right">Acción</th>
                        </tr>
                      </thead>
                      <tbody className={`divide-y ${modoOscuro ? 'divide-gray-800' : 'divide-gray-200'}`}>
                        {movimientos.map((m) => (
                          <tr key={m.id} className={modoOscuro ? 'hover:bg-slate-800/40' : 'hover:bg-gray-50/50'}>
                            <td className="py-3.5 text-xs">{m.fecha}</td>
                            <td className="py-3.5 font-medium">{m.categoria}</td>
                            <td className={`py-3.5 ${textMuted}`}>{m.nota}</td>
                            <td className={`py-3.5 font-bold text-right ${m.tipo === 'gasto' ? 'text-rose-400' : 'text-emerald-400'}`}>
                              {m.monto}
                            </td>
                            <td className="py-3.5 text-center">
                              {m.relacion === 'Corregido' ? (
                                <span className="bg-amber-900/60 text-amber-200 border border-amber-700/50 text-[10px] font-bold px-2 py-0.5 rounded-md">
                                  Corregido
                                </span>
                              ) : m.relacion.includes('Ajuste') ? (
                                <span className="bg-blue-900/60 text-blue-200 border border-blue-700/50 text-[10px] font-bold px-2 py-0.5 rounded-md">
                                  {m.relacion}
                                </span>
                              ) : (
                                <span className={`text-[11px] ${textMuted}`}>Normal</span>
                              )}
                            </td>
                            <td className="py-3.5 text-right">
                              <button className="bg-[#9333EA] text-white text-[11px] px-3 py-1 rounded-lg font-medium hover:bg-purple-600 transition">
                                Corregir
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>

                {/* NUEVA CATEGORÍA */}
                <div className={`border rounded-2xl p-5 shadow-sm ${bgCard}`}>
                  <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
                  <form onSubmit={handleAgregarCategoria} className="flex gap-3">
                    <input
                      type="text"
                      placeholder="Nombre de categoría"
                      value={nuevaCatNombre}
                      onChange={(e) => setNuevaCatNombre(e.target.value)}
                      className={`flex-1 border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                    />
                    <select
                      value={nuevaCatTipo}
                      onChange={(e) => setNuevaCatTipo(e.target.value as 'gasto' | 'ingreso')}
                      className={`border px-3 py-2 rounded-xl text-xs focus:outline-none cursor-pointer ${bgInput}`}
                    >
                      <option value="gasto">Gasto</option>
                      <option value="ingreso">Ingreso</option>
                    </select>
                    <button type="submit" className="bg-[#9333EA] text-white px-4 py-2 rounded-xl text-xs font-semibold hover:bg-purple-600">
                      Añadir
                    </button>
                  </form>
                </div>
              </div>

              {/* REGISTRAR MOVIMIENTO */}
              <div className={`border rounded-2xl p-5 shadow-sm h-fit ${bgCard}`}>
                <h3 className="text-base font-bold mb-4">Registrar Movimiento</h3>
                <form onSubmit={handleAgregarMovimiento} className="space-y-4">
                  <div className="flex gap-4 text-xs font-medium">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="radio"
                        name="tipo"
                        checked={tipoMov === 'gasto'}
                        onChange={() => setTipoMov('gasto')}
                        className="accent-[#9333EA]"
                      />
                      Gasto
                    </label>
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="radio"
                        name="tipo"
                        checked={tipoMov === 'ingreso'}
                        onChange={() => setTipoMov('ingreso')}
                        className="accent-[#9333EA]"
                      />
                      Ingreso
                    </label>
                  </div>

                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Monto</label>
                    <input
                      type="text"
                      placeholder="Ej: 50,00"
                      value={montoMov}
                      onChange={(e) => setMontoMov(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                    />
                  </div>

                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Fecha</label>
                    <input
                      type="date"
                      value={fechaMov}
                      onChange={(e) => setFechaMov(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                    />
                  </div>

                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Categoría</label>
                    <select
                      value={catMov}
                      onChange={(e) => setCatMov(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 cursor-pointer ${bgInput}`}
                    >
                      <option value="">Seleccione categoría...</option>
                      {categorias[tipoMov].map((c) => (
                        <option key={c} value={c}>{c}</option>
                      ))}
                    </select>
                  </div>

                  <div>
                    <label className={`block text-[11px] font-semibold mb-1 ${textMuted}`}>Nota (opcional)</label>
                    <input
                      type="text"
                      placeholder="Ej: Mercado del sábado"
                      value={notaMov}
                      onChange={(e) => setNotaMov(e.target.value)}
                      className={`w-full border px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500 ${bgInput}`}
                    />
                  </div>

                  <button
                    type="submit"
                    className="w-full bg-[#9333EA] text-white py-2.5 rounded-xl font-semibold text-xs hover:bg-purple-600 transition shadow-md mt-2"
                  >
                    Guardar Movimiento
                  </button>
                </form>
              </div>

            </div>
          </div>
        )}
      </main>
    </div>
  );
}