'use client';

import React, { useState } from 'react';
import { CampoInicioSesion, DatosInicioSesion, ErroresSesion } from '../../types/sesion';
import { temaSesion } from '../../utils/temaSesion';
import { CampoTexto } from './CampoTexto';
import { BotonEnviar } from './BotonEnviar';

interface Props {
  modoOscuro: boolean;
  /** Errores por campo (y «general») que entrega quien conecta la pantalla. */
  errores?: ErroresSesion<CampoInicioSesion>;
  cargando?: boolean;
  onEnviar?: (datos: DatosInicioSesion) => void;
  onIrARegistro: () => void;
}

/** Pantalla de inicio de sesión. Solo presenta y recoge entrada: no valida reglas. */
export const PantallaInicioSesion: React.FC<Props> = ({
  modoOscuro,
  errores = {},
  cargando = false,
  onEnviar,
  onIrARegistro,
}) => {
  const [correo, setCorreo] = useState('');
  const [contrasena, setContrasena] = useState('');
  const tema = temaSesion(modoOscuro);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onEnviar?.({ correo, contrasena });
  };

  return (
    <div className="w-full max-w-md mx-auto pt-6">
      <div className={`p-7 rounded-2xl border shadow-sm ${tema.tarjeta}`}>
        <h2 className="text-2xl font-bold tracking-tight">Iniciar sesión</h2>
        <p className={`text-xs mt-1 mb-6 ${tema.textoSuave}`}>
          Identifícate para usar las funciones compartidas. Tus cuentas y movimientos no salen de tu dispositivo.
        </p>

        <form onSubmit={handleSubmit} noValidate className="space-y-4">
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
            autoComplete="current-password"
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

          <BotonEnviar cargando={cargando} textoNormal="Iniciar sesión" textoCargando="Iniciando sesión…" />
        </form>

        <p className={`text-xs text-center mt-5 ${tema.textoSuave}`}>
          ¿No tienes usuario?{' '}
          <button type="button" onClick={onIrARegistro} className={`font-semibold underline cursor-pointer ${tema.textoEnlace}`}>
            Crear usuario
          </button>
        </p>
      </div>

      <p className={`text-[11px] text-center mt-4 ${tema.textoSuave}`}>
        Iniciar sesión es opcional: tus cuentas, movimientos, tasas y patrimonio funcionan sin usuario y sin conexión.
      </p>
    </div>
  );
};
