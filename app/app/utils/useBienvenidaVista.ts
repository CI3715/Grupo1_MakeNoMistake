import { useSyncExternalStore } from 'react';

/**
 * Indica si la bienvenida ya se cerró EN ESTA EJECUCIÓN de la app.
 * Vive solo en memoria (no usa localStorage): al cerrar y volver a abrir la app
 * se reinicia y la bienvenida sale otra vez. Solo presentación; nada del libro.
 */
let vista = false;
const oyentes = new Set<() => void>();

function suscribir(avisar: () => void) {
  oyentes.add(avisar);
  return () => {
    oyentes.delete(avisar);
  };
}

function leer(): boolean {
  return vista;
}

function marcarVista() {
  vista = true;
  oyentes.forEach((avisar) => avisar());
}

export function useBienvenidaVista(): [boolean, () => void] {
  // Servidor / primer render: false, para que la bienvenida salga desde el arranque.
  const yaVista = useSyncExternalStore(suscribir, leer, () => false);
  return [yaVista, marcarVista];
}
