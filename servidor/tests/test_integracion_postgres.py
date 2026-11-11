"""Rutas de registro, sesión y perfil contra PostgreSQL real.

La app arranca con su ciclo de vida normal (``CUENTAS_CLARAS_DB_URL`` apunta a
la base de pruebas), así que se prueba el mismo camino que en producción. Se
omiten si no está definida ``CUENTAS_CLARAS_DB_URL_PRUEBAS``.
"""

import os
from collections.abc import Callable, Generator
from datetime import datetime, timedelta, timezone

import pytest
from dotenv import load_dotenv
from fastapi.testclient import TestClient

import main
import token_sesion
from base_datos import BaseDatos
from configuracion import RUTA_ENV
from main import app
from repositorio_usuarios import RepositorioUsuarios

load_dotenv(RUTA_ENV)
URL_PRUEBAS = os.environ.get("CUENTAS_CLARAS_DB_URL_PRUEBAS") or None

pytestmark = pytest.mark.skipif(
    URL_PRUEBAS is None,
    reason=(
        "Define CUENTAS_CLARAS_DB_URL_PRUEBAS para probar las rutas contra PostgreSQL."
    ),
)

SECRETO = "secreto-de-integracion-con-mas-de-32-caracteres"


def registro(nombre_usuario: str, correo: str) -> dict[str, str]:
    return {
        "nombre_usuario": nombre_usuario,
        "nombre_visible": "Ana Pérez",
        "correo": correo,
        "contrasena": "clave-segura-1234",
    }


@pytest.fixture()
def cliente_pg(monkeypatch: pytest.MonkeyPatch) -> Generator[TestClient, None, None]:
    assert URL_PRUEBAS is not None
    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()
    with base_datos.conexion() as conexion:
        conexion.execute("TRUNCATE TABLE usuarios")

    monkeypatch.setenv("CUENTAS_CLARAS_DB_URL", URL_PRUEBAS)
    monkeypatch.setenv("CUENTAS_CLARAS_SECRETO_JWT", SECRETO)
    # Sin almacén inyectado, el arranque conecta con PostgreSQL como en
    # producción. monkeypatch devuelve el de memoria al terminar.
    monkeypatch.delattr(app.state, "repositorio_usuarios")
    with TestClient(app) as cliente:
        yield cliente


def test_arranque_usa_el_repositorio_de_postgres(cliente_pg: TestClient) -> None:
    assert isinstance(app.state.repositorio_usuarios, RepositorioUsuarios)


def test_registro_sesion_y_perfil(cliente_pg: TestClient) -> None:
    creado = cliente_pg.post("/usuarios", json=registro("ana.perez", "Ana@Example.com"))
    assert creado.status_code == 201
    usuario = creado.json()
    assert usuario["correo"] == "ana@example.com"
    assert "contrasena" not in usuario and "hash_contrasena" not in usuario

    # La sesión de PostgreSQL puede estar en otra zona horaria; la respuesta
    # siempre va en UTC con el formato AAAA-MM-DDThh:mm:ssZ.
    fecha = datetime.strptime(usuario["fecha_alta"], "%Y-%m-%dT%H:%M:%SZ")
    fecha = fecha.replace(tzinfo=timezone.utc)
    assert abs(datetime.now(timezone.utc) - fecha) < timedelta(minutes=2)

    sesion = cliente_pg.post(
        "/sesiones",
        json={"correo": "ANA@example.com", "contrasena": "clave-segura-1234"},
    )
    assert sesion.status_code == 200
    token = sesion.json()["token"]

    perfil = cliente_pg.get(
        "/usuarios/yo", headers={"Authorization": f"Bearer {token}"}
    )
    assert perfil.status_code == 200
    assert perfil.json() == usuario


def test_contrasena_incorrecta_retorna_401(cliente_pg: TestClient) -> None:
    cliente_pg.post("/usuarios", json=registro("ana.perez", "ana@example.com"))
    respuesta = cliente_pg.post(
        "/sesiones",
        json={"correo": "ana@example.com", "contrasena": "otra-clave-1234"},
    )
    assert respuesta.status_code == 401


def test_duplicados_informan_ambos_campos(cliente_pg: TestClient) -> None:
    cliente_pg.post("/usuarios", json=registro("ana.perez", "ana@example.com"))

    respuesta = cliente_pg.post(
        "/usuarios", json=registro("ana.perez", "ANA@EXAMPLE.COM")
    )

    assert respuesta.status_code == 409
    campos = [e["campo"] for e in respuesta.json()["errores"]]
    assert campos == ["nombre_usuario", "correo"]


@pytest.mark.parametrize(
    ("nombre_usuario", "correo", "campo"),
    [
        ("otra.persona", "ana@example.com", "correo"),
        ("ana.perez", "otra@example.com", "nombre_usuario"),
    ],
)
def test_duplicado_detectado_por_postgres_retorna_409(
    cliente_pg: TestClient,
    monkeypatch: pytest.MonkeyPatch,
    nombre_usuario: str,
    correo: str,
    campo: str,
) -> None:
    """Dos registros simultáneos: la consulta previa no ve el duplicado y lo
    detecta la restricción UNIQUE al insertar."""
    cliente_pg.post("/usuarios", json=registro("ana.perez", "ana@example.com"))

    consultar: Callable[..., list[str]] = main.campos_ocupados
    llamadas = []

    def primera_consulta_vacia(*args: object) -> list[str]:
        llamadas.append(args)
        return [] if len(llamadas) == 1 else consultar(*args)

    monkeypatch.setattr(main, "campos_ocupados", primera_consulta_vacia)

    respuesta = cliente_pg.post("/usuarios", json=registro(nombre_usuario, correo))

    assert respuesta.status_code == 409
    assert [e["campo"] for e in respuesta.json()["errores"]] == [campo]
    assert len(llamadas) == 2


def test_token_con_sub_que_no_es_uuid_retorna_401(cliente_pg: TestClient) -> None:
    token = token_sesion.crear_token("no-es-un-uuid").token
    respuesta = cliente_pg.get(
        "/usuarios/yo", headers={"Authorization": f"Bearer {token}"}
    )
    assert respuesta.status_code == 401


def test_base_del_servidor_solo_tiene_la_tabla_usuarios(
    cliente_pg: TestClient,
) -> None:
    """INV-16: ninguna tabla con cuentas, saldos, movimientos ni categorías."""
    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        filas = conexion.execute(
            "SELECT table_name FROM information_schema.tables "
            "WHERE table_schema = 'public' ORDER BY table_name"
        ).fetchall()
    assert [fila["table_name"] for fila in filas] == ["usuarios"]
