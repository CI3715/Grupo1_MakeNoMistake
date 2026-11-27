#!/usr/bin/env bash
# Lista de verificación de calidad de la Entrega 8 (respaldo y sincronización del libro).
# Ejecutar desde la RAÍZ del repositorio:  bash verificar-entrega8.sh
# Corre todas las comprobaciones aunque alguna falle, guarda la salida completa en
# verificacion-entrega8.log (no se sube al repositorio) y muestra un resumen.
#
# Novedades respecto a la Entrega 7:
#  - Batería de contrato del transporte (falso, carpeta y Git) y prueba de extremo a
#    extremo con dos instalaciones (scripts/sync-test.sh). Estos pasos exigen que se
#    ejecute AL MENOS UNA prueba: un filtro que no coincide con ninguna no cuenta.
#  - Pruebas de fusión, cifrado, formato de exportación y privacidad del destino.
#  - La ADR de cada decisión de la entrega debe estar en docs/adr/ADR.md, y deben existir
#    docs/formato-exportacion.md y docs/guia-sincronizacion.md.
#  - Las pruebas de PostgreSQL real son opcionales aquí (EXIGIR_POSTGRES=1 para exigirlas):
#    la Entrega 8 no toca el servidor.
#
# Requisitos: cargo-audit, el programa `git` (adaptador Git y pruebas), Python 3.9+
# (reconstrucción independiente de saldos) y uv para las comprobaciones del servidor.
# No uses `npm audit fix --force`.

RAIZ="$(pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
LOG="$RAIZ/verificacion-entrega8.log"
: > "$LOG"
RESUMEN=()
FALLAS=0
EXIGIR_POSTGRES="${EXIGIR_POSTGRES:-0}"


registrar() { # registrar "nombre" código_de_salida
  if [ "$2" -eq 0 ]; then
    RESUMEN+=("OK      $1")
  else
    RESUMEN+=("FALLA   $1")
    FALLAS=$((FALLAS + 1))
  fi
}

paso() { # paso "nombre" carpeta comando...
  local nombre="$1" carpeta="$2"
  shift 2
  echo "=== $nombre  ($carpeta)" | tee -a "$LOG"
  (cd "$RAIZ/$carpeta" && "$@") >> "$LOG" 2>&1
  registrar "$nombre" $?
}

paso_cargo_filtro() { # paso_cargo_filtro "nombre" filtro   (exige >= 1 prueba)
  local nombre="$1" filtro="$2" salida estado ejecutadas
  echo "=== $nombre  (app/src-tauri, filtro \"$filtro\")" | tee -a "$LOG"
  salida="$(cd "$RAIZ/app/src-tauri" && cargo test --workspace "$filtro" 2>&1)"
  estado=$?
  echo "$salida" >> "$LOG"
  ejecutadas="$(echo "$salida" | awk '/^test result:/ { for (i = 1; i <= NF; i++) if ($i == "passed;") s += $(i - 1) } END { print s + 0 }')"
  if [ "$estado" -eq 0 ] && [ "$ejecutadas" -gt 0 ]; then
    registrar "$nombre ($ejecutadas pruebas)" 0
  else
    [ "$ejecutadas" -eq 0 ] && echo "NINGUNA prueba coincide con el filtro \"$filtro\"" >> "$LOG"
    registrar "$nombre" 1
  fi
}

paso_pytest_filtro() { # paso_pytest_filtro "nombre" expresión_-k   (exige >= 1 prueba pasada)
  local nombre="$1" expresion="$2" salida estado
  echo "=== $nombre  (servidor, -k \"$expresion\")" | tee -a "$LOG"
  salida="$(cd "$RAIZ/servidor" && uv run pytest -q -k "$expresion" 2>&1)"
  estado=$?
  echo "$salida" >> "$LOG"
  if [ "$estado" -eq 0 ] && echo "$salida" | grep -Eq '[0-9]+ passed'; then
    registrar "$nombre" 0
  else
    echo "Ninguna prueba pasó con -k \"$expresion\" (¿omitidas o sin coincidencias?)" >> "$LOG"
    registrar "$nombre" 1
  fi
}

# Todos los archivos de pruebas que necesitan PostgreSQL real (se omiten si falta
# CUENTAS_CLARAS_DB_URL_PRUEBAS). Si se añade otro archivo así, se añade aquí.
ARCHIVOS_POSTGRES=(
  tests/test_integracion_postgres.py
  tests/test_repositorio_usuarios.py
  tests/test_repositorio_amistades.py
  tests/test_repositorio_gastos.py
  tests/test_repositorio_grupos.py
  tests/test_cambios_postgres.py
  tests/test_rls_postgres.py
)

