'use client';

import React, { useEffect, useRef, useState } from 'react';
import { CampoRegistro, DatosRegistro, ErroresSesion } from '../../types/sesion';
import { registrarUsuarioCommand } from '../../utils/tauriCommands';
import { erroresDeFormulario } from '../../utils/erroresSesion';
import { PantallaRegistro } from './PantallaRegistro';

const CAMPOS: readonly CampoRegistro[] = ['nombre_usuario', 'nombre_visible', 'correo', 'contrasena'];

interface Props {
  modoOscuro: boolean;
  /** Registro correcto: recibe el correo con el que se creó el usuario. */
  onRegistrado: (correo: string) => void;
  onIrAInicioSesion: () => void;
}

/**
 * Conecta la pantalla de registro con `registrar_usuario`. Aquí solo hay estado de
 * carga y errores; las reglas (3 a 30 caracteres, 10 de contraseña, duplicados) las
 * decide y las valida el servidor, y llegan como errores por campo.
 */
export function RegistroConectado({ modoOscuro, onRegistrado, onIrAInicioSesion }: Props) {
  const [cargando, setCargando] = useState(false);
  const [errores, setErrores] = useState<ErroresSesion<CampoRegistro>>({});
  const montado = useRef(true);

  useEffect(() => {
    montado.current = true;
    return () => {
      montado.current = false;
    };
  }, []);

  const enviar = async (datos: DatosRegistro) => {
    if (cargando) return;
    setCargando(true);
    setErrores({});
    try {
      const usuario = await registrarUsuarioCommand(datos);
      if (montado.current) onRegistrado(usuario.correo);
    } catch (err) {
      if (montado.current) setErrores(erroresDeFormulario(err, CAMPOS));
    } finally {
      if (montado.current) setCargando(false);
    }
  };

  return (
    <PantallaRegistro
      modoOscuro={modoOscuro}
      errores={errores}
      cargando={cargando}
      onEnviar={enviar}
      onIrAInicioSesion={onIrAInicioSesion}
    />
  );
}
