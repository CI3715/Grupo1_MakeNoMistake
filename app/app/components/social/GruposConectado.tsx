'use client';

import React, { useState } from 'react';
import {
  crearGrupoCommand,
  listarGruposCommand,
  responderInvitacionGrupoCommand,
  type RespuestaInvitacion,
} from '../../utils/tauriCommands';
import { useCargaSocial } from '../../utils/useCargaSocial';
import { useEnvio } from '../../utils/useEnvio';
import { enfocarPrimerError } from '../../utils/enfocar';
import { GRUPO_VACIO, type CampoGrupo, type ValoresGrupo, type VistaGrupos } from '../../types/social';
import { AvisoSocial, CargandoSocial, EncabezadoSocial, TarjetaCifra } from './ElementosSociales';
import { FormularioGrupo } from './FormularioGrupo';
import { InvitacionesGrupo } from './InvitacionesGrupo';
import { ListaGrupos } from './ListaGrupos';
import { DetalleGrupoConectado } from './DetalleGrupoConectado';
import { NuevoGastoConectado } from './NuevoGastoConectado';

const CAMPOS_GRUPO: readonly CampoGrupo[] = ['nombre', 'descripcion', 'moneda', 'actividad'];

/**
 * Pantalla «Grupos». Lleva la navegación interna (lista → detalle → nuevo gasto) con estado
 * local para que page.tsx no crezca (RES-06).
 */
export function GruposConectado({ modoOscuro }: { modoOscuro: boolean }) {
  const [vista, setVista] = useState<VistaGrupos>({ tipo: 'lista' });

  if (vista.tipo === 'detalle') {
    return (
      <DetalleGrupoConectado
        key={vista.grupoId}
        modoOscuro={modoOscuro}
        grupoId={vista.grupoId}
        onVolver={() => setVista({ tipo: 'lista' })}
        onNuevoGasto={() => setVista({ tipo: 'nuevo-gasto', grupoId: vista.grupoId })}
      />
    );
  }
  if (vista.tipo === 'nuevo-gasto') {
    return (
      <NuevoGastoConectado
        key={vista.grupoId}
        modoOscuro={modoOscuro}
        grupoId={vista.grupoId}
        onVolver={() => setVista({ tipo: 'detalle', grupoId: vista.grupoId })}
        onRegistrado={() => setVista({ tipo: 'detalle', grupoId: vista.grupoId })}
      />
    );
  }
  return <ListaGruposConectada modoOscuro={modoOscuro} onAbrir={(id) => setVista({ tipo: 'detalle', grupoId: id })} />;
}

function ListaGruposConectada({ modoOscuro, onAbrir }: { modoOscuro: boolean; onAbrir: (grupoId: string) => void }) {
  const { estado, recargar } = useCargaSocial(listarGruposCommand);
  const [valores, setValores] = useState<ValoresGrupo>(GRUPO_VACIO);
  const [exito, setExito] = useState<string | null>(null);
  const creacion = useEnvio<CampoGrupo>(CAMPOS_GRUPO);
  const respuesta = useEnvio();

  const crear = async () => {
    setExito(null);
    const r = await creacion.enviar(() =>
      crearGrupoCommand({
        nombre: valores.nombre.trim(),
        descripcion: valores.descripcion.trim(),
        moneda: valores.moneda.trim().toUpperCase(),
        actividad: valores.actividad.trim(),
      })
    );
    if (r.ok) {
      setValores(GRUPO_VACIO);
      onAbrir(r.valor.id); // el grupo nuevo se abre para invitar a los amigos
    } else {
      enfocarPrimerError();
    }
  };

  const responder = async (grupoId: string, resp: RespuestaInvitacion) => {
    setExito(null);
    const r = await respuesta.enviar(() => responderInvitacionGrupoCommand({ grupo_id: grupoId, respuesta: resp }), grupoId);
    if (r.ok) {
      setExito(resp === 'aceptar' ? `Ya eres miembro de «${r.valor.nombre}».` : 'Invitación rechazada.');
      recargar();
    }
  };

  const errorGeneral = creacion.errores.general ?? respuesta.errores.general;
  const datos = estado.tipo === 'listo' ? estado.datos : null;

  return (
    <div>
      <EncabezadoSocial
        modoOscuro={modoOscuro}
        titulo="Compartir, sin enredos"
        subtitulo="Grupos / Tus actividades y gastos en común"
      />

      {exito && <AvisoSocial modoOscuro={modoOscuro} tono="exito" className="mb-4">{exito}</AvisoSocial>}
      {errorGeneral && <AvisoSocial modoOscuro={modoOscuro} tono="ambar" className="mb-4">{errorGeneral}</AvisoSocial>}

      {datos && (
        <div className="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2">
          <TarjetaCifra
            modoOscuro={modoOscuro}
            titulo="Grupos activos"
            cifra={`${datos.grupos.length} ${datos.grupos.length === 1 ? 'grupo' : 'grupos'}`}
            nota={datos.grupos.length === 0 ? 'Aún no estás en ningún grupo' : datos.grupos.map((g) => g.nombre).join(' · ')}
          />
          <TarjetaCifra
            modoOscuro={modoOscuro}
            titulo="Por revisar"
            cifra={`${datos.invitaciones.length} ${datos.invitaciones.length === 1 ? 'invitación' : 'invitaciones'}`}
            nota={datos.invitaciones.length === 0 ? 'Ningún grupo te espera' : 'Ningún grupo se añade sin aceptar'}
          />
        </div>
      )}

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
        <div className="min-w-0 space-y-4">
          {estado.tipo === 'cargando' && <CargandoSocial modoOscuro={modoOscuro} />}
          {estado.tipo === 'error' && (
            <AvisoSocial modoOscuro={modoOscuro} tono="ambar" accion={{ texto: 'Reintentar', onClick: recargar }}>
              {estado.mensaje}
            </AvisoSocial>
          )}
          {datos && (
            <>
              <ListaGrupos modoOscuro={modoOscuro} grupos={datos.grupos} onAbrir={onAbrir} />
              <InvitacionesGrupo
                modoOscuro={modoOscuro}
                invitaciones={datos.invitaciones}
                enCurso={respuesta.enCurso}
                onResponder={responder}
              />
            </>
          )}
        </div>

        <FormularioGrupo
          modoOscuro={modoOscuro}
          valores={valores}
          onCambio={(campo, valor) => setValores((v) => ({ ...v, [campo]: valor }))}
          errores={creacion.errores}
          cargando={creacion.cargando}
          onEnviar={crear}
        />
      </div>
    </div>
  );
}
