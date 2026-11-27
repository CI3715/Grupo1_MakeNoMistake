"""Rutas de registro, sesión, perfil, grupos y gastos contra PostgreSQL real.

La app arranca con su ciclo de vida normal (``CUENTAS_CLARAS_DB_URL`` apunta a
la base de pruebas), así que se prueba el mismo camino que en producción. Se
omiten si no está definida ``CUENTAS_CLARAS_DB_URL_PRUEBAS``.
"""

import os
from collections.abc import Callable, Generator
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from uuid import uuid4

import pytest
from conftest import TABLAS_PERMITIDAS
from dotenv import load_dotenv
from fastapi.testclient import TestClient
from flujo_entrega7 import recorrer_flujo_entrega_7
from flujo_http import BALANCES_2_5, gasto, grupo_de_tres, registrar_ejemplo_2_5

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
        conexion.execute("TRUNCATE TABLE usuarios CASCADE")

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


def test_base_del_servidor_solo_tiene_tablas_permitidas(
    cliente_pg: TestClient,
) -> None:
    """INV-16: ninguna tabla con cuentas, saldos, movimientos ni categorías."""
    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        filas = conexion.execute(
            "SELECT table_name FROM information_schema.tables "
            "WHERE table_schema = 'public'"
        ).fetchall()
    assert {fila["table_name"] for fila in filas} == TABLAS_PERMITIDAS


def test_flujo_de_grupo_y_gastos_como_usuario_nuevo(cliente_pg: TestClient) -> None:
    """RF-COM-04/05/08 de punta a punta contra PostgreSQL: personas nuevas, grupo,
    los dos gastos del enunciado 2.5 y los mismos balances para todos."""
    grupo_id, auth = grupo_de_tres(cliente_pg)
    registrar_ejemplo_2_5(cliente_pg, grupo_id, auth)

    for quien in ("ana", "beto", "carla"):
        respuesta = cliente_pg.get(f"/grupos/{grupo_id}", headers=auth[quien])
        assert respuesta.status_code == 200, respuesta.text
        assert respuesta.json()["balances"] == BALANCES_2_5

    datos = cliente_pg.get(f"/grupos/{grupo_id}", headers=auth["ana"]).json()
    assert [g["concepto"] for g in datos["gastos"]] == ["Taxi", "Cena"]
    assert datos["gastos"][1]["partes"] == [
        {"usuario": "ana", "monto": "33.34", "incluye_redondeo": True},
        {"usuario": "beto", "monto": "33.33", "incluye_redondeo": False},
        {"usuario": "carla", "monto": "33.33", "incluye_redondeo": False},
    ]

    # En la base: NUMERIC exacto, regla canónica y partes que suman el monto.
    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        filas = conexion.execute(
            "SELECT g.regla, g.monto, sum(p.monto) AS suma, "
            "array_agg(p.porcentaje ORDER BY p.porcentaje) AS porcentajes "
            "FROM gastos g JOIN partes_gasto p ON p.gasto_id = g.id "
            "GROUP BY g.id ORDER BY g.fecha"
        ).fetchall()
    assert [(f["regla"], f["monto"], f["suma"]) for f in filas] == [
        ("iguales", Decimal("100.00"), Decimal("100.00")),
        ("porcentajes", Decimal("60.00"), Decimal("60.00")),
    ]
    assert filas[1]["porcentajes"] == [2500, 2500, 5000]


def test_gasto_invalido_o_ajeno_no_guarda_nada(cliente_pg: TestClient) -> None:
    grupo_id, auth = grupo_de_tres(cliente_pg)

    no_cuadra = cliente_pg.post(
        f"/grupos/{grupo_id}/gastos",
        json=gasto("100.00", "ana", "porcentajes", [("ana", "50"), ("beto", "45")]),
        headers=auth["ana"],
    )
    assert no_cuadra.status_code == 400
    assert no_cuadra.json()["errores"] == [
        {"campo": "porcentajes", "mensaje": "Los porcentajes suman 95; faltan 5."}
    ]

    ajeno = cliente_pg.post(
        f"/grupos/{grupo_id}/gastos",
        json=gasto("10.00", "dario", "partes_iguales", [("dario", "")]),
        headers=auth["dario"],
    )
    inventado = cliente_pg.get(f"/grupos/{uuid4()}", headers=auth["ana"])
    assert ajeno.status_code == inventado.status_code == 404
    assert ajeno.content == inventado.content

    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        fila = conexion.execute("SELECT count(*) AS n FROM gastos").fetchone()
    assert fila is not None and fila["n"] == 0


def test_flujo_de_cambios_e_idempotencia_como_usuario_nuevo(
    cliente_pg: TestClient,
) -> None:
    """Entrega 7 de punta a punta contra PostgreSQL: cambios, cursor y reintentos."""
    recibidos = recorrer_flujo_entrega_7(cliente_pg)
    assert recibidos["dario"] == []

    # En la base: un evento por parte, una clave por gasto con clave y el
    # registro sin tablas del libro personal (INV-16).
    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        conteos = conexion.execute(
            "SELECT (SELECT count(*) FROM gastos) AS gastos, "
            "(SELECT count(*) FROM cambios) AS cambios, "
            "(SELECT count(*) FROM claves_idempotencia) AS claves"
        ).fetchone()
    assert conteos is not None
    assert (conteos["gastos"], conteos["cambios"], conteos["claves"]) == (3, 8, 3)


