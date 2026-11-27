'use client';

import React, { useId, useState } from 'react';
import type {
  ConfigurarDestinoPayload,
  DestinoSincronizacionDto,
  ErrorIntercambio,
  TipoDestino,
} from '../../types/copia';
import {
  elegirCarpetaDeSincronizacion,
  MENSAJE_SELECTOR_NO_DISPONIBLE,
} from '../../utils/copiaDialogos';
import { largoDeFrase, LARGO_MINIMO_FRASE, MENSAJE_FRASE_CORTA } from '../../utils/copiaFormato';
import { temaSocial } from '../../utils/temaSocial';
import { CopiaAvisoError } from '../copia/CopiaAvisoError';
import { CopiaCampoRuta } from '../copia/CopiaCampoRuta';
import { CampoTexto } from '../sesion/CampoTexto';
import { BotonAccion } from '../social/BotonAccion';

interface Props {
  modoOscuro: boolean;
  /** Destino actual, si se está cambiando (solo para proponer el mismo tipo). */
  actual: DestinoSincronizacionDto | null;
  /** Devuelve el error para mostrarlo aquí, o `null` si se guardó. */
  onGuardar: (datos: ConfigurarDestinoPayload) => Promise<ErrorIntercambio | null>;
  onCancelar?: () => void;
}

/**
 * Configura el destino de la sincronización: una carpeta compartida o un repositorio Git
 * propio (RF-INT-03, RF-INT-05, RF-INT-07). La frase de paso se pide dos veces; para unir un
 * dispositivo nuevo se usan el mismo destino y la misma frase. La frase y el token se envían
 * al núcleo, que los guarda en el almacén de secretos del sistema, y se borran de aquí.
 */
