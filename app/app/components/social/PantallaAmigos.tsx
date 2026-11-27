'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { listarAmistadesCommand } from '../../utils/tauriCommands';
import { useCargaRemota } from '../../utils/useCargaRemota';
import { AvisoFalloSocial } from './AvisoFalloSocial';
import { BuscadorUsuario } from './BuscadorUsuario';
import { EncabezadoPantalla } from './EncabezadoPantalla';
import { EstadoCargando } from './EstadoCargando';
import { InvitacionesAmistad } from './InvitacionesAmistad';
import { ListaAmigos } from './ListaAmigos';

/** «Tu gente»: buscador exacto, invitaciones recibidas y enviadas, y lista de amigos. */
export function PantallaAmigos({ ctx }: { ctx: ContextoSocial }) {
  const { carga, recargar } = useCargaRemota(listarAmistadesCommand);

  // Tras una acción se vuelve a pedir la lista sin tapar la pantalla con el cargando.
  const actualizar = () => recargar(false);

  return (
    <div className="@container">
      <EncabezadoPantalla
        modoOscuro={ctx.modoOscuro}
        titulo="Tu gente"
        subtitulo="Amigos · Conecta con quien elijas, sin directorios públicos."
      />

      {carga.estado === 'cargando' && <EstadoCargando modoOscuro={ctx.modoOscuro} que="tus amigos" />}

      {carga.estado === 'fallo' && (
        <AvisoFalloSocial ctx={ctx} fallo={carga.fallo} onReintentar={() => recargar()} />
      )}

      {carga.estado === 'listo' && (
        <div className="grid gap-4 @2xl:grid-cols-2">
          <div className="min-w-0 space-y-4">
            <ListaAmigos ctx={ctx} amigos={carga.datos.amigos} onCambio={actualizar} />
            <InvitacionesAmistad
              ctx={ctx}
              recibidas={carga.datos.recibidas}
              enviadas={carga.datos.enviadas}
              onCambio={actualizar}
            />
          </div>
          <div className="min-w-0">
            <BuscadorUsuario ctx={ctx} onInvitada={actualizar} />
          </div>
        </div>
      )}
    </div>
  );
}
