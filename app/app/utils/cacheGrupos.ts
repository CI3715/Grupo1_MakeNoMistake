import { clasificarFallo } from './erroresSocial';
import { verGrupoCommand, type GrupoDetalleDto } from './tauriCommands';

/**
 * Último detalle de cada grupo que se pudo leer. Solo existe para que, sin conexión, el
 * formulario de gasto y el detalle sigan abiertos con lo último que se vio (RNF-17, ESC-09):
 * el gasto registrado así queda en la cola y se envía al volver la red.
 * Vive en memoria y se borra al iniciar o cerrar sesión: nunca pasa de un usuario a otro.
 */
const vistos = new Map<string, GrupoDetalleDto>();

export function olvidarGruposVistos(): void {
  vistos.clear();
}

/** Como `verGrupoCommand`, pero sin conexión devuelve lo último visto de ese grupo (si hay). */
export async function verGrupoConCache(grupoId: string): Promise<GrupoDetalleDto> {
  try {
    const grupo = await verGrupoCommand(grupoId);
    vistos.set(grupoId, grupo);
    return grupo;
  } catch (err) {
    const visto = vistos.get(grupoId);
    if (visto && clasificarFallo(err).tipo === 'sin_conexion') return visto;
    throw err;
  }
}
