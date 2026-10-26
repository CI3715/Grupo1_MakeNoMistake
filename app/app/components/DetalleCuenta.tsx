// app/components/DetalleCuenta.tsx
import React, { useState } from "react";

export interface Movimiento {
  id: string;
  fecha: string;
  categoria: string;
  nota: string;
  monto: number;
  tipo: "Ingreso" | "Gasto";
  relacion: string;
}

interface DetalleCuentaProps {
  cuenta: { nombre: string; moneda: string; simbolo: string; saldoInicial: number; saldoActual: number };
  movimientos: Movimiento[];
  categorias: { Ingreso: string[]; Gasto: string[] };
  ingresos: number;
  gastos: number;
  ajustes: number;
  onGuardarMovimiento: (m: { tipo: "Ingreso" | "Gasto"; monto: number; fecha: string; categoria: string; nota: string }) => void;
  onCorregir: (mov: Movimiento) => void;
  onAgregarCategoria: (tipo: "Ingreso" | "Gasto", nombre: string) => void;
}

export default function DetalleCuenta({
  cuenta,
  movimientos,
  categorias,
  ingresos,
  gastos,
  ajustes,
  onGuardarMovimiento,
  onCorregir,
  onAgregarCategoria,
}: DetalleCuentaProps) {
  const hoy = new Date().toISOString().split("T")[0];

  const [tipo, setTipo] = useState<"Ingreso" | "Gasto">("Gasto");
  const [monto, setMonto] = useState("");
  const [fecha, setFecha] = useState(hoy);
  const [categoria, setCategoria] = useState("");
  const [nota, setNota] = useState("");

  const [nuevaCatNombre, setNuevaCatNombre] = useState("");
  const [nuevaCatTipo, setNuevaCatTipo] = useState<"Ingreso" | "Gasto">("Gasto");

  const handleSubmitMovimiento = (e: React.FormEvent) => {
    e.preventDefault();
    const val = parseFloat(monto);
    if (isNaN(val) || val <= 0 || !categoria) return;
    onGuardarMovimiento({ tipo, monto: val, fecha, categoria, nota });
    setMonto("");
    setNota("");
  };

  const handleNuevaCategoriaSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!nuevaCatNombre.trim()) return;
    onAgregarCategoria(nuevaCatTipo, nuevaCatNombre.trim());
    setNuevaCatNombre("");
  };

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">{cuenta.nombre} ({cuenta.moneda})</h2>
        <p className="text-xs text-[var(--text-muted)] mt-0.5">Detalle de cuenta y movimientos</p>
      </div>

      {/* COMPOSICIÓN DEL SALDO */}
      <div className="bg-[var(--bg-card)] border border-[var(--border-color)] rounded-2xl p-5 shadow-sm">
        <h3 className="text-[11px] font-bold text-gray-400 uppercase tracking-wider mb-3">Composición del Saldo</h3>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-center">
          <div className="bg-gray-100/70 dark:bg-slate-800/50 p-3 rounded-xl">
            <p className="text-xs text-[var(--text-muted)]">Saldo Inicial</p>
            <p className="text-sm font-bold mt-1">{cuenta.simbolo} {cuenta.saldoInicial.toFixed(2)}</p>
          </div>
          <div className="bg-emerald-50 dark:bg-emerald-950/30 p-3 rounded-xl border border-emerald-100/50 dark:border-emerald-900/20">
            <p className="text-xs text-emerald-600 dark:text-emerald-400 font-medium">Ingresos</p>
            <p className="text-sm font-bold text-emerald-600 dark:text-emerald-400 mt-1">+ {cuenta.simbolo} {ingresos.toFixed(2)}</p>
          </div>
          <div className="bg-rose-50 dark:bg-rose-950/30 p-3 rounded-xl border border-rose-100/50 dark:border-rose-900/20">
            <p className="text-xs text-rose-600 dark:text-rose-400 font-medium">Gastos</p>
            <p className="text-sm font-bold text-rose-600 dark:text-rose-400 mt-1">- {cuenta.simbolo} {gastos.toFixed(2)}</p>
          </div>
          <div className="bg-blue-50 dark:bg-blue-950/30 p-3 rounded-xl border border-blue-100/50 dark:border-blue-900/20">
            <p className="text-xs text-blue-600 dark:text-blue-400 font-medium">Ajustes</p>
            <p className="text-sm font-bold text-blue-600 dark:text-blue-400 mt-1">{cuenta.simbolo} + {ajustes.toFixed(2)}</p>
          </div>
          <div className="bg-purple-50 dark:bg-purple-950/30 p-3 rounded-xl border border-purple-100/50 dark:border-purple-900/20">
            <p className="text-xs text-purple-600 dark:text-purple-400 font-medium">Saldo Actual</p>
            <p className="text-sm font-extrabold text-[#A855F7] mt-1">{cuenta.simbolo} {cuenta.saldoActual.toFixed(2)}</p>
          </div>
        </div>
      </div>

      {/* HISTORIAL Y FORMULARIO */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          {/* Historial */}
          <div className="bg-[var(--bg-card)] border border-[var(--border-color)] rounded-2xl p-5 shadow-sm">
            <h3 className="text-base font-bold mb-4">Historial de Movimientos</h3>
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-[var(--border-color)] text-[var(--text-muted)]">
                    <th className="pb-3 font-semibold">Fecha</th>
                    <th className="pb-3 font-semibold">Categoría</th>
                    <th className="pb-3 font-semibold">Nota</th>
                    <th className="pb-3 font-semibold text-right">Monto</th>
                    <th className="pb-3 font-semibold text-center">Estado / Relación</th>
                    <th className="pb-3 font-semibold text-right">Acción</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--border-color)]">
                  {movimientos.map((m) => (
                    <tr key={m.id} className="hover:bg-gray-50/50 dark:hover:bg-slate-800/20">
                      <td className="py-3.5 text-xs">{m.fecha}</td>
                      <td className="py-3.5 font-medium">{m.categoria}</td>
                      <td className="py-3.5 text-[var(--text-muted)]">{m.nota || "-"}</td>
                      <td className={`py-3.5 font-bold text-right ${m.tipo === "Gasto" ? "text-rose-500" : "text-emerald-500"}`}>
                        {m.tipo === "Gasto" ? `- ${cuenta.simbolo}` : `+ ${cuenta.simbolo}`} {m.monto.toFixed(2)}
                      </td>
                      <td className="py-3.5 text-center">
                        {m.relacion === "Corregido" ? (
                          <span className="bg-amber-200 dark:bg-amber-900/60 text-amber-900 dark:text-amber-200 text-[10px] font-bold px-2 py-0.5 rounded-md">
                            Corregido
                          </span>
                        ) : m.relacion.includes("Ajuste") ? (
                          <span className="bg-blue-200 dark:bg-blue-900/60 text-blue-900 dark:text-blue-200 text-[10px] font-bold px-2 py-0.5 rounded-md">
                            {m.relacion}
                          </span>
                        ) : (
                          <span className="text-[11px] text-gray-400">Normal</span>
                        )}
                      </td>
                      <td className="py-3.5 text-right">
                        <button
                          onClick={() => onCorregir(m)}
                          className="bg-[#A855F7] text-white text-[11px] px-3 py-1 rounded-lg font-medium hover:bg-purple-600 transition-all"
                        >
                          Corregir
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Nueva Categoría */}
          <div className="bg-[var(--bg-card)] border border-[var(--border-color)] rounded-2xl p-5 shadow-sm">
            <h3 className="text-sm font-bold mb-3">Nueva Categoría</h3>
            <form onSubmit={handleNuevaCategoriaSubmit} className="flex gap-3">
              <input
                type="text"
                placeholder="Nombre de categoría"
                value={nuevaCatNombre}
                onChange={(e) => setNuevaCatNombre(e.target.value)}
                className="flex-1 bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
              />
              <select
                value={nuevaCatTipo}
                onChange={(e) => setNuevaCatTipo(e.target.value as "Ingreso" | "Gasto")}
                className="bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none"
              >
                <option value="Gasto">Gasto</option>
                <option value="Ingreso">Ingreso</option>
              </select>
              <button type="submit" className="bg-[#A855F7] text-white px-4 py-2 rounded-xl text-xs font-semibold hover:bg-purple-600">
                Añadir
              </button>
            </form>
          </div>
        </div>

        {/* Panel Registrar Movimiento */}
        <div className="bg-[var(--bg-card)] border border-[var(--border-color)] rounded-2xl p-5 shadow-sm h-fit">
          <h3 className="text-base font-bold mb-4">Registrar Movimiento</h3>
          <form onSubmit={handleSubmitMovimiento} className="space-y-4">
            <div className="flex gap-4 text-xs font-medium">
              <label className="flex items-center gap-2 cursor-pointer">
                <input type="radio" name="tipo" checked={tipo === "Gasto"} onChange={() => setTipo("Gasto")} className="accent-[#A855F7]" />
                Gasto
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input type="radio" name="tipo" checked={tipo === "Ingreso"} onChange={() => setTipo("Ingreso")} className="accent-[#A855F7]" />
                Ingreso
              </label>
            </div>

            <div>
              <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">Monto</label>
              <input
                type="number"
                step="0.01"
                placeholder="Ej: 50,00"
                value={monto}
                onChange={(e) => setMonto(e.target.value)}
                className="w-full bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
              />
            </div>

            <div>
              <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">Fecha</label>
              <input
                type="date"
                value={fecha}
                onChange={(e) => setFecha(e.target.value)}
                className="w-full bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
              />
            </div>

            <div>
              <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">Categoría</label>
              <select
                value={categoria}
                onChange={(e) => setCategoria(e.target.value)}
                className="w-full bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
              >
                <option value="">Seleccione categoría...</option>
                {categorias[tipo].map((c) => (
                  <option key={c} value={c}>{c}</option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">Nota (opcional)</label>
              <input
                type="text"
                placeholder="Ej: Mercado del sábado"
                value={nota}
                onChange={(e) => setNota(e.target.value)}
                className="w-full bg-[var(--input-bg)] border border-[var(--border-color)] px-3 py-2 rounded-xl text-xs focus:outline-none focus:ring-1 focus:ring-purple-500"
              />
            </div>

            <button type="submit" className="w-full bg-[#A855F7] text-white py-2.5 rounded-xl font-semibold text-xs hover:bg-purple-600 transition-all shadow-md mt-2">
              Añadir Movimiento
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}