'use client';

import React, { useCallback, useState } from 'react';
import { invitarAlGrupoCommand, verGrupoCommand } from '../../utils/tauriCommands';
import { useCargaSocial } from '../../utils/useCargaSocial';
import { useEnvio } from '../../utils/useEnvio';
import { enfocarPrimerError } from '../../utils/enfocar';
import type { CampoUsuario } from '../../types/social';
import { DetalleGrupo } from './DetalleGrupo';
import { AvisoSocial, CargandoSocial, EncabezadoSocial } from './ElementosSociales';

const CAMPOS_USUARIO: readonly CampoUsuario[] = ['usuario'];

interface Props {
  modoOscuro: boolean;
  grupoId: string;
  onVolver: () => void;
  onNuevoGasto: () => void;
}

/** Detalle del grupo conectado a `ver_grupo` e `invitar_al_grupo`. */
export function DetalleGrupoConectado({ modoOscuro, grupoId, onVolver, onNuevoGasto }: Props) {
  const cargar = useCallback(() => verGrupoCommand(grupoId), [grupoId]);
  const { estado, recargar } = useCargaSocial(cargar);
  const [usuario, setUsuario] = useState('');
  const [exito, setExito] = useState<string | null>(null);
  const invitacion = useEnvio<CampoUsuario>(CAMPOS_USUARIO);

  const invitar = async () => {
    setExito(null);
    const r = await invitacion.enviar(() =>
      invitarAlGrupoCommand({ grupo_id: grupoId, usuario: usuario.trim().toLowerCase() })
    );
    if (r.ok) {
      setExito(`Invitación enviada a @${r.valor.usuario}. Entrará al grupo cuando la acepte.`);
      setUsuario('');
    } else {
      enfocarPrimerError();
    }
  };

  if (estado.tipo === 'cargando') {
    return (
      <div>
        <EncabezadoSocial modoOscuro={modoOscuro} titulo="Grupo" volver={{ texto: 'Volver a grupos', onClick: onVolver }} />
        <CargandoSocial modoOscuro={modoOscuro} />
      </div>
    );
  }

  if (estado.tipo === 'error') {
    return (
      <div>
        <EncabezadoSocial modoOscuro={modoOscuro} titulo="Grupo" volver={{ texto: 'Volver a grupos', onClick: onVolver }} />
        <AvisoSocial modoOscuro={modoOscuro} tono="ambar" accion={{ texto: 'Reintentar', onClick: recargar }}>
          {estado.mensaje}
        </AvisoSocial>
      </div>
    );
  }

  return (
    <div>
      {invitacion.errores.general && (
        <AvisoSocial modoOscuro={modoOscuro} tono="ambar" className="mb-4">
          {invitacion.errores.general}
        </AvisoSocial>
      )}
      <DetalleGrupo
        modoOscuro={modoOscuro}
        detalle={estado.datos}
        onVolver={onVolver}
        onNuevoGasto={onNuevoGasto}
        invitar={{
          valor: usuario,
          onCambio: (v) => {
            setUsuario(v);
            setExito(null);
            invitacion.limpiar();
          },
          onEnviar: invitar,
          cargando: invitacion.cargando,
          error: invitacion.errores.usuario,
          exito,
        }}
      />
    </div>
  );
}
