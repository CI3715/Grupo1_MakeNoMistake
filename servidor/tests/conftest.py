"""Fixtures y configuración compartida para las pruebas del servidor.

Carga la configuración local, fija variables de entorno falsas para el token y
usa la base PostgreSQL de pruebas con la tabla ``usuarios`` vacía en cada
prueba que toca el servidor.
"""

import os
from collections.abc import Generator

import pytest
from dotenv import load_dotenv
from fastapi.testclient import TestClient

from base_datos import BaseDatos
from configuracion import RUTA_ENV
from main import app

SECRETO_PRUEBA = "secreto-de-prueba-muy-seguro-para-la-entrega-5"
MINUTOS_PRUEBA = 60

# En local, `servidor/.env` ya trae la base de pruebas. En un clon limpio no
# existe y las pruebas que necesitan PostgreSQL se omiten.
load_dotenv(RUTA_ENV)
URL_PRUEBAS = os.environ.get("CUENTAS_CLARAS_DB_URL_PRUEBAS") or None


def _base_datos_pruebas() -> BaseDatos:
    if URL_PRUEBAS is None:
        pytest.skip(
            "Define CUENTAS_CLARAS_DB_URL_PRUEBAS para probar el servidor "
            "contra PostgreSQL."
        )

    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()
    return base_datos


@pytest.fixture(autouse=True)
def entorno_prueba(
    monkeypatch: pytest.MonkeyPatch,
) -> Generator[None, None, None]:
    """Configura las variables de entorno de prueba para aislar cada ejecución."""
    monkeypatch.setenv("JWT_SECRET", SECRETO_PRUEBA)
    monkeypatch.setenv("JWT_MINUTOS", str(MINUTOS_PRUEBA))
    monkeypatch.setenv("CUENTAS_CLARAS_SECRETO_JWT", SECRETO_PRUEBA)
    monkeypatch.setenv("CUENTAS_CLARAS_MINUTOS_TOKEN", str(MINUTOS_PRUEBA))
    if URL_PRUEBAS is None:
        # Sin base de pruebas no se debe tocar la base de desarrollo.
        monkeypatch.delenv("CUENTAS_CLARAS_DB_URL", raising=False)
    else:
        monkeypatch.setenv("CUENTAS_CLARAS_DB_URL", URL_PRUEBAS)
    yield


@pytest.fixture
def base_datos_prueba() -> BaseDatos:
    """Base PostgreSQL de pruebas con el esquema creado."""
    return _base_datos_pruebas()


@pytest.fixture
def cliente() -> Generator[TestClient, None, None]:
    """Cliente HTTP con la base de pruebas vacía antes de cada prueba.

    `CASCADE` arrastra las tablas compartidas que referencian a `usuarios`;
    sin él, PostgreSQL rechaza el `TRUNCATE` por las claves foráneas.
    """
    base_datos = _base_datos_pruebas()
    with base_datos.conexion() as conexion:
        conexion.execute("TRUNCATE TABLE usuarios CASCADE")

    with TestClient(app) as c:
        yield c
