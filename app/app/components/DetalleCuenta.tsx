import React, { useState } from 'react';
import { Cuenta, DetalleCuentaData, Categoria, Movimiento, TipoMovimiento, Moneda } from '../types/cuentas';
import { ComposicionSaldo } from './ComposicionSaldo';
import { ListaMovimientos } from './ListaMovimientos';
import { DialogoCorregir } from './DialogoCorregir';
import { DialogoAdvertencia } from './DialogoAdvertencia';
import { FormularioMovimiento } from './FormularioMovimiento';
import { FormularioCategoria } from './FormularioCategoria';
import { IconoFlechaAtras, IconoGasto, IconoIngreso, IconoMas, IconoTipo } from './Iconos';
import { nombreTipo } from '../utils/tipos';

interface Props {
  detalle: DetalleCuentaData;
  cuentas: Cuenta[];
  monedas: Moneda[];
  categorias: Categoria[];
  onVolver: () => void;
  onCambiarCuenta: (cuentaId: string) => void;
  onRegistrarMovimiento: (datos: {
    cuenta_id: string;
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => Promise<void>;
  onPrevisualizarMovimiento: (datos: {
    cuenta_id: string;
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => Promise<{ saldo_resultante: string; deja_saldo_negativo: boolean }>;
  onCorregirMovimiento: (movimientoId: string, montoCorrecto: string) => Promise<void>;
  onCrearCategoria: (nombre: string, tipo: TipoMovimiento) => Promise<void>;
}

export const DetalleCuenta: React.FC<Props> = ({
  detalle,
  cuentas,
  monedas,
  categorias,
  onVolver,
  onCambiarCuenta,
  onRegistrarMovimiento,
  onPrevisualizarMovimiento,
  onCorregirMovimiento,
  onCrearCategoria,
}) => {
  const { cuenta, composicion, movimientos } = detalle;
  const monedaObj = monedas.find((m) => m.codigo === cuenta.moneda);
  const simbolo = monedaObj?.simbolo || cuenta.moneda;

  // Estados de modales
  const [modalMovimiento, setModalMovimiento] = useState(false);
  const [tipoMovimientoModal, setTipoMovimientoModal] = useState<TipoMovimiento>('gasto');
  const [modalCategoria, setModalCategoria] = useState(false);
  const [tipoCategoriaModal, setTipoCategoriaModal] = useState<TipoMovimiento>('gasto');
  const [movimientoAEditar, setMovimientoAEditar] = useState<Movimiento | null>(null);

  // Advertencia de saldo negativo
  const [advertenciaSaldo, setAdvertenciaSaldo] = useState<string | null>(null);
  const [movimientoPendiente, setMovimientoPendiente] = useState<{
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  } | null>(null);

  const abrirModalMovimiento = (tipo: TipoMovimiento) => {
    setTipoMovimientoModal(tipo);
    setModalMovimiento(true);
  };

  const handleRegistrarMovimiento = async (datos: {
    tipo: TipoMovimiento;
    monto: string;
    fecha: string;
    categoria_id: string;
    nota: string;
  }) => {
    // Si es gasto, previsualizar para advertir si deja saldo negativo
    if (datos.tipo === 'gasto') {
      try {
        const prev = await onPrevisualizarMovimiento({
          cuenta_id: cuenta.id,
          ...datos,
        });
        if (prev.deja_saldo_negativo) {
          setAdvertenciaSaldo(prev.saldo_resultante);
          setMovimientoPendiente(datos);
          setModalMovimiento(false);
          return;
        }
      } catch {
        // En caso de error de validación, continuar para que lo maneje el registro
      }
    }

    await onRegistrarMovimiento({
      cuenta_id: cuenta.id,
      ...datos,
    });
  };

  const confirmarMovimientoConAdvertencia = async () => {
    if (!movimientoPendiente) return;
    try {
      await onRegistrarMovimiento({
        cuenta_id: cuenta.id,
        ...movimientoPendiente,
      });
    } finally {
      setAdvertenciaSaldo(null);
      setMovimientoPendiente(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Barra de navegación superior con botón Volver y selector de cuentas */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <button
          type="button"
          onClick={onVolver}
          className="inline-flex h-10 items-center justify-center gap-2 rounded-2xl border border-borde bg-superficie px-4 text-xs font-semibold text-texto shadow-suave transition hover:bg-superficie-2 hover:border-acento focus-visible:outline-2 focus-visible:outline-acento sm:w-auto sm:text-sm"
        >
          <IconoFlechaAtras width={16} height={16} />
          <span>Volver a Mis cuentas</span>
        </button>

        <div className="flex flex-col gap-2 w-full sm:w-auto sm:flex-row sm:items-center">
          {cuentas.length > 1 && (
            <div className="flex w-full sm:w-auto items-center justify-between sm:justify-start gap-1.5 rounded-2xl border border-borde bg-superficie px-3 py-2 shadow-sm">
              <span className="text-xs text-suave">Cuenta:</span>
              <select
                value={cuenta.id}
                onChange={(e) => onCambiarCuenta(e.target.value)}
                className="bg-transparent text-xs font-semibold text-texto outline-none cursor-pointer max-w-[200px] truncate"
              >
                {cuentas.map((c) => (
                  <option key={c.id} value={c.id} className="bg-superficie text-texto">
                    {c.nombre} ({c.moneda})
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2 w-full sm:w-auto sm:flex sm:flex-row">
            <button
              type="button"
              onClick={() => abrirModalMovimiento('gasto')}
              className="inline-flex h-10 items-center justify-center gap-1.5 rounded-xl bg-peligro px-4 text-xs font-semibold text-white shadow-sm transition hover:brightness-110 active:scale-[0.98] sm:text-sm"
            >
              <IconoGasto width={14} height={14} />
              Nuevo Gasto
            </button>
            <button
              type="button"
              onClick={() => abrirModalMovimiento('ingreso')}
              className="inline-flex h-10 items-center justify-center gap-1.5 rounded-xl bg-exito px-4 text-xs font-semibold text-white shadow-sm transition hover:brightness-110 active:scale-[0.98] sm:text-sm"
            >
              <IconoIngreso width={14} height={14} />
              Nuevo Ingreso
            </button>
          </div>
        </div>
      </div>

      {/* Tarjeta principal con detalle y composición */}
      <div className="rounded-3xl border border-borde bg-superficie p-4 sm:p-6 shadow-suave">
        {/* Cabecera interna de la cuenta */}
        <div className="mb-6 flex flex-wrap items-center justify-between gap-3 border-b border-borde pb-5">
          <div className="flex items-center gap-3">
            <span className="grid size-11 sm:size-12 shrink-0 place-items-center rounded-2xl bg-acento-suave text-acento-texto shadow-sm">
              <IconoTipo tipo={cuenta.tipo} width={22} height={22} />
            </span>
            <div className="min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <h3 className="text-lg font-bold tracking-tight text-texto sm:text-2xl truncate">
                  {cuenta.nombre}
                </h3>
                <span className="rounded-full bg-acento-suave px-2.5 py-0.5 text-xs font-bold text-acento-texto">
                  {cuenta.moneda}
                </span>
              </div>
              <p className="mt-0.5 text-xs text-suave">
                {nombreTipo(cuenta.tipo)} · ID local: <span className="font-mono text-[11px] opacity-75">{cuenta.id}</span>
              </p>
            </div>
          </div>

          <button
            type="button"
            onClick={() => {
              setTipoCategoriaModal('gasto');
              setModalCategoria(true);
            }}
            className="inline-flex items-center gap-1.5 rounded-xl border border-borde bg-superficie-2 px-3 py-1.5 text-xs font-semibold text-suave transition hover:border-acento hover:text-texto"
          >
            <IconoMas width={14} height={14} />
            Nueva Categoría
          </button>
        </div>

        {/* Composición del Saldo */}
        <ComposicionSaldo
          composicion={composicion}
          simbolo={simbolo}
          saldoActual={cuenta.saldo_actual}
        />

        {/* Lista de Movimientos */}
        <ListaMovimientos
          movimientos={movimientos}
          simbolo={simbolo}
          onCorregir={(m) => setMovimientoAEditar(m)}
        />
      </div>

      {/* Modales */}
      {modalMovimiento && (
        <FormularioMovimiento
          cuentaNombre={cuenta.nombre}
          simbolo={simbolo}
          tipoInicial={tipoMovimientoModal}
          categorias={categorias}
          onRegistrar={handleRegistrarMovimiento}
          onAbrirNuevaCategoria={(t) => {
            setTipoCategoriaModal(t);
            setModalCategoria(true);
          }}
          onCerrar={() => setModalMovimiento(false)}
        />
      )}

      {modalCategoria && (
        <FormularioCategoria
          tipoInicial={tipoCategoriaModal}
          onCrear={async (nombre, tipo) => {
            await onCrearCategoria(nombre, tipo);
          }}
          onCerrar={() => setModalCategoria(false)}
        />
      )}

      {advertenciaSaldo !== null && (
        <DialogoAdvertencia
          saldoNegativo={advertenciaSaldo}
          simbolo={simbolo}
          onConfirmar={confirmarMovimientoConAdvertencia}
          onCancelar={() => {
            setAdvertenciaSaldo(null);
            setMovimientoPendiente(null);
          }}
        />
      )}

      {movimientoAEditar && (
        <DialogoCorregir
          movimiento={movimientoAEditar}
          simbolo={simbolo}
          onConfirmar={async (movId, montoCorrecto) => {
            await onCorregirMovimiento(movId, montoCorrecto);
            setMovimientoAEditar(null);
          }}
          onCerrar={() => setMovimientoAEditar(null)}
        />
      )}
    </div>
  );
};