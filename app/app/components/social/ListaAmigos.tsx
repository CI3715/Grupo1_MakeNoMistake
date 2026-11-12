'use client';

import React, { useState } from 'react';
import type { ContextoSocial, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import { plural } from '../../utils/formatoSocial';
import { eliminarAmistadCommand, type AmigoDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useMontado } from '../../utils/useCargaRemota';
import { Avatar } from './Avatar';
import { BotonAccion } from './BotonAccion';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

interface Props {
  ctx: ContextoSocial;
  amigos: AmigoDto[];
  /** Se llama tras eliminar, para actualizar las listas. */
  onCambio: () => void;
}

/** Amigos con «Eliminar». Pide confirmación en la misma fila antes de borrar la amistad. */
export function ListaAmigos({ ctx, amigos, onCambio }: Props) {
  const [confirmando, setConfirmando] = useState<string | null>(null);
  const [eliminando, setEliminando] = useState(false);
  const [error, setError] = useState<{ mensaje: string; tipo: TipoFallo } | null>(null);
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  const eliminar = async (amigo: AmigoDto) => {
    if (eliminando) return;
    setEliminando(true);
    setError(null);
    try {
      await eliminarAmistadCommand(amigo.amistad_id);
      if (!montado.current) return;
      setConfirmando(null);
      onCambio();
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, [] as readonly string[]);
        setError({ mensaje: r.errores.general ?? '', tipo: r.tipo });
      }
    } finally {
      if (montado.current) setEliminando(false);
    }
  };

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo={`Amigos · ${amigos.length}`}
      descripcion="Con quién puedes compartir grupos."
    >
      {amigos.length === 0 ? (
        <EstadoVacioSocial
          modoOscuro={ctx.modoOscuro}
          titulo="Todavía no tienes amigos"
          texto="Busca a alguien por su nombre de usuario exacto y envíale una invitación."
        />
      ) : (
        <ul className="space-y-1">
          {amigos.map((amigo) => {
            const { nombre_usuario: usuario, nombre_visible: visible } = amigo.usuario;
            const enConfirmacion = confirmando === amigo.amistad_id;
            return (
              <li key={amigo.amistad_id} className={`border-b py-3 last:border-b-0 ${tema.linea}`}>
                <div className="flex items-center gap-3">
                  <Avatar nombre={visible} />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-semibold">{visible}</p>
                    <p className={`truncate text-[11px] ${tema.textoSuave}`}>@{usuario}</p>
                  </div>
                  {!enConfirmacion && (
                    <BotonAccion
                      modoOscuro={ctx.modoOscuro}
                      variante="secundario"
                      etiquetaAria={`Eliminar la amistad con ${visible}`}
                      onClick={() => {
                        setError(null);
                        setConfirmando(amigo.amistad_id);
                      }}
                    >
                      Eliminar
                    </BotonAccion>
                  )}
                </div>

                {enConfirmacion && (
                  <div
                    role="group"
                    aria-label={`Confirmar eliminar la amistad con ${visible}`}
                    className={`mt-3 rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}
                  >
                    <p className="font-semibold">¿Eliminar a {visible} de tus amigos?</p>
                    <p className="mt-1 opacity-90">
                      Dejarán de poder invitarse a grupos nuevos. Los grupos y gastos que ya
                      comparten no se borran ni cambian.
                    </p>
                    <div className="mt-3 flex flex-wrap gap-2">
                      <BotonAccion
                        modoOscuro={ctx.modoOscuro}
                        variante="peligro"
                        cargando={eliminando}
                        onClick={() => eliminar(amigo)}
                      >
                        {eliminando ? 'Eliminando…' : 'Eliminar amistad'}
                      </BotonAccion>
                      <BotonAccion
                        modoOscuro={ctx.modoOscuro}
                        variante="secundario"
                        deshabilitado={eliminando}
                        onClick={() => setConfirmando(null)}
                      >
                        Conservar
                      </BotonAccion>
                    </div>
                    {error && (
                      <div className="mt-3">
                        <MensajeGeneral ctx={ctx} mensaje={error.mensaje} tipo={error.tipo} />
                      </div>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      <p className="sr-only" aria-live="polite">
        {plural(amigos.length, 'amigo', 'amigos')}
      </p>
    </Tarjeta>
  );
}
