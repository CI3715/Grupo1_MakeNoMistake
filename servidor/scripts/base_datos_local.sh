#!/usr/bin/env bash
# Base de datos PostgreSQL local para el servidor, pensada para desarrollo.
#
# Crea un clúster propio dentro de servidor/.postgres (ignorado por Git) en el
# puerto 55432, para no chocar con un PostgreSQL del sistema que ya use el 5432.
# No necesita Docker ni permisos de superusuario.
#
# Uso:
#   bash scripts/base_datos_local.sh iniciar
#   bash scripts/base_datos_local.sh estado
#   bash scripts/base_datos_local.sh psql
#   bash scripts/base_datos_local.sh detener
#
# El usuario y la clave no están en el código: se leen de
# CUENTAS_CLARAS_DB_USUARIO y CUENTAS_CLARAS_DB_CLAVE, definidas en
# servidor/.env (ignorado por Git) o exportadas en el entorno.
#
# Requiere los binarios de PostgreSQL 17 instalados (initdb, pg_ctl y psql).

set -euo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATOS="${CUENTAS_CLARAS_PGDATA:-$RAIZ/.postgres}"
PUERTO="${CUENTAS_CLARAS_PGPORT:-55432}"
BASE="cuentas_claras"
BASE_PRUEBAS="cuentas_claras_pruebas"

valor_env() {
  local nombre="$1"
  if [ -f "$RAIZ/.env" ]; then
    sed -n "s/^${nombre}=//p" "$RAIZ/.env" | tail -n 1
  fi
}

USUARIO="${CUENTAS_CLARAS_DB_USUARIO:-$(valor_env CUENTAS_CLARAS_DB_USUARIO)}"
CLAVE="${CUENTAS_CLARAS_DB_CLAVE:-$(valor_env CUENTAS_CLARAS_DB_CLAVE)}"

exigir_credenciales() {
  if [ -z "$USUARIO" ] || [ -z "$CLAVE" ]; then
    echo "Faltan CUENTAS_CLARAS_DB_USUARIO y/o CUENTAS_CLARAS_DB_CLAVE." >&2
    echo "Defínelas en servidor/.env o expórtalas antes de ejecutar el script." >&2
    exit 1
  fi

  if ! [[ "$USUARIO" =~ ^[a-zA-Z0-9_]+$ ]]; then
    echo "CUENTAS_CLARAS_DB_USUARIO solo puede tener letras, números y guion bajo." >&2
    exit 1
  fi
}

if ! command -v pg_config >/dev/null 2>&1; then
  echo "No se encontró pg_config. Instala PostgreSQL 17 antes de continuar." >&2
  exit 1
fi

BIN="$(pg_config --bindir)"
INITDB="$BIN/initdb"
PG_CTL="$BIN/pg_ctl"
PG_ISREADY="$BIN/pg_isready"
PSQL="$(command -v psql)"

psql_postgres() {
  "$PSQL" -h 127.0.0.1 -p "$PUERTO" -U postgres -d postgres -v ON_ERROR_STOP=1 "$@"
}

existe_rol() {
  psql_postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname = '$USUARIO'" | grep -qx "1"
}

existe_base() {
  psql_postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '$1'" | grep -qx "1"
}

iniciar() {
  exigir_credenciales

  if [ ! -f "$DATOS/PG_VERSION" ]; then
    echo "Creando el clúster local en $DATOS…"
    mkdir -p "$DATOS"
    "$INITDB" -D "$DATOS" -U postgres --auth=trust --encoding=UTF8 >/dev/null
  fi

  if ! "$PG_CTL" -D "$DATOS" status >/dev/null 2>&1; then
    echo "Iniciando PostgreSQL en el puerto $PUERTO…"
    "$PG_CTL" -D "$DATOS" -o "-p $PUERTO -k $DATOS" -l "$DATOS/postgres.log" start >/dev/null
  fi

  until "$PG_ISREADY" -h 127.0.0.1 -p "$PUERTO" >/dev/null 2>&1; do
    sleep 0.2
  done

  local clave_sql="${CLAVE//\'/\'\'}"
  if existe_rol; then
    psql_postgres -c \
      "ALTER ROLE $USUARIO WITH LOGIN PASSWORD '$clave_sql'" >/dev/null
  else
    psql_postgres -c "CREATE ROLE $USUARIO LOGIN PASSWORD '$clave_sql'"
  fi

  for base in "$BASE" "$BASE_PRUEBAS"; do
    if ! existe_base "$base"; then
      psql_postgres -c "CREATE DATABASE $base OWNER $USUARIO"
    fi
  done

  echo "PostgreSQL listo en 127.0.0.1:$PUERTO"
  echo "  usuario:            $USUARIO"
  echo "  base de desarrollo: $BASE"
  echo "  base de pruebas:    $BASE_PRUEBAS"
}

detener() {
  if [ -f "$DATOS/PG_VERSION" ] && "$PG_CTL" -D "$DATOS" status >/dev/null 2>&1; then
    "$PG_CTL" -D "$DATOS" stop -m fast >/dev/null
    echo "PostgreSQL detenido."
  else
    echo "PostgreSQL no está en ejecución."
  fi
}

estado() {
  if [ -f "$DATOS/PG_VERSION" ] && "$PG_CTL" -D "$DATOS" status >/dev/null 2>&1; then
    echo "PostgreSQL en ejecución en 127.0.0.1:$PUERTO (datos en $DATOS)."
  else
    echo "PostgreSQL detenido."
  fi
}

case "${1:-}" in
  iniciar) iniciar ;;
  detener) detener ;;
  estado) estado ;;
  psql)
    exigir_credenciales
    shift
    "$PSQL" -h 127.0.0.1 -p "$PUERTO" -U "$USUARIO" -d "$BASE" "$@"
    ;;
  *)
    echo "Uso: $0 {iniciar|detener|estado|psql}" >&2
    exit 1
    ;;
esac
