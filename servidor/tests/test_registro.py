"""Pruebas del endpoint de registro de usuarios (POST /usuarios).

Cubre los requerimientos RF-SOC-01, RNF-05, INV-16 y los criterios de la
sección 3 de la especificación de la Entrega 5.
"""

import sqlite3
from pathlib import Path

from argon2 import PasswordHasher
from fastapi.testclient import TestClient


def hash_guardado(bd_prueba: Path, nombre_usuario: str) -> str:
    with sqlite3.connect(bd_prueba) as conexion:
        fila = conexion.execute(
            "SELECT hash_contrasena FROM usuarios WHERE nombre_usuario = ?",
            (nombre_usuario,),
        ).fetchone()
    assert fila is not None
    return str(fila[0])


def test_registro_valido_retorna_201_y_datos_sin_contrasena(
    cliente: TestClient,
) -> None:
    """Un registro válido responde 201 Created y devuelve los datos del usuario.

    No debe incluir ni contraseña en texto plano ni el hash. La fecha de alta
    debe tener el formato UTC AAAA-MM-DDThh:mm:ssZ.
    """
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 201

    datos = respuesta.json()
    assert "id" in datos and isinstance(datos["id"], str) and len(datos["id"]) > 0
    assert datos["nombre_usuario"] == "ana.perez"
    assert datos["nombre_visible"] == "Ana Pérez"
    assert datos["correo"] == "ana@example.com"
    assert "fecha_alta" in datos
    assert "contrasena" not in datos
    assert "hash_contrasena" not in datos
    assert "hash" not in datos


def test_registro_nombre_usuario_menor_a_3_caracteres_retorna_400(
    cliente: TestClient,
) -> None:
    """Nombre de usuario con menos de 3 caracteres se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "an",
        "nombre_visible": "Ana",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_usuario" for e in errores)


def test_registro_nombre_usuario_mayor_a_30_caracteres_retorna_400(
    cliente: TestClient,
) -> None:
    """Nombre de usuario con más de 30 caracteres se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "a" * 31,
        "nombre_visible": "Ana",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_usuario" for e in errores)


def test_registro_nombre_usuario_con_mayusculas_retorna_400(
    cliente: TestClient,
) -> None:
    """Nombre de usuario con mayúsculas se rechaza con 400 (no se normaliza)."""
    cuerpo = {
        "nombre_usuario": "AnaPerez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_usuario" for e in errores)


def test_registro_nombre_usuario_con_caracteres_invalidos_retorna_400(
    cliente: TestClient,
) -> None:
    """Caracteres distintos de minúsculas, números, punto o guion bajo dan 400."""
    for invalido in ["ana-perez", "ana perez", "ana@perez", "ana$"]:
        cuerpo = {
            "nombre_usuario": invalido,
            "nombre_visible": "Ana",
            "correo": "ana@example.com",
            "contrasena": "clave-segura-1234",
        }
        respuesta = cliente.post("/usuarios", json=cuerpo)
        assert respuesta.status_code == 400
        errores = respuesta.json().get("errores", [])
        assert any(e.get("campo") == "nombre_usuario" for e in errores)


def test_registro_nombre_visible_vacio_retorna_400(cliente: TestClient) -> None:
    """Nombre visible vacío se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_visible" for e in errores)


def test_registro_nombre_visible_solo_espacios_retorna_400(
    cliente: TestClient,
) -> None:
    """Nombre visible con solo espacios en blanco se recorta y se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "     ",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_visible" for e in errores)


def test_registro_nombre_visible_mayor_a_50_caracteres_retorna_400(
    cliente: TestClient,
) -> None:
    """Nombre visible con más de 50 caracteres se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "A" * 51,
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "nombre_visible" for e in errores)


def test_registro_correo_sin_arroba_retorna_400(cliente: TestClient) -> None:
    """Correo sin arroba se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana-example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "correo" for e in errores)


