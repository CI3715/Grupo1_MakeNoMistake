"""Fixtures y configuración compartida para las pruebas del servidor.

Provee aislamiento de base de datos SQLite por prueba, variables de entorno
falsas para JWT, clientes de prueba tipados y ayudantes para usuarios y sesiones
(incluyendo ana, beto, carla y dario para la Entrega 6).
"""

from collections.abc import Generator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient

from main import app

SECRETO_PRUEBA = "secreto-de-prueba-muy-seguro-para-la-entrega-5"
MINUTOS_PRUEBA = 60
CONTRASENA_PRUEBA = "Password123!"


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


def registrar_usuario_prueba(
    c: TestClient,
    usuario: str,
    nombre_visible: str,
    correo: str,
    contrasena: str = CONTRASENA_PRUEBA,
) -> dict[str, Any]:
    """Registra un usuario en la API y retorna la respuesta 201."""
    resp = c.post(
        "/usuarios",
        json={
            "nombre_usuario": usuario,
            "nombre_visible": nombre_visible,
            "correo": correo,
            "contrasena": contrasena,
        },
    )
    assert resp.status_code == 201, f"Fallo al registrar {usuario}: {resp.text}"
    return resp.json()  # type: ignore[no-any-return]


def iniciar_sesion_prueba(
    c: TestClient, correo: str, contrasena: str = CONTRASENA_PRUEBA
) -> str:
    """Inicia sesión y retorna el token de acceso."""
    resp = c.post(
        "/sesiones",
        json={"correo": correo, "contrasena": contrasena},
    )
    assert resp.status_code == 200, f"Fallo al iniciar sesión {correo}: {resp.text}"
    token: str = resp.json()["token_acceso"]
    return token


def encabezado_auth(token: str) -> dict[str, str]:
    """Retorna el encabezado Authorization para solicitudes Bearer."""
    return {"Authorization": f"Bearer {token}"}


@pytest.fixture
def usuarios_flujo(cliente: TestClient) -> dict[str, dict[str, Any]]:
    """Crea los 4 usuarios del flujo de la Entrega 6 (ana, beto, carla y dario).

    - ana: miembro y creadora del grupo
    - beto: amigo y miembro
    - carla: amiga y miembro
    - dario: usuario registrado ajeno (no amigo ni miembro del grupo)
    """
    datos_usuarios = [
        ("ana", "Ana García", "ana@ejemplo.com"),
        ("beto", "Beto Pérez", "beto@ejemplo.com"),
        ("carla", "Carla Gómez", "carla@ejemplo.com"),
        ("dario", "Darío Silva", "dario@ejemplo.com"),
    ]
    resultado: dict[str, dict[str, Any]] = {}
    for usuario, visible, correo in datos_usuarios:
        perfil = registrar_usuario_prueba(cliente, usuario, visible, correo)
        token = iniciar_sesion_prueba(cliente, correo)
        resultado[usuario] = {
            "perfil": perfil,
            "usuario": usuario,
            "nombre_visible": visible,
            "correo": correo,
            "token": token,
            "auth": encabezado_auth(token),
        }
    return resultado


@pytest.fixture
def token_ana(usuarios_flujo: dict[str, dict[str, Any]]) -> str:
    """Token de acceso para ana."""
    return str(usuarios_flujo["ana"]["token"])


@pytest.fixture
def token_beto(usuarios_flujo: dict[str, dict[str, Any]]) -> str:
    """Token de acceso para beto."""
    return str(usuarios_flujo["beto"]["token"])


@pytest.fixture
def token_carla(usuarios_flujo: dict[str, dict[str, Any]]) -> str:
    """Token de acceso para carla."""
    return str(usuarios_flujo["carla"]["token"])


@pytest.fixture
def token_dario(usuarios_flujo: dict[str, dict[str, Any]]) -> str:
    """Token de acceso para dario (usuario ajeno no miembro)."""
    return str(usuarios_flujo["dario"]["token"])
