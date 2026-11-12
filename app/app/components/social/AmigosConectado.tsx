'use client';

import React, { useState } from 'react';
import {
  buscarUsuarioCommand,
  eliminarAmistadCommand,
  invitarAmigoCommand,
  listarAmistadesCommand,
  responderAmistadCommand,
  type RespuestaInvitacion,
  type UsuarioPublicoDto,
} from '../../utils/tauriCommands';
import { useCargaSocial } from '../../utils/useCargaSocial';
import { useEnvio } from '../../utils/useEnvio';
import { enfocarPrimerError } from '../../utils/enfocar';
import type { CampoUsuario } from '../../types/social';
import { BuscadorUsuario } from './BuscadorUsuario';
import { ListaAmigos } from './ListaAmigos';
import { InvitacionesAmistad } from './InvitacionesAmistad';
import { AvisoSocial, CargandoSocial, EncabezadoSocial } from './ElementosSociales';

const CAMPOS_USUARIO: readonly CampoUsuario[] = ['usuario'];

/**
 * Pantalla «Amigos» conectada a los comandos: buscar (nombre exacto), invitar, aceptar,
 * rechazar y eliminar. El error de cada campo va debajo de él; lo que no es de un campo
 * (sin conexión, sesión vencida) va en un aviso arriba y el resto de la app sigue funcionando.
 */
export function AmigosConectado({ modoOscuro }: { modoOscuro: boolean }) {
  const { estado, recargar } = useCargaSocial(listarAmistadesCommand);
  const [usuario, setUsuario] = useState('');
  const [encontrado, setEncontrado] = useState<UsuarioPublicoDto | null>(null);
  const [exito, setExito] = useState<string | null>(null);

  const busqueda = useEnvio<CampoUsuario>(CAMPOS_USUARIO);
  const invitacion = useEnvio<CampoUsuario>(CAMPOS_USUARIO);
  const filas = useEnvio();

  const buscar = async () => {
    setExito(null);
    setEncontrado(null);
    invitacion.limpiar();
    // El nombre de usuario va en minúsculas (acuerdo del martes).
    const r = await busqueda.enviar(() => buscarUsuarioCommand({ usuario: usuario.trim().toLowerCase() }));
    if (r.ok) setEncontrado(r.valor);
    else enfocarPrimerError();
  };

  const invitar = async () => {
    if (!encontrado) return;
    const r = await invitacion.enviar(() => invitarAmigoCommand({ usuario: encontrado.nombre_usuario }));
    if (r.ok) {
      setExito(`Invitación enviada a @${encontrado.nombre_usuario}. Será tu amigo cuando la acepte.`);
      setEncontrado(null);
      setUsuario('');
      recargar();
    } else {
      enfocarPrimerError();
    }
  };

  const responder = async (id: string, respuesta: RespuestaInvitacion) => {
    setExito(null);
    const r = await filas.enviar(() => responderAmistadCommand({ amistad_id: id, respuesta }), id);
    if (r.ok) {
      setExito(respuesta === 'aceptar' ? '¡Listo! Ahora son amigos.' : 'Invitación rechazada.');
      recargar();
    }
  };

  const eliminar = async (id: string) => {
    setExito(null);
    const r = await filas.enviar(() => eliminarAmistadCommand(id), id);
    if (r.ok) {
      setExito('Amistad eliminada. Los grupos y gastos que compartían siguen igual.');
      recargar();
    }
  };

  const errorGeneral = busqueda.errores.general ?? invitacion.errores.general ?? filas.errores.general;

  return (
    <div>
      <EncabezadoSocial
        modoOscuro={modoOscuro}
        titulo="Tu gente"
        subtitulo="Amigos / Conecta con quien tú elijas, sin directorios públicos."
      />

      {exito && <AvisoSocial modoOscuro={modoOscuro} tono="exito" className="mb-4">{exito}</AvisoSocial>}
      {errorGeneral && <AvisoSocial modoOscuro={modoOscuro} tono="ambar" className="mb-4">{errorGeneral}</AvisoSocial>}

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
        <div className="min-w-0 space-y-4">
          {estado.tipo === 'cargando' && <CargandoSocial modoOscuro={modoOscuro} />}
          {estado.tipo === 'error' && (
            <AvisoSocial modoOscuro={modoOscuro} tono="ambar" accion={{ texto: 'Reintentar', onClick: recargar }}>
              {estado.mensaje}
            </AvisoSocial>
          )}
          {estado.tipo === 'listo' && (
            <>
              <ListaAmigos
                modoOscuro={modoOscuro}
                amigos={estado.datos.amigos}
                enCurso={filas.enCurso}
                onEliminar={eliminar}
              />
              <InvitacionesAmistad
                modoOscuro={modoOscuro}
                recibidas={estado.datos.recibidas}
                enviadas={estado.datos.enviadas}
                enCurso={filas.enCurso}
                onResponder={responder}
              />
            </>
          )}
        </div>

        <BuscadorUsuario
          modoOscuro={modoOscuro}
          valor={usuario}
          onCambio={(v) => {
            setUsuario(v);
            setEncontrado(null);
            busqueda.limpiar();
          }}
          onBuscar={buscar}
          buscando={busqueda.cargando}
          error={busqueda.errores.usuario ?? invitacion.errores.usuario}
          encontrado={encontrado}
          onInvitar={invitar}
          invitando={invitacion.cargando}
        />
      </div>
    </div>
  );
}
