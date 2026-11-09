"""Pruebas del inicio de sesión (POST /sesiones) y perfil propio (GET /usuarios/yo).

Cubre los requerimientos RF-SOC-02, RES-07 y los criterios de aceptación
de la sección 3 de la Entrega 5.
"""

import os
from datetime import datetime, timedelta, timezone

import jwt
import pytest
from fastapi.testclient import TestClient

MENSAJE_CREDENCIALES = {"mensaje": "Correo o contraseña incorrectos."}
MENSAJE_SESION = {"mensaje": "Sesión no válida o vencida."}

USUARIO = {
    "nombre_usuario": "ana.perez",
    "nombre_visible": "Ana Pérez",
    "correo": "ana@example.com",
    "contrasena": "clave-segura-1234",
}


@pytest.fixture
def id_registrado(cliente: TestClient) -> str:
    respuesta = cliente.post("/usuarios", json=USUARIO)
    assert respuesta.status_code == 201
    return str(respuesta.json()["id"])


def iniciar_sesion(cliente: TestClient) -> str:
    respuesta = cliente.post(
        "/sesiones",
        json={"correo": USUARIO["correo"], "contrasena": USUARIO["contrasena"]},
    )
    assert respuesta.status_code == 200
    return str(respuesta.json()["token"])


def secreto_prueba() -> str:
    """El secreto que conftest.py pone en JWT_SECRET."""
    return os.environ["JWT_SECRET"]


def firmar(carga: dict[str, object], secreto: str | None = None) -> str:
    return jwt.encode(carga, secreto or secreto_prueba(), algorithm="HS256")


def test_iniciar_sesion_credenciales_validas_retorna_200_con_token_y_vence_en(
    cliente: TestClient, id_registrado: str
) -> None:
    """Credenciales válidas entregan token Bearer firmado y fecha de vencimiento."""
    antes = datetime.now(timezone.utc).replace(microsecond=0)
    cuerpo_login = {
        "correo": "ana@example.com",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo_login)
    assert respuesta.status_code == 200

    datos = respuesta.json()
    assert "token" in datos and isinstance(datos["token"], str)
    assert len(datos["token"]) > 0
    assert datos.get("tipo") == "Bearer"

    vence_en = datetime.strptime(datos["vence_en"], "%Y-%m-%dT%H:%M:%SZ").replace(
        tzinfo=timezone.utc
    )
    esperado = antes + timedelta(minutes=int(os.environ["JWT_MINUTOS"]))
    assert abs((vence_en - esperado).total_seconds()) <= 5

    carga = jwt.decode(datos["token"], secreto_prueba(), algorithms=["HS256"])
    assert carga["sub"] == id_registrado
    assert carga["exp"] == int(vence_en.timestamp())


