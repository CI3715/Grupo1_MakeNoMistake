"""Los datos privados no salen por las rutas sociales (RF-SOC-04, RNF-05).

Recorre el flujo completo con personas reales (registro, inicio de sesión,
amistades, grupos y gastos) y comprueba cuatro cosas:

1. ninguna respuesta de la búsqueda, las amistades o los grupos trae un correo,
   una contraseña ni un hash, y el token solo sale al iniciar sesión;
2. la búsqueda solo encuentra el nombre de usuario exacto: una búsqueda parcial,
   por prefijo, con comodines, por correo o vacía no devuelve a nadie, así que no
   se puede enumerar quién está registrado;
3. sin una sesión válida la búsqueda no revela nada;
4. ni los logs ni la salida estándar contienen contraseñas, tokens ni correos.

Se usa el almacén en memoria de las pruebas, que cumple el mismo contrato que el
de PostgreSQL.
"""

import json
import logging
from collections.abc import Generator
from dataclasses import dataclass
from typing import Any

import pytest
from fastapi.testclient import TestClient
from flujo_http import (
    CONTRASENA,
    grupo_de_tres,
    hacerse_amigos,
    invitar,
    registrar_ejemplo_2_5,
)

from main import app

USUARIOS = ("ana", "beto", "carla", "dario")
CONTRASENA_INCORRECTA = "otra-clave-incorrecta-99"
TOKEN_FALSO = "token-falso-que-no-debe-aparecer"

# Las únicas respuestas que pueden traer el correo: las de la propia persona.
PROPIAS = {("POST", "/usuarios"), ("GET", "/usuarios/yo")}
CAMPOS_PRIVADOS = {
    "correo",
    "email",
    "contrasena",
    "password",
    "hash",
    "hash_contrasena",
}


@dataclass(frozen=True)
class Llamada:
    """Una respuesta que dio el servidor durante la prueba."""

    metodo: str
    ruta: str
    estado: int
    texto: str


