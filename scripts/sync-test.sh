#!/usr/bin/env bash
# Prueba de extremo a extremo de la sincronización con dos instalaciones (Entrega 8,
# Ernesto - Tarea 5; flujo de la sección 2.8; ESC-03 y ESC-04).
#
# Ejecutar desde cualquier carpeta del repositorio:   bash scripts/sync-test.sh
#
# Qué hace: corre la prueba `dos_instalaciones_extremo_a_extremo` (dos instalaciones con
# carpetas de datos y bases SQLite distintas, cifrado real, una carpeta compartida real y
# un repositorio Git real que hace de remoto) y, aparte, la batería de contrato de los
# tres adaptadores del transporte. Imprime la tabla del ejemplo de convergencia de beto.
#
# Requisitos: Rust (cargo) y el programa `git`. No usa red ni el servidor.

set -u
export PATH="$HOME/.cargo/bin:$PATH"
RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NUCLEO="$RAIZ/app/src-tauri"
FALLAS=0

for herramienta in cargo git; do
  if ! command -v "$herramienta" >/dev/null 2>&1; then
    echo "Falta la herramienta: $herramienta" >&2
    exit 2
  fi
done

paso() { # paso "nombre" comando...
  local nombre="$1"
  shift
  echo
  echo "=== $nombre"
  if (cd "$NUCLEO" && "$@"); then
    echo "OK      $nombre"
  else
    echo "FALLA   $nombre"
    FALLAS=$((FALLAS + 1))
  fi
}

# 1. Flujo completo con dos instalaciones: carpeta y Git, en los dos órdenes.
paso "dos instalaciones: exportar, importar, restaurar y sincronizar (carpeta y Git)" \
  cargo test -p infraestructura --test dos_instalaciones_extremo_a_extremo -- --nocapture --test-threads=1

# 2. Los tres adaptadores superan la misma batería de contrato (RNF-14).
paso "contrato del transporte: adaptador falso" \
  cargo test -p aplicacion --test pruebas_transporte_memoria
paso "contrato del transporte: carpeta compartida" \
  cargo test -p infraestructura --test transporte_carpeta
paso "contrato del transporte: Git" \
  cargo test -p infraestructura --test transporte_git

# 3. El ejemplo de convergencia con el transporte falso y el caso de uso de sincronizar.
paso "convergencia en memoria (ejemplo de la sección 2.6)" \
  cargo test -p aplicacion --test pruebas_convergencia --test pruebas_sincronizar_libro

echo
if [ "$FALLAS" -eq 0 ]; then
  echo "Todo en orden: las dos instalaciones convergen, repetir no cambia nada y restaurar funciona."
else
  echo "Pasos con fallas: $FALLAS"
fi
exit "$FALLAS"
