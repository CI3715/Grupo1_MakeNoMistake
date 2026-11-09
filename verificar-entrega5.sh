#!/usr/bin/env bash
# Lista de verificación de calidad de la Entrega 5.
# Ejecutar desde la RAÍZ del repositorio:  bash verificar-entrega5.sh
# Corre todas las comprobaciones aunque alguna falle, guarda la salida completa
# en verificacion-entrega5.log (no se sube al repositorio) y muestra un resumen.
# Diferencias con la Entrega 4: en el servidor ya no se corre solo pytest; se
# añaden ruff check, ruff format --check, mypy y uv audit.
# Nota: `npm audit`, `cargo audit` y `uv audit` devuelven error si hay
# vulnerabilidades conocidas; eso no es un fallo del script: es lo que va a
# docs/deuda-tecnica.md.
# No uses `npm audit fix --force`.
 
RAIZ="$(pwd)"
LOG="$RAIZ/verificacion-entrega5.log"
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
paso "npm run build"                            app           npm run build
paso "uv run ruff check (servidor)"             servidor      uv run ruff check .
paso "uv run ruff format --check (servidor)"    servidor      uv run ruff format --check .
paso "uv run mypy (servidor)"                   servidor      uv run mypy
paso "uv run pytest (servidor)"                 servidor      uv run pytest
paso "uv audit (servidor)"                      servidor      uv audit
 
echo
echo "================ RESUMEN ================"
printf '%s\n' "${RESUMEN[@]}"
echo "Fallas: $FALLAS   (detalle en $LOG)"
exit "$FALLAS"