def test_iniciar_sesion_vigencia_configurable(
    cliente: TestClient, id_registrado: str, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Con JWT_MINUTOS=1 (la demostración), el token vence en un minuto."""
    monkeypatch.setenv("JWT_MINUTOS", "1")
    token = iniciar_sesion(cliente)
    carga = jwt.decode(token, secreto_prueba(), algorithms=["HS256"])
    assert carga["exp"] - carga["iat"] == 60


def test_iniciar_sesion_correo_case_insensitive(
    cliente: TestClient, id_registrado: str
) -> None:
    """El inicio de sesión encuentra al usuario sin importar mayúsculas en el correo."""
    cuerpo_login = {
        "correo": "ANA@EXAMPLE.COM",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo_login)
    assert respuesta.status_code == 200


def test_iniciar_sesion_usuario_inexistente_retorna_401_mensaje_identico(
    cliente: TestClient, id_registrado: str
) -> None:
    """Usuario inexistente devuelve 401 con mensaje idéntico a contraseña incorrecta."""
    cuerpo = {
        "correo": "noexiste@example.com",
        "contrasena": "clave-cualquiera-123",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo)
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_CREDENCIALES


def test_iniciar_sesion_contrasena_incorrecta_retorna_401_mensaje_identico(
    cliente: TestClient, id_registrado: str
) -> None:
    """Contraseña errónea devuelve 401 con mensaje idéntico a inexistente."""
    cuerpo = {
        "correo": "ana@example.com",
        "contrasena": "clave-totalmente-erronea",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo)
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_CREDENCIALES


def test_iniciar_sesion_respuestas_401_indistinguibles(
    cliente: TestClient, id_registrado: str
) -> None:
    """Correo inexistente y contraseña incorrecta: mismo código, cuerpo y cabeceras."""
    inexistente = cliente.post(
        "/sesiones", json={"correo": "otro@example.com", "contrasena": "x" * 12}
    )
    incorrecta = cliente.post(
        "/sesiones", json={"correo": "ana@example.com", "contrasena": "x" * 12}
    )
    assert inexistente.status_code == incorrecta.status_code == 401
    assert inexistente.content == incorrecta.content
    assert set(inexistente.headers.keys()) == set(incorrecta.headers.keys())


def test_iniciar_sesion_usuario_inexistente_verifica_contra_hash_falso(
    cliente: TestClient, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Si el correo no existe se calcula igual un argon2, para no delatar por tiempo."""
    import contrasenas

    llamadas: list[str] = []
    original = contrasenas.verificar

    def espia(hash_guardado: str, contrasena: str) -> bool:
        llamadas.append(hash_guardado)
        return original(hash_guardado, contrasena)

    monkeypatch.setattr(contrasenas, "verificar", espia)
    respuesta = cliente.post(
        "/sesiones", json={"correo": "nadie@example.com", "contrasena": "x" * 12}
    )
    assert respuesta.status_code == 401
    assert llamadas == [contrasenas.HASH_FALSO]


def test_iniciar_sesion_campos_faltantes_o_vacios_retorna_400(
    cliente: TestClient,
) -> None:
    """Falta de correo o contraseña o valores vacíos responde 400."""
    respuesta_vacio = cliente.post("/sesiones", json={})
    assert respuesta_vacio.status_code == 400
    errores = respuesta_vacio.json().get("errores", [])
    campos = {e.get("campo") for e in errores}
    assert campos == {"correo", "contrasena"}

    respuesta_blancos = cliente.post(
        "/sesiones", json={"correo": "  ", "contrasena": ""}
    )
    assert respuesta_blancos.status_code == 400


def test_perfil_propio_token_valido_retorna_200_sin_contrasena_ni_hash(
    cliente: TestClient, id_registrado: str
) -> None:
    """GET /usuarios/yo con token válido devuelve el perfil del usuario autenticado."""
    encabezados = {"Authorization": f"Bearer {iniciar_sesion(cliente)}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 200

    datos = respuesta.json()
    assert datos["id"] == id_registrado
    assert datos["nombre_usuario"] == "ana.perez"
    assert datos["nombre_visible"] == "Ana Pérez"
    assert datos["correo"] == "ana@example.com"
    assert "fecha_alta" in datos
    assert "contrasena" not in datos
    assert "hash_contrasena" not in datos
    assert "argon2" not in respuesta.text


def test_perfil_propio_token_ausente_retorna_401(cliente: TestClient) -> None:
    """GET /usuarios/yo sin encabezado de autorización responde 401."""
    respuesta = cliente.get("/usuarios/yo")
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == MENSAJE_SESION


def test_perfil_propio_token_invalido_o_alterado_retorna_401(
    cliente: TestClient, id_registrado: str
) -> None:
    """Token corrupto o manipulado responde 401."""
    for token in ["token.totalmente.alterado", iniciar_sesion(cliente)[:-3] + "AAA"]:
        encabezados = {"Authorization": f"Bearer {token}"}
        respuesta = cliente.get("/usuarios/yo", headers=encabezados)
        assert respuesta.status_code == 401
        assert respuesta.headers.get("www-authenticate") == "Bearer"
        assert respuesta.json() == MENSAJE_SESION


def test_perfil_propio_token_firmado_con_otro_secreto_retorna_401(
    cliente: TestClient, id_registrado: str
) -> None:
    """Token firmado con un secreto distinto al configurado responde 401."""
    vence = datetime.now(timezone.utc) + timedelta(minutes=5)
    token = firmar({"sub": id_registrado, "exp": vence}, secreto="z" * 40)
    encabezados = {"Authorization": f"Bearer {token}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_SESION


def test_perfil_propio_token_vencido_retorna_401(
    cliente: TestClient, id_registrado: str
) -> None:
    """Token cuya expiración ha transcurrido responde 401."""
    vencio = datetime.now(timezone.utc) - timedelta(seconds=1)
    token = firmar({"sub": id_registrado, "exp": vencio})
    encabezados = {"Authorization": f"Bearer {token}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_SESION


def test_perfil_propio_token_sin_vencimiento_retorna_401(
    cliente: TestClient, id_registrado: str
) -> None:
    """Un token sin ``exp`` no vale aunque la firma sea correcta."""
    encabezados = {"Authorization": f"Bearer {firmar({'sub': id_registrado})}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401


def test_perfil_propio_usuario_inexistente_retorna_401(cliente: TestClient) -> None:
    """Firma válida pero el usuario ya no existe en la base: 401."""
    vence = datetime.now(timezone.utc) + timedelta(minutes=5)
    token = firmar({"sub": "id-que-no-existe", "exp": vence})
    respuesta = cliente.get(
        "/usuarios/yo", headers={"Authorization": f"Bearer {token}"}
    )
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_SESION


def test_perfil_propio_esquema_distinto_a_bearer_retorna_401(
    cliente: TestClient,
) -> None:
    """Esquema de autorización diferente a Bearer (ej. Basic) responde 401."""
    encabezados = {"Authorization": "Basic dXN1YXJpbzpjbGF2ZQ=="}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.json() == MENSAJE_SESION