class ClienteConHistorial(TestClient):
    """Cliente de pruebas que recuerda todo lo que el servidor respondió."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.historial: list[Llamada] = []

    def request(self, *args: Any, **kwargs: Any) -> Any:
        respuesta = super().request(*args, **kwargs)
        self.historial.append(
            Llamada(
                metodo=respuesta.request.method,
                ruta=respuesta.request.url.path,
                estado=respuesta.status_code,
                texto=respuesta.text,
            )
        )
        return respuesta


@dataclass(frozen=True)
class Recorrido:
    cliente: ClienteConHistorial
    grupo_id: str
    auth: dict[str, dict[str, str]]

    @property
    def historial(self) -> list[Llamada]:
        return self.cliente.historial

    def token(self, usuario: str) -> str:
        return self.auth[usuario]["Authorization"].split(" ", 1)[1]


def recorrer(cliente: ClienteConHistorial) -> Recorrido:
    """ana, beto, carla y dario usan todas las rutas sociales con datos reales."""
    grupo_id, auth = grupo_de_tres(cliente)
    registrar_ejemplo_2_5(cliente, grupo_id, auth)

    # Una invitación de amistad que queda pendiente entre beto y carla.
    pendiente = cliente.post(
        "/amistades", json={"usuario": "carla"}, headers=auth["beto"]
    )
    assert pendiente.status_code == 201, pendiente.text
    # dario es amigo de ana y tiene una invitación al grupo sin responder.
    hacerse_amigos(cliente, auth["ana"], "dario", auth["dario"])
    invitar(cliente, grupo_id, auth["ana"], "dario")

    for usuario in USUARIOS:
        amistades = cliente.get("/amistades", headers=auth[usuario])
        assert amistades.status_code == 200, amistades.text
        grupos = cliente.get("/grupos", headers=auth[usuario])
        assert grupos.status_code == 200, grupos.text
    for usuario in ("ana", "beto", "carla"):
        detalle = cliente.get(f"/grupos/{grupo_id}", headers=auth[usuario])
        assert detalle.status_code == 200, detalle.text
    for buscado in ("beto", "carla", "dario"):
        busqueda = cliente.get(
            "/usuarios", params={"usuario": buscado}, headers=auth["ana"]
        )
        assert busqueda.status_code == 200, busqueda.text
    return Recorrido(cliente=cliente, grupo_id=grupo_id, auth=auth)


@pytest.fixture
def registrado() -> Generator[ClienteConHistorial, None, None]:
    cliente = ClienteConHistorial(app)
    with cliente:
        yield cliente


@pytest.fixture
def recorrido(registrado: ClienteConHistorial) -> Recorrido:
    return recorrer(registrado)


def claves(valor: object) -> set[str]:
    """Todos los nombres de campo de un JSON, a cualquier profundidad."""
    if isinstance(valor, dict):
        encontradas = {str(clave) for clave in valor}
        for hijo in valor.values():
            encontradas |= claves(hijo)
        return encontradas
    if isinstance(valor, list):
        encontradas = set()
        for hijo in valor:
            encontradas |= claves(hijo)
        return encontradas
    return set()


def buscar(recorrido: Recorrido, buscado: dict[str, str]) -> Any:
    return recorrido.cliente.get(
        "/usuarios", params=buscado, headers=recorrido.auth["ana"]
    )


# ------------------------------------------------- 1. Lo que devuelven las rutas


def test_el_recorrido_pasa_por_las_rutas_sociales(recorrido: Recorrido) -> None:
    """Si el recorrido no tocara estas rutas, las demás pruebas no valdrían."""
    vistas = {(llamada.metodo, llamada.ruta) for llamada in recorrido.historial}
    grupo = f"/grupos/{recorrido.grupo_id}"
    for esperada in (
        ("GET", "/usuarios"),
        ("POST", "/amistades"),
        ("GET", "/amistades"),
        ("POST", "/grupos"),
        ("GET", "/grupos"),
        ("POST", f"{grupo}/miembros"),
        ("POST", f"{grupo}/miembros/yo/respuesta"),
        ("POST", f"{grupo}/gastos"),
        ("GET", grupo),
    ):
        assert esperada in vistas, esperada


def test_ninguna_respuesta_social_trae_un_correo(recorrido: Recorrido) -> None:
    for llamada in recorrido.historial:
        if (llamada.metodo, llamada.ruta) in PROPIAS:
            continue
        assert "@" not in llamada.texto, (
            f"{llamada.metodo} {llamada.ruta} trae un correo"
        )


def test_ninguna_respuesta_social_trae_campos_privados(
    recorrido: Recorrido,
) -> None:
    for llamada in recorrido.historial:
        if (llamada.metodo, llamada.ruta) in PROPIAS or not llamada.texto:
            continue
        privados = claves(json.loads(llamada.texto)) & CAMPOS_PRIVADOS
        assert not privados, f"{llamada.metodo} {llamada.ruta} trae {privados}"
        assert "argon2" not in llamada.texto


def test_el_token_solo_sale_al_iniciar_sesion(recorrido: Recorrido) -> None:
    for llamada in recorrido.historial:
        if (llamada.metodo, llamada.ruta) == ("POST", "/sesiones"):
            continue
        for usuario in USUARIOS:
            assert recorrido.token(usuario) not in llamada.texto, (
                f"{llamada.metodo} {llamada.ruta} devuelve un token"
            )


def test_la_busqueda_exacta_solo_trae_usuario_y_nombre_visible(
    recorrido: Recorrido,
) -> None:
    respuesta = buscar(recorrido, {"usuario": "beto"})

    assert respuesta.status_code == 200
    assert respuesta.json() == {"nombre_usuario": "beto", "nombre_visible": "Beto"}


# ----------------------------------------- 2. No se puede enumerar a las personas


@pytest.mark.parametrize(
    "busqueda",
    [
        # parciales y por prefijo de personas que sí existen
        "an",
        "b",
        "bet",
        "eto",
        "car",
        "dar",
        # no es el nombre exacto
        "ANA",
        "ana.",
        "beto,ana",
        # comodines de SQL y de shell
        "%",
        "%beto%",
        "beto%",
        "_eto",
        "*",
        ".*",
        # por correo, completo o a medias
        "ana@ejemplo.com",
        "@ejemplo.com",
        "ejemplo.com",
    ],
)
def test_una_busqueda_que_no_es_exacta_no_devuelve_a_nadie(
    recorrido: Recorrido, busqueda: str
) -> None:
    respuesta = buscar(recorrido, {"usuario": busqueda})

    assert respuesta.status_code == 404
    assert respuesta.json() == {"mensaje": "Usuario no encontrado."}


def test_un_usuario_que_no_existe_responde_igual_que_una_busqueda_parcial(
    recorrido: Recorrido,
) -> None:
    inexistente = buscar(recorrido, {"usuario": "fantasma"})
    parcial = buscar(recorrido, {"usuario": "bet"})

    assert inexistente.status_code == parcial.status_code == 404
    assert inexistente.text == parcial.text


@pytest.mark.parametrize("busqueda", [{}, {"usuario": ""}, {"usuario": "   "}])
def test_una_busqueda_vacia_no_lista_a_nadie(
    recorrido: Recorrido, busqueda: dict[str, str]
) -> None:
    respuesta = buscar(recorrido, busqueda)

    assert respuesta.status_code == 400
    cuerpo = respuesta.json()
    assert set(cuerpo) == {"errores"}
    assert [error["campo"] for error in cuerpo["errores"]] == ["usuario"]


# ------------------------------------------------ 3. Sin sesión no se revela nada


@pytest.mark.parametrize(
    "encabezados",
    [
        {},
        {"Authorization": f"Bearer {TOKEN_FALSO}"},
        {"Authorization": "Basic YW5hOnh4"},
    ],
)
def test_sin_sesion_valida_la_busqueda_no_revela_nada(
    recorrido: Recorrido, encabezados: dict[str, str]
) -> None:
    existente = recorrido.cliente.get(
        "/usuarios", params={"usuario": "beto"}, headers=encabezados
    )
    inexistente = recorrido.cliente.get(
        "/usuarios", params={"usuario": "fantasma"}, headers=encabezados
    )

    assert existente.status_code == inexistente.status_code == 401
    assert existente.text == inexistente.text
    assert "beto" not in existente.text
    assert TOKEN_FALSO not in existente.text


# ---------------------------------------------- 4. Los logs y la salida estándar


def test_ningun_log_ni_salida_contiene_secretos(
    registrado: ClienteConHistorial,
    caplog: pytest.LogCaptureFixture,
    capsys: pytest.CaptureFixture[str],
) -> None:
    caplog.set_level(logging.DEBUG)

    recorrido = recorrer(registrado)
    # Peticiones con credenciales que no sirven.
    sin_valor = registrado.get(
        "/usuarios",
        params={"usuario": "beto"},
        headers={"Authorization": f"Bearer {TOKEN_FALSO}"},
    )
    assert sin_valor.status_code == 401
    incorrecto = registrado.post(
        "/sesiones",
        json={"correo": "ana@ejemplo.com", "contrasena": CONTRASENA_INCORRECTA},
    )
    assert incorrecto.status_code == 401

    salida = capsys.readouterr()
    visto = caplog.text + salida.out + salida.err

    tokens = [recorrido.token(usuario) for usuario in USUARIOS]
    assert all(tokens)
    secretos = [CONTRASENA, CONTRASENA_INCORRECTA, TOKEN_FALSO, "argon2", *tokens]
    secretos += [f"{usuario}@ejemplo.com" for usuario in USUARIOS]
    for secreto in secretos:
        assert secreto not in visto, f"el log o la salida contiene {secreto!r}"
