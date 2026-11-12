'use client';

import React, { useState } from 'react';
import type { ContextoSocial, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import {
  responderInvitacionGrupoCommand,
  type InvitacionGrupoDto,
  type RespuestaInvitacion,
} from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useMontado } from '../../utils/useCargaRemota';
import { BotonAccion } from './BotonAccion';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

interface Props {
  ctx: ContextoSocial;
  invitaciones: InvitacionGrupoDto[];
  onCambio: () => void;
}

/** Invitaciones a grupos. Hasta aceptar no se ven gastos ni balances (RF-COM-02). */
export function InvitacionesGrupo({ ctx, invitaciones, onCambio }: Props) {
  const [respondiendo, setRespondiendo] = useState<{ id: string; respuesta: RespuestaInvitacion } | null>(null);
  const [error, setError] = useState<{ id: string; mensaje: string; tipo: TipoFallo } | null>(null);
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  if (invitaciones.length === 0) return null;

  const responder = async (inv: InvitacionGrupoDto, respuesta: RespuestaInvitacion) => {
    if (respondiendo) return;
    setRespondiendo({ id: inv.grupo_id, respuesta });
    setError(null);
    try {
      await responderInvitacionGrupoCommand({ grupo_id: inv.grupo_id, respuesta });
      if (montado.current) onCambio();
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, [] as readonly string[]);
        setError({ id: inv.grupo_id, mensaje: r.errores.general ?? '', tipo: r.tipo });
      }
    } finally {
      if (montado.current) setRespondiendo(null);
    }
  };

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo={invitaciones.length === 1 ? 'Invitación a un grupo' : 'Invitaciones a grupos'}
    >
      <ul className="space-y-3">
        {invitaciones.map((inv) => {
          const ocupada = respondiendo?.id === inv.grupo_id;
          return (
            <li key={inv.grupo_id} className={`rounded-xl border p-3 ${tema.interior}`}>
              <span
                className={`inline-block rounded-md px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${tema.etiqueta}`}
              >
                Requiere tu aceptación
              </span>
              <p className="mt-2 text-sm font-semibold">{inv.nombre}</p>
              <p className={`mt-0.5 text-xs ${tema.textoSuave}`}>
                {inv.invitado_por.nombre_visible} (@{inv.invitado_por.nombre_usuario}) te invita ·
                Moneda {inv.moneda}
                {inv.actividad ? ` · ${inv.actividad}` : ''}. Todavía no perteneces al grupo.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                <BotonAccion
                  modoOscuro={ctx.modoOscuro}
                  cargando={ocupada && respondiendo?.respuesta === 'aceptar'}
                  deshabilitado={respondiendo !== null}
                  etiquetaAria={`Aceptar la invitación al grupo ${inv.nombre}`}
                  onClick={() => responder(inv, 'aceptar')}
                >
                  Aceptar invitación
                </BotonAccion>
                <BotonAccion
                  modoOscuro={ctx.modoOscuro}
                  variante="secundario"
                  cargando={ocupada && respondiendo?.respuesta === 'rechazar'}
                  deshabilitado={respondiendo !== null}
                  etiquetaAria={`Rechazar la invitación al grupo ${inv.nombre}`}
                  onClick={() => responder(inv, 'rechazar')}
                >
                  Rechazar
                </BotonAccion>
              </div>
              {error?.id === inv.grupo_id && (
                <div className="mt-3">
                  <MensajeGeneral ctx={ctx} mensaje={error.mensaje} tipo={error.tipo} />
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </Tarjeta>
  );
}
