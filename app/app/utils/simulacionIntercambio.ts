/**
 * SIMULACIÓN para desarrollo (Entrega 8). Solo se usa con `NEXT_PUBLIC_SIMULAR_INTERCAMBIO=1`.
 *
 * Reemplaza a los comandos de Rust que todavía se están integrando (cifrado real, importar,
 * destino y sincronizar) para poder construir y probar la interfaz, incluidos los cinco
 * errores de la sección 2.7. No escribe ni lee archivos, no toca el libro y no hace red.
 *
 * El «resultado simulado» se elige con el selector de `CopiaAvisoSimulacion`. Los mensajes
 * repiten los del núcleo (`aplicacion::respaldo` y `aplicacion::transporte`).
 *
 * Importa de `tauriCommands` solo TIPOS (se borran al compilar): no hay ciclo en ejecución.
 */

import type {
  ActualizarCredencialesPayload,
  ConfigurarDestinoPayload,
  DestinoSincronizacionDto,
  EstadoSincronizacionLibroDto,
  InformeSincronizacionLibroDto,
  ResumenImportacionDto,
} from '../types/copia';
import type { ErroresDto } from './tauriCommands';

export type EscenarioSimulado =
  | 'exito'
  | 'frase_incorrecta'
  | 'frase_requerida'
  | 'version_no_soportada'
  | 'archivo_invalido'
  | 'evento_rechazado'
  | 'destino_inaccesible'
  | 'sin_conexion'
  | 'credenciales_rechazadas';

export const ESCENARIOS_SIMULADOS: { valor: EscenarioSimulado; texto: string }[] = [
  { valor: 'exito', texto: 'Todo sale bien' },
  { valor: 'frase_incorrecta', texto: 'Frase incorrecta o archivo alterado' },
  { valor: 'frase_requerida', texto: 'El archivo está cifrado y falta la frase' },
  { valor: 'version_no_soportada', texto: 'Versión de formato no soportada' },
  { valor: 'archivo_invalido', texto: 'Archivo que no es una copia' },
  { valor: 'evento_rechazado', texto: 'Evento rechazado con motivo' },
  { valor: 'destino_inaccesible', texto: 'Destino inaccesible' },
  { valor: 'sin_conexion', texto: 'Sin conexión' },
  { valor: 'credenciales_rechazadas', texto: 'Credenciales de Git rechazadas' },
];

const TOTAL_EVENTOS = 44;
const ESPERA_MS = 600;

let escenario: EscenarioSimulado = 'exito';
let destino: DestinoSincronizacionDto | null = null;
let ultimaSincronizacion: string | null = null;
let sincronizacionesExitosas = 0;
const archivosImportados = new Set<string>();

export function escenarioSimulado(): EscenarioSimulado {
  return escenario;
}

export function elegirEscenarioSimulado(nuevo: EscenarioSimulado): void {
  escenario = nuevo;
}

function esperar(): Promise<void> {
  return new Promise((resolver) => setTimeout(resolver, ESPERA_MS));
}

function fallar(codigo: string, mensaje: string): never {
  const error: ErroresDto = { errores: [{ campo: null, codigo, mensaje }] };
  throw error;
}

function fallarSegunEscenarioDeDestino(): void {
  switch (escenario) {
    case 'destino_inaccesible':
      fallar(
        'destino_inaccesible',
        'No se pudo llegar al repositorio o a la carpeta; se reintentará.'
      );
      break;
    case 'sin_conexion':
      fallar(
        'sin_conexion',
        'Sin conexión: no se pudo llegar al repositorio o a la carpeta. Se reintentará.'
      );
      break;
    case 'credenciales_rechazadas':
      fallar(
        'credenciales_rechazadas',
        'Las credenciales fueron rechazadas. Actualízalas en la configuración del destino de sincronización.'
      );
      break;
    default:
  }
}

