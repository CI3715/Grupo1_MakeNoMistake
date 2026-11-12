'use client';

import React, { useState } from 'react';
import type { ContextoSocial, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import {
  responderAmistadCommand,
  type InvitacionAmistadDto,
  type RespuestaInvitacion,
} from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useMontado } from '../../utils/useCargaRemota';
import { Avatar } from './Avatar';
import { BotonAccion } from './BotonAccion';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

type Pestana = 'recibidas' | 'enviadas';

interface Props {
  ctx: ContextoSocial;
  recibidas: InvitacionAmistadDto[];
  enviadas: InvitacionAmistadDto[];
  onCambio: () => void;
}

/** Invitaciones de amistad: pestañas Recibidas (aceptar o rechazar) y Enviadas (solo ver). */
export function InvitacionesAmistad({ ctx, recibidas, enviadas, onCambio }: Props) {
  const [pestana, setPestana] = useState<Pestana>('recibidas');
  const [respondiendo, setRespondiendo] = useState<{ id: string; respuesta: RespuestaInvitacion } | null>(null);
  const [error, setError] = useState<{ id: string; mensaje: string; tipo: TipoFallo } | null>(null);
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  const responder = async (invitacion: InvitacionAmistadDto, respuesta: RespuestaInvitacion) => {
    if (respondiendo) return;
    setRespondiendo({ id: invitacion.id, respuesta });
    setError(null);
    try {
      await responderAmistadCommand({ amistad_id: invitacion.id, respuesta });
      if (montado.current) onCambio();
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, [] as readonly string[]);
        setError({ id: invitacion.id, mensaje: r.errores.general ?? '', tipo: r.tipo });
      }
    } finally {
      if (montado.current) setRespondiendo(null);
    }
  };

  const lista = pestana === 'recibidas' ? recibidas : enviadas;
  const pestanas: { id: Pestana; texto: string; cantidad: number }[] = [
    { id: 'recibidas', texto: 'Recibidas', cantidad: recibidas.length },
    { id: 'enviadas', texto: 'Enviadas', cantidad: enviadas.length },
  ];

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Invitaciones"
      descripcion="Una amistad existe solo cuando la otra persona acepta."
    >
      <div
        role="tablist"
        aria-label="Invitaciones de amistad"
        className={`mb-3 grid grid-cols-2 gap-1 rounded-xl p-1 ${tema.interior} border`}
      >
        {pestanas.map((p) => (
          <button
            key={p.id}
            type="button"
            role="tab"
            id={`pestana-${p.id}`}
            aria-selected={pestana === p.id}
            aria-controls="panel-invitaciones"
            onClick={() => setPestana(p.id)}
            className={`rounded-lg px-3 py-1.5 text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
              pestana === p.id ? 'bg-[#9333EA] text-white' : tema.textoSuave
            }`}
          >
            {p.texto} · {p.cantidad}
          </button>
        ))}
      </div>

      <div id="panel-invitaciones" role="tabpanel" aria-labelledby={`pestana-${pestana}`}>
        {lista.length === 0 ? (
          <EstadoVacioSocial
            modoOscuro={ctx.modoOscuro}
            titulo={pestana === 'recibidas' ? 'No tienes invitaciones' : 'No has enviado invitaciones'}
            texto={
              pestana === 'recibidas'
                ? 'Cuando alguien te invite como amigo, la verás aquí.'
                : 'Las invitaciones que envíes aparecerán aquí hasta que las respondan.'
            }
          />
        ) : (
          <ul>
            {lista.map((inv) => {
              const { nombre_usuario: usuario, nombre_visible: visible } = inv.usuario;
              const ocupada = respondiendo?.id === inv.id;
              return (
                <li key={inv.id} className={`border-b py-3 last:border-b-0 ${tema.linea}`}>
                  <div className="flex flex-wrap items-center gap-3">
                    <Avatar nombre={visible} />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-semibold">{visible}</p>
                      <p className={`truncate text-[11px] ${tema.textoSuave}`}>
                        @{usuario} ·{' '}
                        {pestana === 'recibidas' ? 'Te invitó' : 'Pendiente de aceptación'}
                      </p>
                    </div>
                    {pestana === 'recibidas' && (
                      <div className="flex gap-2">
                        <BotonAccion
                          modoOscuro={ctx.modoOscuro}
                          cargando={ocupada && respondiendo?.respuesta === 'aceptar'}
                          deshabilitado={respondiendo !== null}
                          etiquetaAria={`Aceptar la invitación de ${visible}`}
                          onClick={() => responder(inv, 'aceptar')}
                        >
                          Aceptar
                        </BotonAccion>
                        <BotonAccion
                          modoOscuro={ctx.modoOscuro}
                          variante="secundario"
                          cargando={ocupada && respondiendo?.respuesta === 'rechazar'}
                          deshabilitado={respondiendo !== null}
                          etiquetaAria={`Rechazar la invitación de ${visible}`}
                          onClick={() => responder(inv, 'rechazar')}
                        >
                          Rechazar
                        </BotonAccion>
                      </div>
                    )}
                  </div>
                  {error?.id === inv.id && (
                    <div className="mt-2">
                      <MensajeGeneral ctx={ctx} mensaje={error.mensaje} tipo={error.tipo} />
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Tarjeta>
  );
}
