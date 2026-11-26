'use client';

import React, { useState } from 'react';
import type { ActualizarCredencialesPayload, ErrorIntercambio } from '../../types/copia';
import { CopiaAvisoError } from '../copia/CopiaAvisoError';
import { CampoTexto } from '../sesion/CampoTexto';
import { BotonAccion } from '../social/BotonAccion';

interface Props {
  modoOscuro: boolean;
  usuarioActual: string | null;
  onGuardar: (datos: ActualizarCredencialesPayload) => Promise<ErrorIntercambio | null>;
  onCerrar: () => void;
}

/**
 * Actualiza usuario y token del repositorio Git sin tocar el resto del destino. Es lo que se
 * hace cuando el repositorio rechaza las credenciales (sección 2.7). El token se borra de la
 * pantalla al guardar.
 */
export function SincronizacionLibroCredenciales({
  modoOscuro,
  usuarioActual,
  onGuardar,
  onCerrar,
}: Props) {
  const [usuario, setUsuario] = useState(usuarioActual ?? '');
  const [token, setToken] = useState('');
  const [intentado, setIntentado] = useState(false);
  const [guardando, setGuardando] = useState(false);
  const [error, setError] = useState<ErrorIntercambio | null>(null);

  const errorUsuario = usuario.trim() === '' ? 'Completa este dato.' : undefined;
  const errorToken = token === '' ? 'Completa este dato.' : undefined;

  const guardar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (guardando) return;
    setIntentado(true);
    setError(null);
    if (errorUsuario || errorToken) return;
    setGuardando(true);
    const resultado = await onGuardar({ usuario: usuario.trim(), token });
    setGuardando(false);
    if (resultado) {
      setError(resultado);
      return;
    }
    setToken('');
    onCerrar();
  };

  return (
    <form onSubmit={guardar} noValidate className="space-y-3">
      <CampoTexto
        id="credenciales-usuario"
        etiqueta="Usuario"
        valor={usuario}
        onCambio={setUsuario}
        autoComplete="off"
        error={intentado ? errorUsuario : undefined}
        deshabilitado={guardando}
        autoFocus
        modoOscuro={modoOscuro}
      />
      <CampoTexto
        id="credenciales-token"
        etiqueta="Token de acceso nuevo"
        tipo="password"
        valor={token}
        onCambio={setToken}
        autoComplete="off"
        ayuda="Reemplaza al anterior en el almacén seguro de tu sistema."
        error={intentado ? errorToken : undefined}
        deshabilitado={guardando}
        modoOscuro={modoOscuro}
      />
      {error && <CopiaAvisoError modoOscuro={modoOscuro} error={error} contexto="sincronizacion" />}
      <div className="flex flex-wrap gap-2">
        <BotonAccion modoOscuro={modoOscuro} tipo="submit" cargando={guardando}>
          {guardando ? 'Guardando…' : 'Guardar credenciales'}
        </BotonAccion>
        <BotonAccion
          modoOscuro={modoOscuro}
          variante="secundario"
          deshabilitado={guardando}
          onClick={onCerrar}
        >
          Cancelar
        </BotonAccion>
      </div>
    </form>
  );
}
