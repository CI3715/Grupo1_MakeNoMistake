#!/usr/bin/env bash
# Lista de verificación de calidad de la Entrega 6.
# Ejecutar desde la RAÍZ del repositorio:  bash verificar-entrega6.sh
# Corre todas las comprobaciones aunque alguna falle, guarda la salida completa
# en verificacion-entrega6.log (no se sube al repositorio) y muestra un resumen.
# Diferencias con la Entrega 5: se añaden los vectores de división (app y
# servidor), las pruebas de la división y de datos privados, y una comprobación
# de que existen pruebas basadas en propiedades (hypothesis).
# Nota: `npm audit`, `cargo audit` y `uv audit` devuelven error si hay
# vulnerabilidades conocidas; eso no es un fallo del script: es lo que va a
# docs/deuda-tecnica.md.
# No uses `npm audit fix --force`.
 
RAIZ="$(pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
LOG="$RAIZ/verificacion-entrega6.log"
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

# --- App (Rust y TypeScript) ---
paso "cargo fmt --all --check"                  app/src-tauri cargo fmt --all --check
paso "cargo clippy -D warnings"                 app/src-tauri cargo clippy --workspace --all-targets -- -D warnings
paso "cargo test --workspace"                   app/src-tauri cargo test --workspace
paso "cargo test --test arquitectura"           app/src-tauri cargo test --test arquitectura
paso "cargo test --test vectores_division"      app/src-tauri cargo test --test vectores_division
paso "cargo audit"                              app/src-tauri cargo audit
paso "npm audit"                                app           npm audit
paso "npx tsc --noEmit"                         app           npx tsc --noEmit
paso "npm run lint"                             app           npm run lint
paso "npm run build"                            app           npm run build

# --- Servidor (Python) ---
paso "uv run ruff check (servidor)"             servidor      uv run ruff check .
paso "uv run ruff format --check (servidor)"    servidor      uv run ruff format --check .
paso "uv run mypy (servidor)"                   servidor      uv run mypy
paso "uv run pytest (servidor)"                 servidor      uv run pytest
paso "pytest vectores de división"              servidor      uv run pytest -q tests/test_vectores_division.py
paso "pytest división de gastos"                servidor      uv run pytest -q tests/test_division_gastos.py
paso "pytest datos privados no se filtran"      servidor      uv run pytest -q tests/test_datos_privados_no_se_filtran.py
paso "hay pruebas de propiedades (hypothesis)"  servidor      grep -rq "@given" tests
paso "uv audit (servidor)"                      servidor      uv audit

echo
echo "================ RESUMEN ================"
printf '%s\n' "${RESUMEN[@]}"
echo "Fallas: $FALLAS   (detalle en $LOG)"
exit "$FALLAS"