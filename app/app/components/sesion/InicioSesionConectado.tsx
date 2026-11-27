'use client';

import React, { useEffect, useRef, useState } from 'react';
import { CampoInicioSesion, DatosInicioSesion, ErroresSesion } from '../../types/sesion';
import { iniciarSesionCommand } from '../../utils/tauriCommands';
import { erroresDeFormulario } from '../../utils/erroresSesion';
import type { ControlSesion } from '../../utils/useSesion';
import { PantallaInicioSesion } from './PantallaInicioSesion';

const CAMPOS: readonly CampoInicioSesion[] = ['correo', 'contrasena'];

interface Props {
  modoOscuro: boolean;
  control: ControlSesion;
  /** Correo recién registrado, para no tener que escribirlo otra vez. */
  correoInicial?: string | null;
  setPantallaActual: (pantalla: string) => void;
  onIrARegistro: () => void;
}

/**
 * Conecta la pantalla de inicio de sesión con `iniciar_sesion`. El token se queda en
 * Rust; aquí solo llega el estado de la sesión y el nombre visible.
 */
export function InicioSesionConectado({
  modoOscuro,
  control,
  correoInicial = null,
  setPantallaActual,
  onIrARegistro,
}: Props) {
  const [cargando, setCargando] = useState(false);
  const [errores, setErrores] = useState<ErroresSesion<CampoInicioSesion>>({});
  const montado = useRef(true);

  useEffect(() => {
    montado.current = true;
    return () => {
      montado.current = false;
    };
  }, []);

  const enviar = async (datos: DatosInicioSesion) => {
    if (cargando) return;
    setCargando(true);
    setErrores({});
    try {
      const sesion = await iniciarSesionCommand(datos);
      control.alIniciarSesion(sesion); // la sesión existe aunque la persona haya cambiado de pantalla
      if (montado.current) setPantallaActual('perfil');
    } catch (err) {
      if (montado.current) setErrores(erroresDeFormulario(err, CAMPOS));
    } finally {
      if (montado.current) setCargando(false);
    }
  };

  return (
    <PantallaInicioSesion
      modoOscuro={modoOscuro}
      errores={errores}
      cargando={cargando}
      correoInicial={correoInicial ?? ''}
      aviso={correoInicial ? 'Usuario creado. Ahora inicia sesión para continuar.' : undefined}
      onEnviar={enviar}
      onIrARegistro={onIrARegistro}
    />
  );
}