export function SincronizacionLibroDestinoFormulario({
  modoOscuro,
  actual,
  onGuardar,
  onCancelar,
}: Props) {
  const tema = temaSocial(modoOscuro);
  const idCarpeta = useId();
  const idGit = useId();

  const [tipo, setTipo] = useState<TipoDestino>(actual?.tipo ?? 'carpeta');
  const [carpeta, setCarpeta] = useState<string | null>(null);
  const [url, setUrl] = useState('');
  const [usuario, setUsuario] = useState('');
  const [token, setToken] = useState('');
  const [frase, setFrase] = useState('');
  const [confirmacion, setConfirmacion] = useState('');
  const [intentado, setIntentado] = useState(false);
  const [guardando, setGuardando] = useState(false);
  const [error, setError] = useState<ErrorIntercambio | null>(null);
  const [avisoSelector, setAvisoSelector] = useState<string | null>(null);

  const falta = 'Completa este dato.';
  const errorCarpeta = tipo === 'carpeta' && carpeta === null ? 'Elige la carpeta.' : undefined;
  const errorUrl = tipo === 'git' && url.trim() === '' ? falta : undefined;
  const errorUsuario = tipo === 'git' && usuario.trim() === '' ? falta : undefined;
  const errorToken = tipo === 'git' && token === '' ? falta : undefined;
  const errorFrase =
    frase.length === 0
      ? 'Escribe una frase de paso.'
      : largoDeFrase(frase) < LARGO_MINIMO_FRASE
        ? MENSAJE_FRASE_CORTA
        : undefined;
  const errorConfirmacion =
    !errorFrase && confirmacion !== frase ? 'Las dos frases no coinciden.' : undefined;
  const hayErrores = Boolean(
    errorCarpeta || errorUrl || errorUsuario || errorToken || errorFrase || errorConfirmacion
  );

  const elegirCarpeta = async () => {
    setAvisoSelector(null);
    try {
      const elegida = await elegirCarpetaDeSincronizacion();
      if (elegida !== null) setCarpeta(elegida);
    } catch {
      setAvisoSelector(MENSAJE_SELECTOR_NO_DISPONIBLE);
    }
  };

  const guardar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (guardando) return;
    setIntentado(true);
    setError(null);
    if (hayErrores) return;

    const datos: ConfigurarDestinoPayload =
      tipo === 'carpeta'
        ? { tipo, ubicacion: carpeta ?? '', frase }
        : { tipo, ubicacion: url.trim(), usuario: usuario.trim(), token, frase };

    setGuardando(true);
    const resultado = await onGuardar(datos);
    setGuardando(false);
    if (resultado) {
      setError(resultado);
      return;
    }
    // Se guardó: los secretos no se quedan en la pantalla.
    setFrase('');
    setConfirmacion('');
    setToken('');
  };

  const opcion = (activa: boolean) =>
    `flex cursor-pointer items-start gap-2.5 rounded-xl border p-3 text-xs ${
      activa ? 'border-acento ' + tema.avisoMorado : tema.linea + ' ' + tema.interior
    }`;

  return (
    <form onSubmit={guardar} noValidate className="space-y-4">
      <fieldset className="space-y-2">
        <legend className={`mb-1 text-xs font-semibold ${tema.textoSuave}`}>Tipo de destino</legend>
        <label htmlFor={idCarpeta} className={opcion(tipo === 'carpeta')}>
          <input
            id={idCarpeta}
            type="radio"
            name="tipo-destino"
            checked={tipo === 'carpeta'}
            onChange={() => setTipo('carpeta')}
            className="mt-0.5 accent-purple-600"
          />
          <span>
            <span className="block font-semibold">Carpeta compartida</span>
            <span className={tema.textoSuave}>
              Una carpeta que otro servicio mantiene al día: Syncthing, Dropbox o Google Drive.
            </span>
          </span>
        </label>
        <label htmlFor={idGit} className={opcion(tipo === 'git')}>
          <input
            id={idGit}
            type="radio"
            name="tipo-destino"
            checked={tipo === 'git'}
            onChange={() => setTipo('git')}
            className="mt-0.5 accent-purple-600"
          />
          <span>
            <span className="block font-semibold">Repositorio Git</span>
            <span className={tema.textoSuave}>Un repositorio privado tuyo, en el servicio que prefieras.</span>
          </span>
        </label>
      </fieldset>

      {tipo === 'carpeta' ? (
        <CopiaCampoRuta
          modoOscuro={modoOscuro}
          etiqueta="Carpeta compartida"
          ruta={carpeta}
          textoBoton="Elegir carpeta…"
          onElegir={() => void elegirCarpeta()}
          deshabilitado={guardando}
          error={intentado ? errorCarpeta : (avisoSelector ?? undefined)}
        />
      ) : (
        <div className="space-y-3">
          <CampoTexto
            id="destino-git-url"
            etiqueta="Dirección del repositorio"
            valor={url}
            onCambio={setUrl}
            placeholder="https://github.com/tu-usuario/mi-libro.git"
            autoComplete="off"
            error={intentado ? errorUrl : undefined}
            deshabilitado={guardando}
            modoOscuro={modoOscuro}
          />
          <CampoTexto
            id="destino-git-usuario"
            etiqueta="Usuario"
            valor={usuario}
            onCambio={setUsuario}
            autoComplete="off"
            error={intentado ? errorUsuario : undefined}
            deshabilitado={guardando}
            modoOscuro={modoOscuro}
          />
          <CampoTexto
            id="destino-git-token"
            etiqueta="Token de acceso"
            tipo="password"
            valor={token}
            onCambio={setToken}
            autoComplete="off"
            ayuda="Debe poder leer y escribir en el repositorio."
            error={intentado ? errorToken : undefined}
            deshabilitado={guardando}
            modoOscuro={modoOscuro}
          />
        </div>
      )}

      <div className="space-y-3">
        <CampoTexto
          id="destino-frase"
          etiqueta="Frase de paso"
          tipo="password"
          valor={frase}
          onCambio={setFrase}
          autoComplete="new-password"
          ayuda={`Mínimo ${LARGO_MINIMO_FRASE} caracteres. Para unir otro dispositivo, usa la misma.`}
          error={intentado ? errorFrase : undefined}
          deshabilitado={guardando}
          modoOscuro={modoOscuro}
        />
        <CampoTexto
          id="destino-frase-confirmacion"
          etiqueta="Repite la frase de paso"
          tipo="password"
          valor={confirmacion}
          onCambio={setConfirmacion}
          autoComplete="new-password"
          error={intentado ? errorConfirmacion : undefined}
          deshabilitado={guardando}
          modoOscuro={modoOscuro}
        />
      </div>

      <p role="note" className={`rounded-xl border p-3 text-xs ${tema.avisoMorado}`}>
        Los archivos del destino van cifrados con tu frase: quien tenga acceso al repositorio o a la
        carpeta no puede leer tu libro. La frase y el token se guardan en el almacén seguro de tu
        sistema, no en el libro ni en el destino. Si olvidas la frase, no se puede recuperar.
      </p>

      {error && (
        <CopiaAvisoError modoOscuro={modoOscuro} error={error} contexto="sincronizacion" />
      )}

      <div className="flex flex-wrap gap-2">
        <BotonAccion modoOscuro={modoOscuro} tipo="submit" cargando={guardando}>
          {guardando ? 'Guardando…' : 'Guardar destino'}
        </BotonAccion>
        {onCancelar && (
          <BotonAccion
            modoOscuro={modoOscuro}
            variante="secundario"
            deshabilitado={guardando}
            onClick={onCancelar}
          >
            Cancelar
          </BotonAccion>
        )}
      </div>
    </form>
  );
}
