'use client';

import React, { useState } from 'react';
import { VistaSesionCompleta } from '../../types/sesion';
import type { ControlSesion } from '../../utils/useSesion';
import { InicioSesionConectado } from './InicioSesionConectado';
import { PantallaPerfil } from './PantallaPerfil';
import { RegistroConectado } from './RegistroConectado';

interface Props {
  modoOscuro: boolean;
  vista: VistaSesionCompleta;
  setPantallaActual: (pantalla: string) => void;
  control: ControlSesion;
}

/**
 * Punto único de entrada de las pantallas de sesión para page.tsx:
 * inicio de sesión, registro y perfil. Los comandos (invoke) se llaman desde
 * los componentes «Conectado»; page.tsx no crece.
 */
export function PantallaSesion({ modoOscuro, vista, setPantallaActual, control }: Props) {
  const [correoNuevo, setCorreoNuevo] = useState<string | null>(null);

  // Con la sesión activa, «Iniciar sesión» no tiene sentido: se muestra el perfil.
  if (vista === 'perfil' || (vista === 'inicio-sesion' && control.sesion.estado === 'vigente')) {
    return <PantallaPerfil modoOscuro={modoOscuro} control={control} setPantallaActual={setPantallaActual} />;
  }

  if (vista === 'registro') {
    return (
      <RegistroConectado
        modoOscuro={modoOscuro}
        onRegistrado={(correo) => {
          setCorreoNuevo(correo);
          setPantallaActual('inicio-sesion');
        }}
        onIrAInicioSesion={() => setPantallaActual('inicio-sesion')}
      />
    );
  }

  return (
    <InicioSesionConectado
      modoOscuro={modoOscuro}
      control={control}
      correoInicial={correoNuevo}
      setPantallaActual={setPantallaActual}
      onIrARegistro={() => {
        setCorreoNuevo(null);
        setPantallaActual('registro');
      }}
    />
  );
}