def test_cambios_sin_token_o_con_cursor_invalido(cliente_pg: TestClient) -> None:
    assert cliente_pg.get("/cambios").status_code == 401
    _, auth = grupo_de_tres(cliente_pg)
    invalido = cliente_pg.get("/cambios?desde=-1", headers=auth["ana"])
    assert invalido.status_code == 400
    assert invalido.json()["errores"][0]["campo"] == "desde"
    vacio = cliente_pg.get("/cambios", headers=auth["dario"])
    assert vacio.json() == {"eventos": [], "cursor": 0, "hay_mas": False}


def test_la_clave_de_un_usuario_no_choca_con_la_de_otro(
    cliente_pg: TestClient,
) -> None:
    """La clave es por usuario: la misma cadena de dos personas no colisiona."""
    grupo_id, auth = grupo_de_tres(cliente_pg)
    url = f"/grupos/{grupo_id}/gastos"
    clave = "clave-compartida-2026"

    de_ana = cliente_pg.post(
        url,
        json=gasto(
            "100.00",
            "ana",
            "partes_iguales",
            [("ana", ""), ("beto", ""), ("carla", "")],
            concepto="Hotel",
        ),
        headers={**auth["ana"], "Idempotency-Key": clave},
    )
    de_beto = cliente_pg.post(
        url,
        json=gasto(
            "60.00",
            "beto",
            "porcentajes",
            [("ana", "50"), ("beto", "25"), ("carla", "25")],
            concepto="Cena",
        ),
        headers={**auth["beto"], "Idempotency-Key": clave},
    )

    assert de_ana.status_code == 201, de_ana.text
    assert de_beto.status_code == 201, de_beto.text
    assert de_ana.json()["id"] != de_beto.json()["id"]

    assert URL_PRUEBAS is not None
    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        filas = conexion.execute(
            "SELECT usuario_id FROM claves_idempotencia ORDER BY usuario_id"
        ).fetchall()
    assert len(filas) == 2
    assert len({fila["usuario_id"] for fila in filas}) == 2


def _olvidar_repositorios(monkeypatch: pytest.MonkeyPatch) -> None:
    """Descarta los repositorios en memoria para que el arranque abra PostgreSQL."""
    for nombre in (
        "repositorio_usuarios",
        "repositorio_grupos",
        "repositorio_amistades",
        "repositorio_gastos",
        "repositorio_cambios",
    ):
        monkeypatch.delattr(app.state, nombre, raising=False)


def test_los_datos_sobreviven_al_reinicio(monkeypatch: pytest.MonkeyPatch) -> None:
    """Usuarios, grupos, gastos y cambios siguen ahí tras reiniciar el servidor."""
    assert URL_PRUEBAS is not None
    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()
    with base_datos.conexion() as conexion:
        conexion.execute("TRUNCATE TABLE usuarios CASCADE")

    monkeypatch.setenv("CUENTAS_CLARAS_DB_URL", URL_PRUEBAS)
    monkeypatch.setenv("CUENTAS_CLARAS_SECRETO_JWT", SECRETO)
    _olvidar_repositorios(monkeypatch)

    with TestClient(app) as cliente:
        grupo_id, auth = grupo_de_tres(cliente)
        creado = cliente.post(
            f"/grupos/{grupo_id}/gastos",
            json=gasto(
                "100.00",
                "ana",
                "partes_iguales",
                [("ana", ""), ("beto", ""), ("carla", "")],
                concepto="Hotel",
            ),
            headers={**auth["ana"], "Idempotency-Key": "hotel-reinicio-2026"},
        )
        assert creado.status_code == 201, creado.text
        gasto_id = creado.json()["id"]

    # Segundo arranque: repositorios nuevos contra la misma base.
    _olvidar_repositorios(monkeypatch)
    with TestClient(app) as cliente:
        detalle = cliente.get(f"/grupos/{grupo_id}", headers=auth["ana"])
        assert detalle.status_code == 200, detalle.text
        assert [g["id"] for g in detalle.json()["gastos"]] == [gasto_id]

        de_beto = cliente.get("/cambios", headers=auth["beto"])
        assert de_beto.status_code == 200, de_beto.text
        assert [e["gasto"]["id"] for e in de_beto.json()["eventos"]] == [gasto_id]

    with BaseDatos(URL_PRUEBAS).conexion() as conexion:
        conteos = conexion.execute(
            "SELECT (SELECT count(*) FROM usuarios) AS usuarios, "
            "(SELECT count(*) FROM grupos) AS grupos, "
            "(SELECT count(*) FROM gastos) AS gastos, "
            "(SELECT count(*) FROM cambios) AS cambios"
        ).fetchone()
    assert conteos is not None
    assert (
        conteos["usuarios"],
        conteos["grupos"],
        conteos["gastos"],
        conteos["cambios"],
    ) == (4, 1, 1, 3)
