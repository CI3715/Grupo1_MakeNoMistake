'use client';

import React from 'react';
import type { VistaSocial } from '../../types/social';
import { BotonSocial, EncabezadoSocial, TarjetaSocial } from './ElementosSociales';
import { temaSocial } from '../../utils/temaSocial';
import { IconoAmigos, IconoRecibo } from '../Iconos';

interface Props {
  modoOscuro: boolean;
  vista: VistaSocial;
  /** La sesión venció: el mensaje es el acordado, «Tu sesión venció, inicia sesión de nuevo». */
  vencida: boolean;
  onIniciarSesion: () => void;
  onRegistrarse: () => void;
}

/**
 * Lo que ven Amigos y Grupos sin sesión: una invitación a iniciar sesión. No llama al servidor.
 * Todo lo personal (cuentas, movimientos, tasas, patrimonio) sigue funcionando.
 */
export const InvitacionIniciarSesion: React.FC<Props> = ({ modoOscuro, vista, vencida, onIniciarSesion, onRegistrarse }) => {
  const tema = temaSocial(modoOscuro);
  const esAmigos = vista === 'amigos';
  return (
    <div>
      <EncabezadoSocial
        modoOscuro={modoOscuro}
        titulo={esAmigos ? 'Tu gente' : 'Compartir, sin enredos'}
        subtitulo={esAmigos ? 'Amigos' : 'Grupos'}
      />
      <TarjetaSocial modoOscuro={modoOscuro} className="mx-auto max-w-md text-center">
        <div
          className={`mx-auto mb-3 flex h-14 w-14 items-center justify-center rounded-2xl ${
            modoOscuro ? 'bg-purple-950/60 border border-purple-800/40 text-purple-300' : 'bg-purple-100 border border-purple-200 text-[#9333EA]'
          }`}
          aria-hidden="true"
        >
          {esAmigos ? <IconoAmigos className="w-7 h-7" /> : <IconoRecibo className="w-7 h-7" />}
        </div>
        <h2 className="text-base font-semibold">
          {vencida ? 'Tu sesión venció, inicia sesión de nuevo.' : 'Inicia sesión para compartir gastos'}
        </h2>
        <p className={`mt-2 text-xs ${tema.textoSuave}`}>
          {esAmigos
            ? 'Para añadir amigos y responder invitaciones necesitas una cuenta registrada.'
            : 'Para crear grupos y dividir gastos con tus amigos necesitas una cuenta registrada.'}{' '}
          Tus cuentas, movimientos y tasas siguen en tu dispositivo y no cambian.
        </p>
        <div className="mt-4 flex flex-wrap justify-center gap-2">
          <BotonSocial modoOscuro={modoOscuro} onClick={onIniciarSesion}>
            Iniciar sesión
          </BotonSocial>
          {!vencida && (
            <BotonSocial modoOscuro={modoOscuro} variante="secundario" onClick={onRegistrarse}>
              Crear cuenta
            </BotonSocial>
          )}
        </div>
      </TarjetaSocial>
    </div>
  );
};
