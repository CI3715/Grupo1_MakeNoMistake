'use client';

import React, { useState } from 'react';
import type { ErrorIntercambio, ResumenImportacionDto } from '../../types/copia';
import { elegirArchivoDeCopia, MENSAJE_SELECTOR_NO_DISPONIBLE } from '../../utils/copiaDialogos';
import { clasificarErrorIntercambio } from '../../utils/erroresCopia';
import { importarCopiaCommand } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { CampoTexto } from '../sesion/CampoTexto';
import { BotonAccion } from '../social/BotonAccion';
import { Tarjeta } from '../social/Tarjeta';
import { CopiaAvisoError } from './CopiaAvisoError';
import { CopiaCampoRuta } from './CopiaCampoRuta';
import { CopiaResumenImportacion } from './CopiaResumenImportacion';

interface Props {
  modoOscuro: boolean;
  /** Se llama cuando la importación terminó, para que las demás pantallas vuelvan a leer el libro. */
  alCambiarLibro: () => void;
}

/**
 * «Importar copia de seguridad» (RF-INT-02). Suma al libro lo que falte: nunca borra ni
 * modifica lo que ya hay, y importar el mismo archivo dos veces no cambia nada la segunda.
 * Una frase incorrecta o un archivo dañado no cambian el libro. La frase se borra del estado
 * al terminar.
 */
export function CopiaImportar({ modoOscuro, alCambiarLibro }: Props) {
  const tema = temaSocial(modoOscuro);
  const [ruta, setRuta] = useState<string | null>(null);
  const [frase, setFrase] = useState('');
  const [intentado, setIntentado] = useState(false);
  const [importando, setImportando] = useState(false);
  const [error, setError] = useState<ErrorIntercambio | null>(null);
  const [avisoSelector, setAvisoSelector] = useState<string | null>(null);
  const [resumen, setResumen] = useState<ResumenImportacionDto | null>(null);

  const elegirArchivo = async () => {
    setAvisoSelector(null);
    try {
      const elegido = await elegirArchivoDeCopia();
      if (elegido !== null) {
        setRuta(elegido);
        setResumen(null);
        setError(null);
      }
    } catch {
      setAvisoSelector(MENSAJE_SELECTOR_NO_DISPONIBLE);
    }
  };

  const importar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (importando) return;
    setIntentado(true);
    setError(null);
    setResumen(null);
    if (ruta === null) return;

    setImportando(true);
    try {
      const resultado = await importarCopiaCommand(ruta, frase === '' ? null : frase);
      setResumen(resultado);
      setFrase('');
      setIntentado(false);
      alCambiarLibro();
    } catch (err) {
      setError(clasificarErrorIntercambio(err));
    } finally {
      setImportando(false);
    }
  };

  // Pedir la frase es un caso del campo, no un fallo general.
  const pideFrase = error?.tipo === 'frase_requerida';

  return (
    <Tarjeta
      modoOscuro={modoOscuro}
      titulo="Importar copia de seguridad"
      descripcion="Restaura tu libro desde un archivo exportado, por ejemplo en una instalación nueva."
    >
      <form onSubmit={importar} noValidate className="space-y-4">
        <p role="note" className={`rounded-xl border p-3 text-xs ${tema.avisoMorado}`}>
          Importar suma a tu libro lo que falte y no borra nada de lo que ya tienes. Si importas el
          mismo archivo otra vez, no se duplica nada.
        </p>

        <CopiaCampoRuta
          modoOscuro={modoOscuro}
          etiqueta="Archivo de la copia"
          ruta={ruta}
          textoBoton="Elegir archivo…"
          onElegir={() => void elegirArchivo()}
          deshabilitado={importando}
          error={
            intentado && ruta === null
              ? 'Elige el archivo que quieres importar.'
              : (avisoSelector ?? undefined)
          }
        />

        <CampoTexto
          id="copia-frase-importar"
          etiqueta="Frase de paso"
          tipo="password"
          valor={frase}
          onCambio={setFrase}
          autoComplete="off"
          ayuda="Déjala vacía si la copia se exportó sin cifrar."
          error={pideFrase ? error?.mensaje : undefined}
          deshabilitado={importando}
          modoOscuro={modoOscuro}
        />

        {error && !pideFrase && (
          <CopiaAvisoError modoOscuro={modoOscuro} error={error} contexto="copia" />
        )}

        {resumen && <CopiaResumenImportacion modoOscuro={modoOscuro} resumen={resumen} />}

        <BotonAccion modoOscuro={modoOscuro} tipo="submit" cargando={importando}>
          {importando ? 'Importando…' : 'Importar copia'}
        </BotonAccion>
      </form>
    </Tarjeta>
  );
}
