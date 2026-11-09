"""Fixtures y configuración compartida para las pruebas del servidor.

Provee aislamiento de base de datos SQLite por prueba, variables de entorno
falsas para JWT y clientes de prueba tipados.
"""

from collections.abc import Generator
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from main import app

SECRETO_PRUEBA = "secreto-de-prueba-muy-seguro-para-la-entrega-5"
MINUTOS_PRUEBA = 60


@pytest.fixture(autouse=True)
def entorno_prueba(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> Generator[None, None, None]:
    """Configura las variables de entorno de prueba para aislar cada ejecución."""
    ruta_bd = tmp_path / "usuarios_prueba.db"
    monkeypatch.setenv("JWT_SECRET", SECRETO_PRUEBA)
    monkeypatch.setenv("JWT_MINUTOS", str(MINUTOS_PRUEBA))
    monkeypatch.setenv("BD_USUARIOS_PATH", str(ruta_bd))
    monkeypatch.setenv("SQLITE_DB_PATH", str(ruta_bd))
    yield


@pytest.fixture
def bd_prueba(tmp_path: Path) -> Path:
    """Retorna la ruta al archivo de base de datos temporal de la prueba."""
    return tmp_path / "usuarios_prueba.db"


@pytest.fixture
def cliente() -> Generator[TestClient, None, None]:
    """Provee un cliente de pruebas HTTP para la aplicación FastAPI."""
    with TestClient(app) as c:
        yield c
