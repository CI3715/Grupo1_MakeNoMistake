import { open, save } from '@tauri-apps/plugin-dialog';
import { fechaHoyLocal } from './formato';
import { SIMULAR_INTERCAMBIO } from './tauriCommands';

/*
 * Diálogos nativos para elegir dónde guardar o de dónde leer (decisión: plugin de diálogos de
 * Tauri). Las pantallas muestran la ruta elegida en un campo de solo lectura, así la persona
 * confirma exactamente dónde se guardará o de dónde se leerá el archivo.
 *
 * En modo simulación (`npm run dev` fuera de Tauri) no hay ventana nativa: se devuelven rutas
 * de mentira para poder probar el flujo.
 */

export const EXTENSION_COPIA_CIFRADA = 'ccbak';
export const EXTENSION_COPIA_LEGIBLE = 'txt';

/** Devuelve la ruta elegida, o `null` si la persona cerró el diálogo sin elegir. */
export async function elegirDondeGuardarCopia(cifrada: boolean): Promise<string | null> {
  const extension = cifrada ? EXTENSION_COPIA_CIFRADA : EXTENSION_COPIA_LEGIBLE;
  const nombre = `cuentas-claras-${fechaHoyLocal()}.${extension}`;
  if (SIMULAR_INTERCAMBIO) return `/simulado/${nombre}`;
  return await save({
    title: 'Guardar copia de seguridad',
    defaultPath: nombre,
    filters: [
      {
        name: cifrada ? 'Copia cifrada de Cuentas Claras' : 'Copia en texto legible',
        extensions: [extension],
      },
    ],
  });
}

/** Devuelve la ruta del archivo elegido, o `null` si se cerró el diálogo. */
export async function elegirArchivoDeCopia(): Promise<string | null> {
  if (SIMULAR_INTERCAMBIO) return '/simulado/copia-simulada.ccbak';
  const elegido = await open({
    title: 'Elegir copia de seguridad',
    multiple: false,
    directory: false,
    filters: [
      {
        name: 'Copias de Cuentas Claras',
        extensions: [EXTENSION_COPIA_CIFRADA, EXTENSION_COPIA_LEGIBLE],
      },
    ],
  });
  return typeof elegido === 'string' ? elegido : null;
}

/** Devuelve la carpeta elegida como destino de sincronización, o `null`. */
export async function elegirCarpetaDeSincronizacion(): Promise<string | null> {
  if (SIMULAR_INTERCAMBIO) return '/simulado/carpeta-compartida';
  const elegida = await open({
    title: 'Elegir carpeta compartida',
    multiple: false,
    directory: true,
  });
  return typeof elegida === 'string' ? elegida : null;
}

/** Texto cuando el selector del sistema no se pudo abrir. */
export const MENSAJE_SELECTOR_NO_DISPONIBLE =
  'No se pudo abrir el selector de archivos. Intenta de nuevo.';
