#!/usr/bin/env bash
# Lista de verificación de calidad de la Entrega 3.
# Ejecutar desde la RAÍZ del repositorio:  bash verificar-entrega3.sh
# Corre todas las comprobaciones aunque alguna falle, guarda la salida completa
# en verificacion-entrega3.log y muestra un resumen al final.
# Nota: `npm audit` y `cargo audit` devuelven error si hay vulnerabilidades
# conocidas; eso no es un fallo del script: es lo que va a docs/deuda-tecnica.md
# (D-01 y D-02). 
 
RAIZ="$(pwd)"
LOG="$RAIZ/verificacion-entrega3.log"
: > "$LOG"
RESUMEN=()
FALLAS=0
 
paso() { # paso "nombre" carpeta comando...
  local nombre="$1" carpeta="$2"
  shift 2
  echo "=== $nombre  ($carpeta)" | tee -a "$LOG"
  if (cd "$RAIZ/$carpeta" && "$@") >> "$LOG" 2>&1; then
    RESUMEN+=("OK      $nombre")
  else
    RESUMEN+=("FALLA   $nombre")
    FALLAS=$((FALLAS + 1))
  fi
}
 
paso "cargo fmt --all --check"                  app/src-tauri cargo fmt --all --check
paso "cargo clippy -D warnings"                 app/src-tauri cargo clippy --workspace --all-targets -- -D warnings
paso "cargo test --workspace"                   app/src-tauri cargo test --workspace
paso "cargo test --test arquitectura"           app/src-tauri cargo test --test arquitectura
paso "cargo audit"                              app/src-tauri cargo audit
paso "npm audit"                                app           npm audit
paso "npx tsc --noEmit"                         app           npx tsc --noEmit
paso "npm run lint"                             app           npm run lint
paso "uv run pytest (servidor)"                 servidor      uv run pytest
 
echo
echo "================ RESUMEN ================"
printf '%s\n' "${RESUMEN[@]}"
echo "Fallas: $FALLAS   (detalle en $LOG)"
exit "$FALLAS"