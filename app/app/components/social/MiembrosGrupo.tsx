'use client';

import React, { useEffect, useId, useState } from 'react';
import { CampoTexto } from '../sesion/CampoTexto';
import type { ContextoSocial, ErroresCampos, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import { normalizarUsuario } from '../../utils/formatoSocial';
import {
  invitarAlGrupoCommand,
  listarAmistadesCommand,
  type GrupoResumenDto,
  type MiembroDto,
} from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useCargaRemota, useMontado } from '../../utils/useCargaRemota';
import { Avatar } from './Avatar';
import { BotonAccion } from './BotonAccion';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

const CAMPOS: readonly 'usuario'[] = ['usuario'];

interface Props {
  ctx: ContextoSocial;
  grupo: GrupoResumenDto;
  miembros: MiembroDto[];
}

/** Miembros del grupo y, para el administrador, invitar a un amigo (RF-COM-02). */
export function MiembrosGrupo({ ctx, grupo, miembros }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  const esAdministrador = grupo.rol === 'administrador';

  return (
    <Tarjeta modoOscuro={ctx.modoOscuro} titulo={`Miembros · ${miembros.length}`}>
      <ul>
        {miembros.map((m) => (
          <li
            key={m.usuario}
            className={`flex items-center gap-3 border-b py-2.5 last:border-b-0 ${tema.linea}`}
          >
            <Avatar nombre={m.nombre_visible} />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-semibold">{m.nombre_visible}</p>
              <p className={`truncate text-[11px] ${tema.textoSuave}`}>
                @{m.usuario} · {m.rol === 'administrador' ? 'Administrador' : 'Miembro'}
              </p>
            </div>
          </li>
        ))}
      </ul>

      <div className={`mt-4 border-t pt-4 ${tema.linea}`}>
        {esAdministrador ? (
          <FormularioInvitar ctx={ctx} grupoId={grupo.id} miembros={miembros} />
        ) : (
          <p className={`text-xs ${tema.textoSuave}`}>
            Solo la persona administradora puede invitar a otros amigos al grupo.
          </p>
        )}
      </div>
    </Tarjeta>
  );
}

function FormularioInvitar({
  ctx,
  grupoId,
  miembros,
}: {
  ctx: ContextoSocial;
  grupoId: string;
  miembros: MiembroDto[];
}) {
  const [usuario, setUsuario] = useState('');
  const [enviando, setEnviando] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [errores, setErrores] = useState<ErroresCampos<'usuario'>>({});
  const [tipoError, setTipoError] = useState<TipoFallo>('otro');
  const montado = useMontado();
  const idLista = useId();
  const tema = temaSocial(ctx.modoOscuro);

  // Sugerencias: tus amigos que todavía no están en el grupo. Si no cargan, el campo
  // sigue funcionando: se puede escribir cualquier nombre de usuario.
  const { carga } = useCargaRemota(listarAmistadesCommand);
  const sugerencias =
    carga.estado === 'listo'
      ? carga.datos.amigos.filter(
          (a) => !miembros.some((m) => m.usuario === a.usuario.nombre_usuario)
        )
      : [];

  useEffect(() => {
    if (errores.usuario) document.getElementById('invitar-grupo-usuario')?.focus();
  }, [errores]);

  const enviar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (enviando) return;
    setEnviando(true);
    setErrores({});
    setAviso(null);
    try {
      const invitado = await invitarAlGrupoCommand({
        grupo_id: grupoId,
        usuario: normalizarUsuario(usuario),
      });
      if (!montado.current) return;
      setAviso(`Invitación enviada a ${invitado.nombre_visible}. Verá el grupo cuando la acepte.`);
      setUsuario('');
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, CAMPOS);
        setErrores(r.errores);
        setTipoError(r.tipo);
      }
    } finally {
      if (montado.current) setEnviando(false);
    }
  };

  return (
    <form onSubmit={enviar} noValidate className="space-y-3">
      <CampoTexto
        id="invitar-grupo-usuario"
        etiqueta="Invitar a un amigo"
        placeholder="@usuario"
        autoComplete="off"
        listaId={idLista}
        ayuda="Solo puedes invitar a tus amigos. Cada uno debe aceptar para entrar."
        valor={usuario}
        onCambio={(t) => {
          setUsuario(t);
          setAviso(null);
        }}
        error={errores.usuario}
        deshabilitado={enviando}
        modoOscuro={ctx.modoOscuro}
      />
      <datalist id={idLista}>
        {sugerencias.map((a) => (
          <option key={a.amistad_id} value={a.usuario.nombre_usuario}>
            {a.usuario.nombre_visible}
          </option>
        ))}
      </datalist>

      {errores.general && <MensajeGeneral ctx={ctx} mensaje={errores.general} tipo={tipoError} />}
      {aviso && (
        <p role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoOk}`}>
          {aviso}
        </p>
      )}

      <BotonAccion modoOscuro={ctx.modoOscuro} tipo="submit" cargando={enviando} variante="secundario" ancho>
        {enviando ? 'Enviando…' : 'Enviar invitación'}
      </BotonAccion>
    </form>
  );
}
