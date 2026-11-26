'use client';

import React, { useId, useState } from 'react';
import type { ErrorIntercambio } from '../../types/copia';
import { elegirDondeGuardarCopia, MENSAJE_SELECTOR_NO_DISPONIBLE } from '../../utils/copiaDialogos';
import { largoDeFrase, LARGO_MINIMO_FRASE, MENSAJE_FRASE_CORTA } from '../../utils/copiaFormato';
import { clasificarErrorIntercambio } from '../../utils/erroresCopia';
import { exportarCopiaCommand } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { CampoTexto } from '../sesion/CampoTexto';
import { BotonAccion } from '../social/BotonAccion';
import { Tarjeta } from '../social/Tarjeta';
import { CopiaAvisoError } from './CopiaAvisoError';
import { CopiaCampoRuta } from './CopiaCampoRuta';

interface Exito {
  ruta: string;
  cifrada: boolean;
}

/**
 * «Exportar copia de seguridad» (RF-INT-01, RF-INT-06). Cifrada con frase de paso (escrita dos
 * veces, mínimo 12 caracteres) o, con una advertencia clara, en texto legible. Exportar no
 * modifica el libro. La frase se borra del estado en cuanto la copia se guarda.
 */
export function CopiaExportar({ modoOscuro }: { modoOscuro: boolean }) {
  const tema = temaSocial(modoOscuro);
  const idCifrada = useId();
  const idLegible = useId();
  const idRiesgo = useId();

  const [cifrar, setCifrar] = useState(true);
  const [frase, setFrase] = useState('');
  const [confirmacion, setConfirmacion] = useState('');
  const [entiendeRiesgo, setEntiendeRiesgo] = useState(false);
  const [ruta, setRuta] = useState<string | null>(null);
  const [intentado, setIntentado] = useState(false);
  const [exportando, setExportando] = useState(false);
  const [error, setError] = useState<ErrorIntercambio | null>(null);
  const [avisoSelector, setAvisoSelector] = useState<string | null>(null);
  const [exito, setExito] = useState<Exito | null>(null);

  const errorFrase = !cifrar
    ? undefined
    : frase.length === 0
      ? 'Escribe una frase de paso.'
      : largoDeFrase(frase) < LARGO_MINIMO_FRASE
        ? MENSAJE_FRASE_CORTA
        : undefined;
  const errorConfirmacion =
    cifrar && !errorFrase && confirmacion !== frase ? 'Las dos frases no coinciden.' : undefined;
  const errorRuta = ruta === null ? 'Elige dónde guardar el archivo.' : undefined;
  const errorRiesgo = !cifrar && !entiendeRiesgo ? 'Confirma que entiendes el riesgo.' : undefined;
  const hayErrores = Boolean(errorFrase || errorConfirmacion || errorRuta || errorRiesgo);

  const cambiarModo = (cifrada: boolean) => {
    if (cifrada === cifrar) return;
    setCifrar(cifrada);
    // La extensión del archivo depende del modo: se vuelve a elegir para que coincida.
    setRuta(null);
    setError(null);
    setExito(null);
    setIntentado(false);
  };

  const elegirUbicacion = async () => {
    setAvisoSelector(null);
    try {
      const elegida = await elegirDondeGuardarCopia(cifrar);
      if (elegida !== null) setRuta(elegida);
    } catch {
      setAvisoSelector(MENSAJE_SELECTOR_NO_DISPONIBLE);
    }
  };

  const exportar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (exportando) return;
    setIntentado(true);
    setError(null);
    setExito(null);
    if (hayErrores || ruta === null) return;

    setExportando(true);
    try {
      await exportarCopiaCommand(ruta, cifrar ? frase : null);
      setExito({ ruta, cifrada: cifrar });
      setFrase('');
      setConfirmacion('');
      setEntiendeRiesgo(false);
      setRuta(null);
      setIntentado(false);
    } catch (err) {
      setError(clasificarErrorIntercambio(err));
    } finally {
      setExportando(false);
    }
  };

  const opcion = (activa: boolean) =>
    `flex cursor-pointer items-start gap-2.5 rounded-xl border p-3 text-xs ${
      activa ? 'border-[#9333EA] ' + tema.avisoMorado : tema.linea + ' ' + tema.interior
    }`;

  return (
    <Tarjeta
      modoOscuro={modoOscuro}
      titulo="Exportar copia de seguridad"
      descripcion="Guarda todo tu libro en un solo archivo. Exportar no cambia nada de lo que ya tienes."
    >
      <form onSubmit={exportar} noValidate className="space-y-4">
        <fieldset className="space-y-2">
          <legend className={`mb-1 text-xs font-semibold ${tema.textoSuave}`}>
            Protección del archivo
          </legend>
          <label htmlFor={idCifrada} className={opcion(cifrar)}>
            <input
              id={idCifrada}
              type="radio"
              name="proteccion-copia"
              checked={cifrar}
              onChange={() => cambiarModo(true)}
              className="mt-0.5 accent-purple-600"
            />
            <span>
              <span className="block font-semibold">Cifrar con frase de paso</span>
              <span className={tema.textoSuave}>
                Sin la frase, nadie puede leer el archivo. Es la opción recomendada.
              </span>
            </span>
          </label>
          <label htmlFor={idLegible} className={opcion(!cifrar)}>
            <input
              id={idLegible}
              type="radio"
              name="proteccion-copia"
              checked={!cifrar}
              onChange={() => cambiarModo(false)}
              className="mt-0.5 accent-purple-600"
            />
            <span>
              <span className="block font-semibold">Exportar sin cifrar</span>
              <span className={tema.textoSuave}>
                Texto legible, útil para revisar tus datos con otra herramienta.
              </span>
            </span>
          </label>
        </fieldset>

        {cifrar ? (
          <div className="space-y-3">
            <div role="note" className={`rounded-xl border p-3 text-xs ${tema.avisoAmbar}`}>
              <p className="font-semibold">Si olvidas la frase, no hay forma de recuperar la copia.</p>
              <p className="mt-0.5 opacity-90">
                Cuentas Claras no la guarda ni puede restablecerla. Anótala en un lugar seguro.
              </p>
            </div>
            <CampoTexto
              id="copia-frase"
              etiqueta="Frase de paso"
              tipo="password"
              valor={frase}
              onCambio={setFrase}
              autoComplete="new-password"
              ayuda={`Mínimo ${LARGO_MINIMO_FRASE} caracteres.`}
              error={intentado ? errorFrase : undefined}
              deshabilitado={exportando}
              modoOscuro={modoOscuro}
            />
            <CampoTexto
              id="copia-frase-confirmacion"
              etiqueta="Repite la frase de paso"
              tipo="password"
              valor={confirmacion}
              onCambio={setConfirmacion}
              autoComplete="new-password"
              error={intentado ? errorConfirmacion : undefined}
              deshabilitado={exportando}
              modoOscuro={modoOscuro}
            />
          </div>
        ) : (
          <div className={`rounded-xl border p-3 text-xs ${tema.avisoError}`}>
            <p role="note" className="font-semibold">
              Cualquiera que abra este archivo verá todas tus cuentas y movimientos.
            </p>
            <p className="mt-0.5 opacity-90">
              Guárdalo solo en un lugar seguro y bórralo cuando ya no lo necesites.
            </p>
            <label htmlFor={idRiesgo} className="mt-3 flex cursor-pointer items-start gap-2">
              <input
                id={idRiesgo}
                type="checkbox"
                checked={entiendeRiesgo}
                onChange={(e) => setEntiendeRiesgo(e.target.checked)}
                disabled={exportando}
                aria-invalid={intentado && errorRiesgo ? true : undefined}
                className="mt-0.5 accent-purple-600"
              />
              <span>Entiendo que el archivo no estará protegido.</span>
            </label>
            {intentado && errorRiesgo && (
              <p role="alert" className="mt-1 text-[11px] font-medium">
                {errorRiesgo}
              </p>
            )}
          </div>
        )}

        <CopiaCampoRuta
          modoOscuro={modoOscuro}
          etiqueta="Dónde se guardará"
          ruta={ruta}
          textoBoton="Elegir ubicación…"
          onElegir={() => void elegirUbicacion()}
          deshabilitado={exportando}
          error={intentado ? errorRuta : (avisoSelector ?? undefined)}
        />

        {error && <CopiaAvisoError modoOscuro={modoOscuro} error={error} contexto="copia" />}

        {exito && (
          <div role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoOk}`}>
            <p className="font-semibold">
              {exito.cifrada ? 'Copia cifrada guardada.' : 'Copia guardada sin cifrar.'}
            </p>
            <p className="mt-0.5 break-all opacity-90">{exito.ruta}</p>
            <p className="mt-0.5 opacity-90">Tu libro no se modificó.</p>
          </div>
        )}

        <BotonAccion modoOscuro={modoOscuro} tipo="submit" cargando={exportando}>
          {exportando ? 'Exportando…' : 'Exportar copia'}
        </BotonAccion>
      </form>
    </Tarjeta>
  );
}