def test_registro_correo_sin_dominio_retorna_400(cliente: TestClient) -> None:
    """Correo sin dominio después de la arroba se rechaza con 400."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "correo" for e in errores)


def test_registro_contrasena_menor_a_10_caracteres_retorna_400(
    cliente: TestClient,
) -> None:
    """Contraseña de 9 caracteres se rechaza con 400 (exige al menos 10)."""
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@example.com",
        "contrasena": "123456789",  # 9 caracteres
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    assert any(e.get("campo") == "contrasena" for e in errores)


def test_registro_multiples_campos_invalidos_informa_todos(
    cliente: TestClient,
) -> None:
    """Si fallan varios campos, se informan todos juntos en la respuesta 400."""
    cuerpo = {
        "nombre_usuario": "An",  # muy corto y con mayúscula
        "nombre_visible": "",  # vacío
        "correo": "correo-malo",  # sin arroba
        "contrasena": "corta",  # < 10 caracteres
    }
    respuesta = cliente.post("/usuarios", json=cuerpo)
    assert respuesta.status_code == 400
    errores = respuesta.json().get("errores", [])
    campos_fallidos = {e.get("campo") for e in errores}
    assert {"nombre_usuario", "nombre_visible", "correo", "contrasena"}.issubset(
        campos_fallidos
    )


def test_registro_nombre_usuario_repetido_retorna_409(cliente: TestClient) -> None:
    """Registrar un nombre de usuario ya existente responde 409 indicando el campo."""
    cuerpo1 = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana1@example.com",
        "contrasena": "clave-segura-1234",
    }
    cuerpo2 = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Otra",
        "correo": "ana2@example.com",
        "contrasena": "otra-clave-segura-1234",
    }
    resp1 = cliente.post("/usuarios", json=cuerpo1)
    assert resp1.status_code == 201

    resp2 = cliente.post("/usuarios", json=cuerpo2)
    assert resp2.status_code == 409
    errores = resp2.json().get("errores", [])
    assert any(e.get("campo") == "nombre_usuario" for e in errores)


def test_registro_correo_repetido_retorna_409(cliente: TestClient) -> None:
    """Registrar un correo ya existente responde 409 indicando el campo correo."""
    cuerpo1 = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    cuerpo2 = {
        "nombre_usuario": "ana.otra",
        "nombre_visible": "Ana Otra",
        "correo": "ana@example.com",
        "contrasena": "otra-clave-segura-1234",
    }
    resp1 = cliente.post("/usuarios", json=cuerpo1)
    assert resp1.status_code == 201

    resp2 = cliente.post("/usuarios", json=cuerpo2)
    assert resp2.status_code == 409
    errores = resp2.json().get("errores", [])
    assert any(e.get("campo") == "correo" for e in errores)


def test_registro_correo_repetido_distinta_capitalizacion_retorna_409(
    cliente: TestClient,
) -> None:
    """La comparación de correo no distingue mayúsculas; detecta conflicto con 409."""
    cuerpo1 = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana Pérez",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    cuerpo2 = {
        "nombre_usuario": "ana.mayus",
        "nombre_visible": "Ana Mayúsculas",
        "correo": "ANA@EXAMPLE.COM",
        "contrasena": "otra-clave-segura-1234",
    }
    resp1 = cliente.post("/usuarios", json=cuerpo1)
    assert resp1.status_code == 201

    resp2 = cliente.post("/usuarios", json=cuerpo2)
    assert resp2.status_code == 409
    errores = resp2.json().get("errores", [])
    assert any(e.get("campo") == "correo" for e in errores)


def test_registro_contrasena_guardada_como_hash_argon2id_con_sal_unica(
    cliente: TestClient, bd_prueba: Path
) -> None:
    """Misma contraseña produce hashes distintos en usuarios diferentes."""
    cuerpo1 = {
        "nombre_usuario": "usuario.uno",
        "nombre_visible": "Usuario Uno",
        "correo": "uno@example.com",
        "contrasena": "misma-contrasena-1234",
    }
    cuerpo2 = {
        "nombre_usuario": "usuario.dos",
        "nombre_visible": "Usuario Dos",
        "correo": "dos@example.com",
        "contrasena": "misma-contrasena-1234",
    }
    resp1 = cliente.post("/usuarios", json=cuerpo1)
    resp2 = cliente.post("/usuarios", json=cuerpo2)
    assert resp1.status_code == 201
    assert resp2.status_code == 201

    hash1 = hash_guardado(bd_prueba, "usuario.uno")
    hash2 = hash_guardado(bd_prueba, "usuario.dos")
    assert hash1.startswith("$argon2id$")
    assert hash2.startswith("$argon2id$")
    assert hash1 != hash2


def test_registro_contrasena_guardada_no_es_recuperable(
    cliente: TestClient, bd_prueba: Path
) -> None:
    """La contraseña almacenada es un hash unidireccional y no puede descifrarse."""
    cuerpo = {
        "nombre_usuario": "carlos.ruiz",
        "nombre_visible": "Carlos Ruiz",
        "correo": "carlos@example.com",
        "contrasena": "clave-super-secreta-1234",
    }
    resp = cliente.post("/usuarios", json=cuerpo)
    assert resp.status_code == 201

    guardado = hash_guardado(bd_prueba, "carlos.ruiz")
    assert "clave-super-secreta-1234" not in guardado
    assert PasswordHasher().verify(guardado, "clave-super-secreta-1234")


def test_registro_no_repite_contrasena_en_respuestas_ni_errores(
    cliente: TestClient,
) -> None:
    """La contraseña nunca debe figurar en el cuerpo de ninguna respuesta (RNF-05)."""
    clave_secreta = "mi-clave-super-secreta-1234"
    cuerpo = {
        "nombre_usuario": "secreto.user",
        "nombre_visible": "Secreto",
        "correo": "secreto@example.com",
        "contrasena": clave_secreta,
    }
    resp = cliente.post("/usuarios", json=cuerpo)
    assert clave_secreta not in resp.text


def test_registro_invalido_no_repite_contrasena_en_errores(
    cliente: TestClient,
) -> None:
    """Ni el 400 propio ni el de tipos repiten el valor enviado (RNF-05)."""
    clave = "clave-que-no-debe-salir-1234"
    cuerpos = [
        {"nombre_usuario": "x", "correo": "malo", "contrasena": clave},
        {"nombre_usuario": 123, "correo": ["a"], "contrasena": clave},
    ]
    for cuerpo in cuerpos:
        resp = cliente.post("/usuarios", json=cuerpo)
        assert resp.status_code == 400
        assert clave not in resp.text
        assert "input" not in resp.text


def test_registro_campo_con_tipo_incorrecto_retorna_400_y_no_422(
    cliente: TestClient,
) -> None:
    cuerpo = {
        "nombre_usuario": 123,
        "nombre_visible": "Ana",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    resp = cliente.post("/usuarios", json=cuerpo)
    assert resp.status_code == 400
    assert resp.json() == {
        "errores": [
            {"campo": "nombre_usuario", "mensaje": "Este campo debe ser texto."}
        ]
    }


def test_registro_cuerpo_no_json_retorna_400(cliente: TestClient) -> None:
    resp = cliente.post(
        "/usuarios",
        content=b"esto no es json",
        headers={"Content-Type": "application/json"},
    )
    assert resp.status_code == 400
    assert resp.json()["errores"][0]["campo"] == "cuerpo"


def test_registro_campos_ausentes_se_informan_todos(cliente: TestClient) -> None:
    resp = cliente.post("/usuarios", json={})
    assert resp.status_code == 400
    campos = {e["campo"] for e in resp.json()["errores"]}
    assert campos == {"nombre_usuario", "nombre_visible", "correo", "contrasena"}


def test_registro_normaliza_correo_y_recorta_nombre_visible(
    cliente: TestClient,
) -> None:
    cuerpo = {
        "nombre_usuario": "ana_p",
        "nombre_visible": "  Ana Pérez  ",
        "correo": "  Ana@Example.COM ",
        "contrasena": "clave-segura-1234",
    }
    resp = cliente.post("/usuarios", json=cuerpo)
    assert resp.status_code == 201
    assert resp.json()["correo"] == "ana@example.com"
    assert resp.json()["nombre_visible"] == "Ana Pérez"


def test_registro_usuario_y_correo_repetidos_informa_ambos(
    cliente: TestClient,
) -> None:
    cuerpo = {
        "nombre_usuario": "ana.perez",
        "nombre_visible": "Ana",
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    assert cliente.post("/usuarios", json=cuerpo).status_code == 201
    resp = cliente.post("/usuarios", json=cuerpo)
    assert resp.status_code == 409
    assert {e["campo"] for e in resp.json()["errores"]} == {
        "nombre_usuario",
        "correo",
    }
