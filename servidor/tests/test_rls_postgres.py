"""RLS real sobre PostgreSQL.

Recorre todas las tablas de ``public`` y falla si alguna no tiene activado
``ROW LEVEL SECURITY``. Así cubre también las tablas nuevas que añadan las
siguientes entregas. Comprueba además que los roles de Supabase no conservan
permisos sobre esas tablas.

No borra datos: aplica el esquema (idempotente) y lee los catálogos. Para
probar Supabase con esta prueba, sin exponer el resto de la suite a los
``TRUNCATE``:

    CUENTAS_CLARAS_DB_URL_PRUEBAS="$CUENTAS_CLARAS_DB_URL" \
        uv run pytest tests/test_rls_postgres.py
"""

import os
from typing import Any

import pytest
from dotenv import load_dotenv

from base_datos import BaseDatos
from configuracion import RUTA_ENV

load_dotenv(RUTA_ENV)
URL_PRUEBAS = os.environ.get("CUENTAS_CLARAS_DB_URL_PRUEBAS") or None

pytestmark = pytest.mark.skipif(
    URL_PRUEBAS is None,
    reason=(
        "Define CUENTAS_CLARAS_DB_URL_PRUEBAS para probar el RLS contra PostgreSQL."
    ),
)

CONSULTA_TABLAS = """
SELECT c.relname, c.relrowsecurity
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'public'
   AND c.relkind IN ('r', 'p')
 ORDER BY c.relname
"""

CONSULTA_PERMISOS = """
SELECT c.relname, r.rolname, privilegio.privilege_type
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  CROSS JOIN LATERAL aclexplode(c.relacl) AS privilegio
  JOIN pg_roles r ON r.oid = privilegio.grantee
 WHERE n.nspname = 'public'
   AND c.relkind IN ('r', 'p')
   AND r.rolname IN ('anon', 'authenticated')
 ORDER BY c.relname, r.rolname, privilegio.privilege_type
"""


@pytest.fixture()
def tablas_publico() -> list[dict[str, Any]]:
    """Tablas de ``public`` después de aplicar el esquema del servidor."""

    assert URL_PRUEBAS is not None
    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()

    with base_datos.conexion() as conexion:
        filas = conexion.execute(CONSULTA_TABLAS).fetchall()

    assert filas, "No hay tablas en public: no se pudo comprobar el RLS"
    return [dict(fila) for fila in filas]


def test_todas_las_tablas_de_public_tienen_rls(
    tablas_publico: list[dict[str, Any]],
) -> None:
    """Falla si cualquier tabla de ``public`` quedó sin RLS."""

    sin_rls = [
        str(tabla["relname"]) for tabla in tablas_publico if not tabla["relrowsecurity"]
    ]
    assert not sin_rls, f"Tablas de public sin RLS: {', '.join(sin_rls)}"


def test_roles_de_supabase_sin_permisos(
    tablas_publico: list[dict[str, Any]],
) -> None:
    """Si los roles existen, no deben tener privilegios sobre las tablas."""

    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        filas = conexion.execute(CONSULTA_PERMISOS).fetchall()

    permisos = [
        f"{fila['relname']}: {fila['rolname']} ({fila['privilege_type']})"
        for fila in filas
    ]
    assert not permisos, "Permisos concedidos a roles de Supabase: " + ", ".join(
        permisos
    )
