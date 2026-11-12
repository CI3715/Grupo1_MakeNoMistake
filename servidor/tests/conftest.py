"""Fixtures y configuración compartida para las pruebas del servidor.

Cada prueba recibe un almacén de usuarios en memoria nuevo, variables de
entorno falsas para el JWT y un cliente de pruebas (incluyendo los usuarios
ana, beto, carla y dario para la Entrega 6). Las pruebas contra PostgreSQL
real están en ``test_repositorio_usuarios.py`` y ``test_integracion_postgres.py``.
"""

import os
from collections.abc import Generator
from typing import Any

import pytest
from dotenv import load_dotenv
from fastapi.testclient import TestClient
from repositorio_en_memoria import (
    RepositorioAmistadesEnMemoria,
    RepositorioEnMemoria,
)

from base_datos import BaseDatos
from configuracion import RUTA_ENV
from main import app

# Igual que en las pruebas de usuarios: en local ``servidor/.env`` ya trae la
# base de pruebas y en un clon limpio las pruebas de PostgreSQL se omiten.
load_dotenv(RUTA_ENV)
URL_PRUEBAS = os.environ.get("CUENTAS_CLARAS_DB_URL_PRUEBAS") or None

SECRETO_PRUEBA = "secreto-de-prueba-muy-seguro-para-la-entrega-5"
MINUTOS_PRUEBA = 60
CONTRASENA_PRUEBA = "Password123!"

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
    monkeypatch.setattr(
        app.state, "repositorio_grupos", repositorio_falso, raising=False
    )
    monkeypatch.setattr(
        app.state,
        "repositorio_amistades",
        RepositorioAmistadesEnMemoria(repositorio_falso),
        raising=False,
    )
    yield


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
    data = resp.json()
    token: str = data.get("token") or data.get("token_acceso", "")
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


@pytest.fixture
def base_datos_pruebas() -> BaseDatos:
    """Base de pruebas vacía, con el esquema creado.

    Se omite la prueba si no está definida ``CUENTAS_CLARAS_DB_URL_PRUEBAS``,
    para que ``uv run pytest`` funcione en un clon limpio sin PostgreSQL.
    """

    if URL_PRUEBAS is None:
        pytest.skip(
            "Define CUENTAS_CLARAS_DB_URL_PRUEBAS para probar los repositorios "
            "contra PostgreSQL."
        )

    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()
    with base_datos.conexion() as conexion:
        # CASCADE limpia también amistades, grupos, miembros, gastos y partes.
        conexion.execute("TRUNCATE TABLE usuarios CASCADE")
    return base_datos
