#!/usr/bin/env bash
# Lista de verificación de calidad de la Entrega 7.
# Ejecutar desde la RAÍZ del repositorio:  bash verificar-entrega7.sh
# Corre todas las comprobaciones aunque alguna falle, guarda la salida completa
# en verificacion-entrega7.log (no se sube al repositorio) y muestra un resumen.
#
# Diferencias con la Entrega 6:
#  - Vectores de sincronización (app y servidor) y comparación de openapi.yaml
#    con el /openapi.json de FastAPI (GET /cambios e Idempotency-Key).
#  - Pruebas de idempotencia, de la interrupción a mitad de lote, de la cola con
#    reintentos y de la clave de idempotencia en el servidor. Estos pasos exigen
#    que se ejecute AL MENOS UNA prueba: un filtro que no coincide con ninguna
#    no cuenta como éxito.
#  - Pruebas de propiedades de la app (proptest) además de las del servidor.
#  - Pruebas contra PostgreSQL real sin omitir (Docker Compose, puerto 55432) y
#    comprobación de RLS.
#  - Todas las ADR deben estar en docs/adr/ADR.md.
#
# Los filtros de abajo buscan por NOMBRE de prueba. Si una prueba se llama
# distinto, se cambia el filtro aquí, no se quita el paso.
#
# Variables opcionales:
#   EXIGIR_POSTGRES=0   no exige las pruebas con PostgreSQL real (por defecto 1).
#                       Antes de exigirlas:  docker compose up -d  (en servidor/)
#                       y definir CUENTAS_CLARAS_DB_URL_PRUEBAS.
#
# Nota: `npm audit`, `cargo audit` y `uv audit` devuelven error si hay
# vulnerabilidades conocidas; eso no es un fallo del script: es lo que va a
# docs/deuda-tecnica.md.
# No uses `npm audit fix --force`.

RAIZ="$(pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
LOG="$RAIZ/verificacion-entrega7.log"
: > "$LOG"
RESUMEN=()
FALLAS=0
EXIGIR_POSTGRES="${EXIGIR_POSTGRES:-1}"

# Filtros por nombre de prueba (ver nota de arriba).
FILTRO_IDEMPOTENCIA_APP="idempot"        # cargo: aplicar dos veces el mismo lote
FILTRO_INTERRUPCION_APP="interrup"       # cargo: cierre a mitad de lote
FILTRO_COLA_APP="cola"                   # cargo: cola con reintentos
FILTRO_PROPIEDADES_APP="propiedad"       # cargo: pruebas de proptest
FILTRO_IDEMPOTENCIA_SERVIDOR="idempotencia"   # pytest -k
FILTRO_RLS_SERVIDOR="rls"                     # pytest -k

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

paso_postgres_real() { # las pruebas de PostgreSQL no se omiten
  local nombre="pytest PostgreSQL real, sin omitidas" salida estado
  echo "=== $nombre  (servidor)" | tee -a "$LOG"
  if [ "$EXIGIR_POSTGRES" != "1" ]; then
    echo "Omitido por EXIGIR_POSTGRES=$EXIGIR_POSTGRES" >> "$LOG"
    RESUMEN+=("OMITIDO $nombre (EXIGIR_POSTGRES=$EXIGIR_POSTGRES)")
    return
  fi
  salida="$(cd "$RAIZ/servidor" && uv run pytest -q -rs tests/test_integracion_postgres.py tests/test_repositorio_usuarios.py 2>&1)"
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
paso "cargo test --test vectores_division"      app/src-tauri cargo test --test vectores_division
paso "cargo test --test vectores_sincronizacion" app/src-tauri cargo test --test vectores_sincronizacion
paso_cargo_filtro "app: idempotencia (mismo lote dos veces)" "$FILTRO_IDEMPOTENCIA_APP"
paso_cargo_filtro "app: interrupción a mitad de lote"        "$FILTRO_INTERRUPCION_APP"
paso_cargo_filtro "app: cola con reintentos"                 "$FILTRO_COLA_APP"
paso_cargo_filtro "app: pruebas de propiedades (proptest)"   "$FILTRO_PROPIEDADES_APP"
paso "hay pruebas de propiedades (proptest!)"   app/src-tauri grep -rq "proptest!" crates src tests
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
paso "pytest vectores de sincronización"        servidor      uv run pytest -q tests/test_vectores_sincronizacion.py
paso "pytest división de gastos"                servidor      uv run pytest -q tests/test_division_gastos.py
paso "pytest datos privados no se filtran"      servidor      uv run pytest -q tests/test_datos_privados_no_se_filtran.py
paso_pytest_filtro "servidor: clave de idempotencia" "$FILTRO_IDEMPOTENCIA_SERVIDOR"
paso_pytest_filtro "servidor: RLS"                   "$FILTRO_RLS_SERVIDOR"
paso_postgres_real
paso "hay pruebas de propiedades (hypothesis)"  servidor      grep -rq "@given" tests
paso "openapi.yaml coincide con /openapi.json"  servidor      uv run --with pyyaml python scripts/comparar_openapi.py
paso "uv audit (servidor)"                      servidor      uv audit

# --- Documentación ---
paso "todas las ADR están en el índice"         docs/adr      bash -c 'faltan=0; for f in ADR-*.md; do grep -q "$f" ADR.md || { echo "Falta en ADR.md: $f"; faltan=1; }; done; exit $faltan'
paso "existe docs/deuda-tecnica.md"             docs          test -s deuda-tecnica.md

echo
echo "================ RESUMEN ================"
printf '%s\n' "${RESUMEN[@]}"
echo "Fallas: $FALLAS   (detalle en $LOG)"
exit "$FALLAS"