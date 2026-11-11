"""Fixtures y configuración compartida para las pruebas del servidor.

Cada prueba recibe un almacén de usuarios en memoria nuevo, variables de
entorno falsas para el JWT y un cliente de pruebas. Las pruebas contra
PostgreSQL real están en ``test_repositorio_usuarios.py`` y
``test_integracion_postgres.py``.
"""

from collections.abc import Generator

import pytest
from fastapi.testclient import TestClient
from repositorio_en_memoria import RepositorioEnMemoria

from main import app

SECRETO_PRUEBA = "secreto-de-prueba-muy-seguro-para-la-entrega-5"
MINUTOS_PRUEBA = 60

# Variables que pueden llegar desde servidor/.env (algunas pruebas lo cargan) y
# que cambiarían el resultado de las pruebas de configuración del token.
VARIABLES_AJENAS = (
    "CUENTAS_CLARAS_DB_URL",
    "CUENTAS_CLARAS_SECRETO_JWT",
    "CUENTAS_CLARAS_MINUTOS_TOKEN",
)

# Solo lo compartido entre varias personas (Entrega 6). Cualquier tabla nueva
# tiene que justificarse aquí: añadir una del libro personal rompe INV-16.
TABLAS_PERMITIDAS = {
    "usuarios",
    "amistades",
    "grupos",
    "miembros",
    "gastos",
    "partes_gasto",
}


@pytest.fixture
def repositorio_falso() -> RepositorioEnMemoria:
    """Almacén en memoria que usan las rutas durante la prueba."""
    return RepositorioEnMemoria()


@pytest.fixture(autouse=True)
def entorno_prueba(
    monkeypatch: pytest.MonkeyPatch, repositorio_falso: RepositorioEnMemoria
) -> Generator[None, None, None]:
    """Aísla cada prueba: entorno propio y ninguna conexión a PostgreSQL."""
    for variable in VARIABLES_AJENAS:
        monkeypatch.delenv(variable, raising=False)
    monkeypatch.setenv("JWT_SECRET", SECRETO_PRUEBA)
    monkeypatch.setenv("JWT_MINUTOS", str(MINUTOS_PRUEBA))
    # Con un almacén ya presente, el arranque de la app no abre conexiones.
    monkeypatch.setattr(
        app.state, "repositorio_usuarios", repositorio_falso, raising=False
    )
    yield


@pytest.fixture
def cliente() -> Generator[TestClient, None, None]:
    """Provee un cliente de pruebas HTTP para la aplicación FastAPI."""
    with TestClient(app) as c:
        yield c
