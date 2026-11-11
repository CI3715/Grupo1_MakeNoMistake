'use client';

import React, { useEffect, useState } from 'react';
import { CampoRegistro, DatosRegistro, ErroresSesion } from '../../types/sesion';
import { temaSesion } from '../../utils/temaSesion';
import { CampoTexto } from './CampoTexto';
import { BotonEnviar } from './BotonEnviar';

interface Props {
  modoOscuro: boolean;
  /** Errores por campo (y «general») que entrega quien conecta la pantalla. */
  errores?: ErroresSesion<CampoRegistro>;
  cargando?: boolean;
  onEnviar?: (datos: DatosRegistro) => void;
  onIrAInicioSesion: () => void;
}

/** Pantalla de registro. Solo presenta y recoge entrada: las reglas las valida el servidor. */
export const PantallaRegistro: React.FC<Props> = ({
  modoOscuro,
  errores = {},
  cargando = false,
  onEnviar,
  onIrAInicioSesion,
}) => {
  const [nombreUsuario, setNombreUsuario] = useState('');
  const [nombreVisible, setNombreVisible] = useState('');
  const [correo, setCorreo] = useState('');
  const [contrasena, setContrasena] = useState('');
  const tema = temaSesion(modoOscuro);

  // Tras un error, el foco va al primer campo que falló.
  useEffect(() => {
    const primero = (['nombre_usuario', 'nombre_visible', 'correo', 'contrasena'] as const).find(
      (c) => errores[c]
    );
    if (primero) document.getElementById(primero)?.focus();
  }, [errores]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onEnviar?.({
      nombre_usuario: nombreUsuario,
      nombre_visible: nombreVisible,
      correo,
      contrasena,
    });
  };

  return (
    <div className="w-full max-w-md mx-auto pt-6">
      <div className={`p-7 rounded-2xl border shadow-sm ${tema.tarjeta}`}>
        <h2 className="text-2xl font-bold tracking-tight">Crear usuario</h2>
        <p className={`text-xs mt-1 mb-6 ${tema.textoSuave}`}>
          Solo tu identidad se guarda en el servidor. Tu libro personal sigue siendo privado y vive en tu dispositivo.
        </p>

        <form onSubmit={handleSubmit} noValidate className="space-y-4">
          <CampoTexto
            id="nombre_usuario"
            etiqueta="Nombre de usuario"
            placeholder="ej. ana.perez"
            autoComplete="username"
            autoFocus
            ayuda="Letras minúsculas, números, punto y guion bajo."
            valor={nombreUsuario}
            onCambio={setNombreUsuario}
            error={errores.nombre_usuario}
            deshabilitado={cargando}
            modoOscuro={modoOscuro}
          />
          <CampoTexto
            id="nombre_visible"
            etiqueta="Nombre visible"
            placeholder="Como te verán tus amigos"
            autoComplete="name"
            valor={nombreVisible}
            onCambio={setNombreVisible}
            error={errores.nombre_visible}
            deshabilitado={cargando}
            modoOscuro={modoOscuro}
          />
          <CampoTexto
            id="correo"
            etiqueta="Correo"
            tipo="email"
            placeholder="tu@correo.com"
            autoComplete="email"
            valor={correo}
            onCambio={setCorreo}
            error={errores.correo}
            deshabilitado={cargando}
            modoOscuro={modoOscuro}
          />
          <CampoTexto
            id="contrasena"
            etiqueta="Contraseña"
            tipo="password"
            autoComplete="new-password"
            ayuda="Al menos 10 caracteres."
            valor={contrasena}
            onCambio={setContrasena}
            error={errores.contrasena}
            deshabilitado={cargando}
            modoOscuro={modoOscuro}
          />

          {errores.general && (
            <div role="alert" className="p-3 rounded-xl border border-rose-800/60 bg-rose-950/40 text-rose-300 text-xs">
              {errores.general}
            </div>
          )}

          <BotonEnviar cargando={cargando} textoNormal="Crear usuario" textoCargando="Creando usuario…" />
        </form>

        <p className={`text-xs text-center mt-5 ${tema.textoSuave}`}>
          ¿Ya tienes usuario?{' '}
          <button type="button" onClick={onIrAInicioSesion} className={`rounded font-semibold underline cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-500 ${tema.textoEnlace}`}>
            Iniciar sesión
          </button>
        </p>
      </div>

      <p className={`text-[11px] text-center mt-4 ${tema.textoSuave}`}>
        Registrarte es opcional: puedes usar toda la aplicación sin usuario.
      </p>
    </div>
  );
};