function validarFrase(frase: string): void {
  if ([...frase].length < 12) {
    fallar('frase_corta', 'La frase de paso debe tener al menos 12 caracteres.');
  }
}

function ahoraIso(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
}

// ---------- Copia de seguridad ----------

export async function exportarCopia(_ruta: string, frase: string | null): Promise<void> {
  await esperar();
  if (frase !== null) validarFrase(frase);
}

export async function importarCopia(
  ruta: string,
  frase: string | null
): Promise<ResumenImportacionDto> {
  await esperar();
  switch (escenario) {
    case 'frase_incorrecta':
      return fallar('frase_incorrecta', 'Frase de paso incorrecta o archivo dañado.');
    case 'frase_requerida':
      if (!frase) {
        return fallar('frase_requerida', 'Este archivo está cifrado: escribe la frase de paso.');
      }
      break;
    case 'version_no_soportada':
      return fallar(
        'version_no_soportada',
        'El archivo es de la versión 3 del formato y esta aplicación soporta la 1.'
      );
    case 'archivo_invalido':
      return fallar(
        'archivo_invalido',
        'El archivo no tiene el formato de una copia de seguridad de Cuentas Claras.'
      );
    case 'evento_rechazado':
      return {
        nuevos: TOTAL_EVENTOS - 3,
        ya_existian: 2,
        rechazados: [
          {
            id: 'evento-0007',
            motivo: 'Ya existe un evento con ese identificador y un contenido distinto.',
          },
        ],
      };
    default:
  }
  if (archivosImportados.has(ruta)) {
    return { nuevos: 0, ya_existian: TOTAL_EVENTOS, rechazados: [] };
  }
  archivosImportados.add(ruta);
  return { nuevos: TOTAL_EVENTOS, ya_existian: 0, rechazados: [] };
}

// ---------- Sincronización del libro ----------

export async function estadoSincronizacionLibro(): Promise<EstadoSincronizacionLibroDto> {
  return { destino, ultima_sincronizacion: ultimaSincronizacion };
}

export async function configurarDestino(
  datos: ConfigurarDestinoPayload
): Promise<DestinoSincronizacionDto> {
  await esperar();
  validarFrase(datos.frase);
  fallarSegunEscenarioDeDestino();
  destino = {
    tipo: datos.tipo,
    ubicacion: datos.ubicacion,
    usuario: datos.tipo === 'git' ? datos.usuario : null,
  };
  ultimaSincronizacion = null;
  sincronizacionesExitosas = 0;
  return destino;
}

export async function actualizarCredenciales(datos: ActualizarCredencialesPayload): Promise<void> {
  await esperar();
  if (escenario === 'credenciales_rechazadas') fallarSegunEscenarioDeDestino();
  if (destino?.tipo === 'git') destino = { ...destino, usuario: datos.usuario };
}

export async function quitarDestino(): Promise<void> {
  await esperar();
  destino = null;
  ultimaSincronizacion = null;
  sincronizacionesExitosas = 0;
}

/**
 * La primera sincronización envía 2 y recibe 2 (el ejemplo de convergencia de la sección 2.6);
 * las siguientes no tienen nada que enviar ni recibir.
 */
export async function sincronizarLibro(): Promise<InformeSincronizacionLibroDto> {
  await esperar();
  if (!destino) {
    fallar('sin_destino', 'Todavía no hay un destino de sincronización configurado.');
  }
  fallarSegunEscenarioDeDestino();
  const primera = sincronizacionesExitosas === 0;
  sincronizacionesExitosas += 1;
  ultimaSincronizacion = ahoraIso();
  return {
    enviados: primera ? 2 : 0,
    recibidos: primera ? 2 : 0,
    rechazados:
      escenario === 'evento_rechazado'
        ? [
            {
              id: 'evento-0012',
              motivo: 'Ya existe un evento con ese identificador y un contenido distinto.',
            },
          ]
        : [],
    ultima_sincronizacion: ultimaSincronizacion,
  };
}
