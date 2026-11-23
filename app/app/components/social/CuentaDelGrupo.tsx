'use client';

import React, { useCallback, useState } from 'react';
import type { ContextoSocial } from '../../types/social';
import { cuentasParaGrupo, elegirCuentaGrupo } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useCargaRemota } from '../../utils/useCargaRemota';
import { useEnvio } from '../../utils/useEnvio';
import { BotonAccion } from './BotonAccion';
import { CampoSelect } from './CampoSelect';
import { Tarjeta } from './Tarjeta';

/** Campos con los que el núcleo puede señalar un error de esta pantalla. */
const CAMPOS = ['cuenta_id', 'cuenta'] as const;

interface Props {
  ctx: ContextoSocial;
  grupoId: string;
  /** Moneda del grupo: solo se ofrecen cuentas activas en esa moneda (INV-18). */
  moneda: string;
}

/**
 * «Cuenta para este grupo»: la cuenta personal donde se aplican tus partes. La elección
 * se guarda solo en este dispositivo y el cambio afecta solo a las partes que lleguen después.
 */
export function CuentaDelGrupo({ ctx, grupoId, moneda }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  const cargar = useCallback(() => cuentasParaGrupo(grupoId), [grupoId]);
  const { carga, recargar } = useCargaRemota(cargar);
  const envio = useEnvio(CAMPOS);
  const [seleccion, setSeleccion] = useState<string | null>(null);
  const [confirmada, setConfirmada] = useState<string | null>(null);
  const { refrescar } = ctx.sincronizacion;

  const elegir = async (cuentaId: string) => {
    if (!cuentaId) return;
    setConfirmada(null);
    setSeleccion(cuentaId);
    const r = await envio.enviar(() => elegirCuentaGrupo(grupoId, cuentaId), 'elegir');
    setSeleccion(null);
    if (!r.ok) return;
    setConfirmada(r.valor.cuentas.find((c) => c.id === r.valor.cuenta_elegida)?.nombre ?? '');
    recargar(false);
    // Las partes pendientes se aplican al elegir: cuentas, movimientos y saldos cambian.
    void refrescar();
  };

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Cuenta para este grupo"
      descripcion="Aquí se aplica tu parte de cada gasto, una sola vez."
    >
      {carga.estado === 'cargando' && (
        <div role="status" aria-busy="true">
          <span className="sr-only">Cargando tus cuentas…</span>
          <div
            className={`h-14 animate-pulse rounded-xl border ${tema.interior}`}
            aria-hidden="true"
          />
        </div>
      )}

      {carga.estado === 'fallo' && (
        <div role="alert" className={`rounded-xl border p-4 text-xs ${tema.avisoError}`}>
          <p className="font-semibold">No se pudieron cargar tus cuentas</p>
          <p className="mt-1">{carga.fallo.mensaje}</p>
          <div className="mt-3">
            <BotonAccion modoOscuro={ctx.modoOscuro} variante="secundario" onClick={() => recargar()}>
              Reintentar
            </BotonAccion>
          </div>
        </div>
      )}

      {carga.estado === 'listo' && carga.datos.cuentas.length === 0 && (
        <div role="status" className={`rounded-xl border p-4 text-xs ${tema.avisoAmbar}`}>
          <p className="font-semibold">No tienes cuentas activas en {moneda}</p>
          <p className="mt-1">
            Tu parte de este grupo se registra en {moneda} y solo puede bajar a una cuenta activa
            en esa moneda. Crea una y vuelve aquí.
          </p>
          <button
            type="button"
            onClick={ctx.irACuentas}
            className="mt-3 cursor-pointer text-xs font-semibold underline underline-offset-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
          >
            Crear una cuenta en {moneda}
          </button>
        </div>
      )}

      {carga.estado === 'listo' && carga.datos.cuentas.length > 0 && (
        <div className="space-y-3">
          <CampoSelect
            id={`cuenta-grupo-${grupoId}`}
            etiqueta="Cuenta para este grupo"
            valor={seleccion ?? carga.datos.cuenta_elegida ?? ''}
            opciones={carga.datos.cuentas.map((c) => ({
              valor: c.id,
              etiqueta: `${c.nombre} · ${c.moneda}`,
            }))}
            placeholder="Elige una cuenta"
            onCambio={(valor) => void elegir(valor)}
            deshabilitado={envio.cargando}
            error={envio.errores.cuenta_id ?? envio.errores.cuenta ?? envio.errores.general}
            modoOscuro={ctx.modoOscuro}
          />

          {carga.datos.cuenta_elegida &&
            !carga.datos.cuentas.some((c) => c.id === carga.datos.cuenta_elegida) && (
              <p role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}>
                La cuenta que elegiste ya no está disponible. Las partes nuevas quedan pendientes
                hasta que elijas otra.
              </p>
            )}

          {confirmada !== null && (
            <p role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoOk}`}>
              Listo{confirmada ? `: tus próximas partes bajan a «${confirmada}»` : ''}. Las que
              esperaban una cuenta se aplicaron.
            </p>
          )}

          <p className={`text-[11px] ${tema.textoSuave}`}>
            Si la cambias, el cambio afecta solo a las partes que lleguen después. Esta elección se
            guarda únicamente en este dispositivo.
          </p>
        </div>
      )}
    </Tarjeta>
  );
}
