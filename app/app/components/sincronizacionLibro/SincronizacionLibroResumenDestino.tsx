'use client';

import React, { useState } from 'react';
import type { ActualizarCredencialesPayload, DestinoSincronizacionDto, ErrorIntercambio } from '../../types/copia';
import { temaSocial } from '../../utils/temaSocial';
import { CopiaAvisoError } from '../copia/CopiaAvisoError';
import { BotonAccion } from '../social/BotonAccion';
import { SincronizacionLibroCredenciales } from './SincronizacionLibroCredenciales';

interface Props {
  modoOscuro: boolean;
  destino: DestinoSincronizacionDto;
  /** Abre el formulario de credenciales desde fuera (aviso de credenciales rechazadas). */
  editandoCredenciales: boolean;
  onEditarCredenciales: (abierto: boolean) => void;
  onCambiar: () => void;
  onGuardarCredenciales: (datos: ActualizarCredencialesPayload) => Promise<ErrorIntercambio | null>;
  onQuitar: () => Promise<ErrorIntercambio | null>;
}

/**
 * Destino ya configurado: dónde está, qué se guardó de forma segura (sin mostrarlo) y las
 * acciones de cambiarlo, actualizar credenciales (Git) o quitarlo. Quitar pide confirmación.
 */
export function SincronizacionLibroResumenDestino({
  modoOscuro,
  destino,
  editandoCredenciales,
  onEditarCredenciales,
  onCambiar,
  onGuardarCredenciales,
  onQuitar,
}: Props) {
  const tema = temaSocial(modoOscuro);
  const [confirmando, setConfirmando] = useState(false);
  const [quitando, setQuitando] = useState(false);
  const [errorQuitar, setErrorQuitar] = useState<ErrorIntercambio | null>(null);
  const esGit = destino.tipo === 'git';

  const quitar = async () => {
    setQuitando(true);
    const error = await onQuitar();
    setQuitando(false);
    if (error) {
      setErrorQuitar(error);
      setConfirmando(false);
    }
  };

  return (
    <div className="space-y-4">
      <dl className="space-y-2 text-xs">
        <div>
          <dt className={`font-semibold ${tema.textoSuave}`}>Tipo</dt>
          <dd className="mt-0.5">
            <span className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${tema.chip}`}>
              {esGit ? 'Repositorio Git' : 'Carpeta compartida'}
            </span>
          </dd>
        </div>
        <div className="min-w-0">
          <dt className={`font-semibold ${tema.textoSuave}`}>
            {esGit ? 'Repositorio' : 'Carpeta'}
          </dt>
          <dd className="mt-0.5 break-all">{destino.ubicacion}</dd>
        </div>
        {esGit && destino.usuario && (
          <div>
            <dt className={`font-semibold ${tema.textoSuave}`}>Usuario</dt>
            <dd className="mt-0.5">{destino.usuario}</dd>
          </div>
        )}
        <div>
          <dt className={`font-semibold ${tema.textoSuave}`}>Secretos</dt>
          <dd className="mt-0.5">
            {esGit
              ? 'La frase de paso y el token están en el almacén seguro de tu sistema.'
              : 'La frase de paso está en el almacén seguro de tu sistema.'}
          </dd>
        </div>
      </dl>

      {editandoCredenciales && esGit ? (
        <SincronizacionLibroCredenciales
          modoOscuro={modoOscuro}
          usuarioActual={destino.usuario}
          onGuardar={onGuardarCredenciales}
          onCerrar={() => onEditarCredenciales(false)}
        />
      ) : (
        <div className="flex flex-wrap gap-2">
          <BotonAccion modoOscuro={modoOscuro} variante="secundario" onClick={onCambiar}>
            Cambiar destino
          </BotonAccion>
          {esGit && (
            <BotonAccion
              modoOscuro={modoOscuro}
              variante="secundario"
              onClick={() => onEditarCredenciales(true)}
            >
              Actualizar credenciales
            </BotonAccion>
          )}
          {!confirmando && (
            <BotonAccion
              modoOscuro={modoOscuro}
              variante="peligro"
              onClick={() => {
                setErrorQuitar(null);
                setConfirmando(true);
              }}
            >
              Quitar destino
            </BotonAccion>
          )}
        </div>
      )}

      {confirmando && (
        <div role="alertdialog" aria-label="Confirmar quitar destino" className={`rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}>
          <p className="font-semibold">¿Quitar este destino?</p>
          <p className="mt-0.5 opacity-90">
            Se borran de este dispositivo la frase y las credenciales guardadas. Tu libro y los
            archivos que ya están en el destino no se tocan.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <BotonAccion modoOscuro={modoOscuro} variante="peligro" cargando={quitando} onClick={() => void quitar()}>
              Sí, quitar destino
            </BotonAccion>
            <BotonAccion
              modoOscuro={modoOscuro}
              variante="secundario"
              deshabilitado={quitando}
              onClick={() => setConfirmando(false)}
            >
              Cancelar
            </BotonAccion>
          </div>
        </div>
      )}

      {errorQuitar && (
        <CopiaAvisoError modoOscuro={modoOscuro} error={errorQuitar} contexto="sincronizacion" />
      )}
    </div>
  );
}
