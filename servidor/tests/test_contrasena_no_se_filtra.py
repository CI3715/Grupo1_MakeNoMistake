"""La contraseña nunca se guarda ni se muestra (RNF-05).

Cubre cuatro cosas: la respuesta del registro no la trae, el almacén de usuarios
solo recibe un hash que no se puede revertir, ningún log ni salida estándar la
contiene y los errores no repiten el valor enviado.

Se prueba contra el almacén en memoria de las pruebas (el mismo contrato que usa
``RepositorioUsuarios`` con PostgreSQL): lo que el servidor le entrega es lo que
llegaría a la base.
"""

import dataclasses
import logging

import pytest
from argon2 import PasswordHasher
from fastapi.testclient import TestClient
from repositorio_en_memoria import RepositorioEnMemoria

CONTRASENA = "clave-secreta-MUY-larga-2026"
CONTRASENA_INCORRECTA = "otra-clave-incorrecta-99"
CONTRASENA_CORTA = "corta-1"

DATOS = {
    "nombre_usuario": "ana.perez",
    "nombre_visible": "Ana Pérez",
    "correo": "ana@example.com",
    "contrasena": CONTRASENA,
}


def datos(**cambios: str) -> dict[str, str]:
    return {**DATOS, **cambios}


def campos(usuario: object) -> dict[str, object]:
    """Campos del usuario guardado, sea dataclass o una clase común."""
    if dataclasses.is_dataclass(usuario) and not isinstance(usuario, type):
        return dataclasses.asdict(usuario)
    return dict(vars(usuario))


def test_el_registro_no_devuelve_contrasena_ni_hash(cliente: TestClient) -> None:
    respuesta = cliente.post("/usuarios", json=DATOS)

    assert respuesta.status_code == 201
    assert CONTRASENA not in respuesta.text
    assert "argon2" not in respuesta.text
    assert not any(
        "contrasena" in campo or "hash" in campo for campo in respuesta.json()
    )


def test_el_almacen_solo_recibe_hashes_que_no_se_pueden_revertir(
    cliente: TestClient, repositorio_falso: RepositorioEnMemoria
) -> None:
    # Dos usuarios con la misma contraseña: cada uno recibe su propia sal.
    assert cliente.post("/usuarios", json=DATOS).status_code == 201
    otro = datos(nombre_usuario="luis.gomez", correo="luis@example.com")
    assert cliente.post("/usuarios", json=otro).status_code == 201

    hashes: list[str] = []
    for correo in ("ana@example.com", "luis@example.com"):
        usuario = repositorio_falso.buscar_por_correo(correo)
        assert usuario is not None
        valores = [str(valor) for valor in campos(usuario).values()]
        assert all(CONTRASENA not in valor for valor in valores)
        encontrados = [v for v in valores if v.startswith("$argon2id$")]
        assert len(encontrados) == 1
        hashes.append(encontrados[0])

    assert hashes[0] != hashes[1]
    for hash_guardado in hashes:
        assert hash_guardado != CONTRASENA
        assert PasswordHasher().verify(hash_guardado, CONTRASENA)


def test_la_contrasena_no_esta_en_ninguna_parte_del_almacen(
    cliente: TestClient, repositorio_falso: RepositorioEnMemoria
) -> None:
    assert cliente.post("/usuarios", json=DATOS).status_code == 201

    volcado = repr(getattr(repositorio_falso, "__dict__", {}))
    # Que el volcado de verdad contenga al usuario; si no, la prueba no vale.
    assert "ana.perez" in volcado
    assert CONTRASENA not in volcado


def test_la_contrasena_no_aparece_en_logs_ni_en_la_salida(
    cliente: TestClient,
    caplog: pytest.LogCaptureFixture,
    capsys: pytest.CaptureFixture[str],
) -> None:
    caplog.set_level(logging.DEBUG)

    assert cliente.post("/usuarios", json=DATOS).status_code == 201
    # Duplicado y datos inválidos.
    assert cliente.post("/usuarios", json=DATOS).status_code == 409
    corta = datos(
        nombre_usuario="pedro.ruiz",
        correo="pedro@example.com",
        contrasena=CONTRASENA_CORTA,
    )
    assert cliente.post("/usuarios", json=corta).status_code == 400
    # Inicio de sesión correcto, con contraseña incorrecta y con usuario inexistente.
    correcto = cliente.post(
        "/sesiones", json={"correo": DATOS["correo"], "contrasena": CONTRASENA}
    )
    assert correcto.status_code == 200
    incorrecto = cliente.post(
        "/sesiones",
        json={"correo": DATOS["correo"], "contrasena": CONTRASENA_INCORRECTA},
    )
    assert incorrecto.status_code == 401
    inexistente = cliente.post(
        "/sesiones",
        json={"correo": "nadie@example.com", "contrasena": CONTRASENA_INCORRECTA},
    )
    assert inexistente.status_code == 401
    # Perfil con el token.
    token = correcto.json()["token"]
    perfil = cliente.get("/usuarios/yo", headers={"Authorization": f"Bearer {token}"})
    assert perfil.status_code == 200

    salida = capsys.readouterr()
    registrado = caplog.text + salida.out + salida.err
    for secreto in (CONTRASENA, CONTRASENA_INCORRECTA, CONTRASENA_CORTA):
        assert secreto not in registrado


@pytest.mark.parametrize(
    "cambios",
    [
        {"contrasena": CONTRASENA_CORTA},
        {"correo": "no-es-un-correo"},
        {"nombre_usuario": "NO VÁLIDO"},
        {"nombre_visible": "   "},
    ],
)
def test_los_errores_del_registro_no_repiten_la_contrasena(
    cliente: TestClient, cambios: dict[str, str]
) -> None:
    cuerpo = datos(**cambios)
    respuesta = cliente.post("/usuarios", json=cuerpo)

    assert respuesta.status_code == 400
    assert cuerpo["contrasena"] not in respuesta.text


def test_los_errores_de_sesion_no_repiten_la_contrasena(cliente: TestClient) -> None:
    assert cliente.post("/usuarios", json=DATOS).status_code == 201

    incorrecta = cliente.post(
        "/sesiones",
        json={"correo": DATOS["correo"], "contrasena": CONTRASENA_INCORRECTA},
    )
    sin_campo = cliente.post("/sesiones", json={"contrasena": CONTRASENA_INCORRECTA})

    assert incorrecta.status_code == 401
    assert sin_campo.status_code == 400
    assert CONTRASENA_INCORRECTA not in incorrecta.text
    assert CONTRASENA_INCORRECTA not in sin_campo.text