paso_postgres_real() { # las pruebas de PostgreSQL no se omiten
  local nombre="pytest PostgreSQL real, sin omitidas" salida estado
  echo "=== $nombre  (servidor)" | tee -a "$LOG"
  if [ "$EXIGIR_POSTGRES" != "1" ]; then
    echo "Omitido por EXIGIR_POSTGRES=$EXIGIR_POSTGRES" >> "$LOG"
    RESUMEN+=("OMITIDO $nombre (EXIGIR_POSTGRES=$EXIGIR_POSTGRES)")
    return
  fi
  salida="$(cd "$RAIZ/servidor" && uv run pytest -q -rs "${ARCHIVOS_POSTGRES[@]}" 2>&1)"
  estado=$?
  echo "$salida" >> "$LOG"
  if [ "$estado" -eq 0 ] && ! echo "$salida" | grep -Eqi 'skipped'; then
    registrar "$nombre" 0
  else
    echo "Hay pruebas omitidas o fallidas: levanta Docker Compose y define CUENTAS_CLARAS_DB_URL_PRUEBAS" >> "$LOG"
    registrar "$nombre" 1
  fi
}

# --- App (Rust y TypeScript) ---
paso "cargo fmt --all --check"                  app/src-tauri cargo fmt --all --check
paso "cargo clippy -D warnings"                 app/src-tauri cargo clippy --workspace --all-targets -- -D warnings
paso "cargo test --workspace"                   app/src-tauri cargo test --workspace
paso "cargo test --test arquitectura"           app/src-tauri cargo test --test arquitectura
paso_cargo_filtro "app: fusión del libro"                       "fusion"
paso_cargo_filtro "app: cifrado (frase incorrecta, archivo alterado)" "cifrado"
paso_cargo_filtro "app: contrato del adaptador falso"           "el_adaptador_falso_cumple_el_contrato"
paso_cargo_filtro "app: contrato del adaptador de carpeta"      "el_adaptador_de_carpeta_cumple_el_contrato"
paso_cargo_filtro "app: contrato del adaptador Git"             "el_adaptador_git_cumple_el_contrato"
paso_cargo_filtro "app: convergencia de beto (ESC-03)"          "convergen"
paso "app: dos instalaciones, carpeta y Git (ESC-03, ESC-04)" app/src-tauri cargo test -p infraestructura --test dos_instalaciones_extremo_a_extremo
paso_cargo_filtro "app: privacidad del destino"                 "privacidad"
paso_cargo_filtro "app: reconstrucción independiente de saldos" "reconstruccion"
paso "scripts/sync-test.sh"                     .             bash scripts/sync-test.sh
paso "hay pruebas de propiedades (proptest!)"   app/src-tauri grep -rq "proptest!" crates src tests
paso "cargo audit"                              app/src-tauri cargo audit
paso "npm audit"                                app           npm audit
paso "npx tsc --noEmit"                         app           npx tsc --noEmit
paso "npm run lint"                             app           npm run lint
paso "npm run build"                            app           npm run build

# --- Servidor (Python): no cambia en esta entrega, pero no debe romperse ---
paso "uv run ruff check (servidor)"             servidor      uv run ruff check .
paso "uv run ruff format --check (servidor)"    servidor      uv run ruff format --check .
paso "uv run mypy (servidor)"                   servidor      uv run mypy
paso "uv run pytest (servidor)"                 servidor      uv run pytest
paso "openapi.yaml coincide con /openapi.json"  servidor      uv run --with pyyaml python scripts/comparar_openapi.py
paso_postgres_real

# --- Documentación ---
paso "todas las ADR están en el índice"         docs/adr      bash -c 'faltan=0; for f in ADR-*.md; do grep -q "$f" ADR.md || { echo "Falta en ADR.md: $f"; faltan=1; }; done; exit $faltan'
paso "existe la ADR del cifrado (ADR-017)"      docs/adr      test -s ADR-017-cifrado-con-frase-de-paso.md
paso "existe la ADR del transporte (ADR-018)"   docs/adr      test -s ADR-018-puerto-transporte-sincronizacion.md
paso "existe docs/formato-exportacion.md"       docs          test -s formato-exportacion.md
paso "existe la guía de sincronización"         docs          test -s guia-sincronizacion.md
paso "existe docs/deuda-tecnica.md"             docs          test -s deuda-tecnica.md
paso "el servidor no recibió el libro: sin HTTP en respaldo ni destino" app/src-tauri bash -c '! grep -nE "reqwest|http" src/respaldo.rs src/sincronizacion_destino.rs'

echo
echo "================ RESUMEN ================"
printf '%s\n' "${RESUMEN[@]}"
echo "Fallas: $FALLAS   (detalle en $LOG)"
exit "$FALLAS"
