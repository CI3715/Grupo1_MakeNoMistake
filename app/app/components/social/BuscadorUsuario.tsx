'use client';

import React, { useState } from 'react';
import { CampoTexto } from '../sesion/CampoTexto';
import type { ContextoSocial, ErroresCampos, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import { normalizarUsuario } from '../../utils/formatoSocial';
import {
  buscarUsuarioCommand,
  invitarAmigoCommand,
  type UsuarioPublicoDto,
} from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useMontado } from '../../utils/useCargaRemota';
import { IconoBuscar } from '../Iconos';
import { BotonAccion } from './BotonAccion';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

const CAMPOS: readonly 'usuario'[] = ['usuario'];

interface Props {
  ctx: ContextoSocial;
  /** Se llama tras enviar una invitación, para actualizar las listas. */
  onInvitada: () => void;
}

/**
 * Buscador por nombre de usuario EXACTO (RF-SOC-04). No hay búsqueda parcial ni listados:
 * la interfaz solo envía el texto y muestra lo que devuelve el comando (nunca un correo).
 */
export function BuscadorUsuario({ ctx, onInvitada }: Props) {
  const [usuario, setUsuario] = useState('');
  const [buscando, setBuscando] = useState(false);
  const [invitando, setInvitando] = useState(false);
  const [encontrado, setEncontrado] = useState<UsuarioPublicoDto | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);
  const [errores, setErrores] = useState<ErroresCampos<'usuario'>>({});
  const [tipoError, setTipoError] = useState<TipoFallo>('otro');
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  const alEscribir = (texto: string) => {
    setUsuario(texto);
    setEncontrado(null); // el resultado era de otro texto
    setAviso(null);
    setErrores({});
  };

  const buscar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (buscando) return;
    setBuscando(true);
    setErrores({});
    setAviso(null);
    setEncontrado(null);
    try {
      const resultado = await buscarUsuarioCommand({ usuario: normalizarUsuario(usuario) });
      if (montado.current) setEncontrado(resultado);
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, CAMPOS);
        setErrores(r.errores);
        setTipoError(r.tipo);
      }
    } finally {
      if (montado.current) setBuscando(false);
    }
  };

  const invitar = async () => {
    if (!encontrado || invitando) return;
    setInvitando(true);
    setErrores({});
    try {
      await invitarAmigoCommand({ usuario: encontrado.nombre_usuario });
      if (!montado.current) return;
      setAviso(`Invitación enviada a ${encontrado.nombre_visible} (@${encontrado.nombre_usuario}).`);
      setEncontrado(null);
      setUsuario('');
      onInvitada();
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, CAMPOS);
        setErrores(r.errores);
        setTipoError(r.tipo);
      }
    } finally {
      if (montado.current) setInvitando(false);
    }
  };

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Añadir amigo"
      descripcion="Busca a una persona por su nombre de usuario exacto."
    >
      <form onSubmit={buscar} noValidate className="space-y-3">
        <CampoTexto
          id="buscar-usuario"
          etiqueta="Nombre de usuario exacto"
          placeholder="beto"
          autoComplete="off"
          ayuda="No se busca por correo ni por coincidencias parciales."
          valor={usuario}
          onCambio={alEscribir}
          error={errores.usuario}
          deshabilitado={buscando || invitando}
          modoOscuro={ctx.modoOscuro}
        />
        <BotonAccion
          modoOscuro={ctx.modoOscuro}
          tipo="submit"
          cargando={buscando}
          deshabilitado={invitando}
          ancho
        >
          <span className="flex items-center justify-center gap-1.5">
            {!buscando && <IconoBuscar className="w-3.5 h-3.5 shrink-0" />}
            <span>{buscando ? 'Buscando…' : 'Buscar usuario exacto'}</span>
          </span>
        </BotonAccion>
      </form>

      {errores.general && (
        <div className="mt-3">
          <MensajeGeneral ctx={ctx} mensaje={errores.general} tipo={tipoError} />
        </div>
      )}

      {aviso && (
        <p
          role="status"
          className={`mt-3 rounded-xl border p-3 text-xs ${tema.avisoOk}`}
        >
          {aviso}
        </p>
      )}

      {encontrado && (
        <div role="status" className={`mt-3 rounded-xl border p-3 text-xs ${tema.avisoMorado}`}>
          <p className="font-semibold">Usuario encontrado</p>
          <p className="mt-1 break-words">
            @{encontrado.nombre_usuario} · {encontrado.nombre_visible}
          </p>
          <p className="mt-1 opacity-80">Solo se muestra el resultado que coincide exactamente.</p>
          <div className="mt-3">
            <BotonAccion
              modoOscuro={ctx.modoOscuro}
              cargando={invitando}
              onClick={invitar}
              ancho
            >
              {invitando ? 'Enviando…' : 'Enviar invitación'}
            </BotonAccion>
          </div>
        </div>
      )}
    </Tarjeta>
  );
}
