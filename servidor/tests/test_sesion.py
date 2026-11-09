"""Pruebas del inicio de sesión (POST /sesiones) y perfil propio (GET /usuarios/yo).

Cubre los requerimientos RF-SOC-02, RES-07 y los criterios de aceptación
de la sección 3 de la Entrega 5.
"""

import pytest
from fastapi.testclient import TestClient

from base_datos import insertar_usuario
from seguridad import crear_token_sesion, hashear_contrasena

PENDIENTE_LOGIN = "Pendiente de integración de POST /sesiones por John"


@pytest.mark.skip(reason=PENDIENTE_LOGIN)
def test_iniciar_sesion_credenciales_validas_retorna_200_con_token_y_vence_en(
    cliente: TestClient,
) -> None:
    """Credenciales válidas entregan token Bearer firmado y fecha de vencimiento."""
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
    assert "vence_en" in datos


@pytest.mark.skip(reason=PENDIENTE_LOGIN)
def test_iniciar_sesion_correo_case_insensitive(cliente: TestClient) -> None:
    """El inicio de sesión encuentra al usuario sin importar mayúsculas en el correo."""
    cuerpo_login = {
        "correo": "ANA@EXAMPLE.COM",
        "contrasena": "clave-segura-1234",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo_login)
    assert respuesta.status_code == 200


@pytest.mark.skip(reason=PENDIENTE_LOGIN)
def test_iniciar_sesion_usuario_inexistente_retorna_401_mensaje_identico(
    cliente: TestClient,
) -> None:
    """Usuario inexistente devuelve 401 con mensaje idéntico a contraseña incorrecta."""
    cuerpo = {
        "correo": "noexiste@example.com",
        "contrasena": "clave-cualquiera-123",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo)
    assert respuesta.status_code == 401
    assert respuesta.json() == {"mensaje": "Correo o contraseña incorrectos."}


@pytest.mark.skip(reason=PENDIENTE_LOGIN)
def test_iniciar_sesion_contrasena_incorrecta_retorna_401_mensaje_identico(
    cliente: TestClient,
) -> None:
    """Contraseña errónea devuelve 401 con mensaje idéntico a inexistente."""
    cuerpo = {
        "correo": "ana@example.com",
        "contrasena": "clave-totalmente-erronea",
    }
    respuesta = cliente.post("/sesiones", json=cuerpo)
    assert respuesta.status_code == 401
    assert respuesta.json() == {"mensaje": "Correo o contraseña incorrectos."}


@pytest.mark.skip(reason=PENDIENTE_LOGIN)
def test_iniciar_sesion_campos_faltantes_o_vacios_retorna_400(
    cliente: TestClient,
) -> None:
    """Falta de correo o contraseña o valores vacíos responde 400."""
    respuesta_vacio = cliente.post("/sesiones", json={})
    assert respuesta_vacio.status_code == 400
    errores = respuesta_vacio.json().get("errores", [])
    campos = {e.get("campo") for e in errores}
    assert "correo" in campos or "contrasena" in campos


def test_perfil_propio_token_valido_retorna_200_sin_contrasena_ni_hash(
    cliente: TestClient,
) -> None:
    """GET /usuarios/yo con token válido devuelve el perfil del usuario autenticado."""
    usuario = insertar_usuario(
        id_usuario="usr-1234-uuid",
        nombre_usuario="ana.perez",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        hash_contrasena=hashear_contrasena("clave-segura-1234"),
        fecha_alta="2026-11-09T10:00:00Z",
    )
    token, _ = crear_token_sesion(usuario["id"])

    encabezados = {"Authorization": f"Bearer {token}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 200

    datos = respuesta.json()
    assert datos["id"] == "usr-1234-uuid"
    assert datos["nombre_usuario"] == "ana.perez"
    assert datos["nombre_visible"] == "Ana Pérez"
    assert datos["correo"] == "ana@example.com"
    assert datos["fecha_alta"] == "2026-11-09T10:00:00Z"
    assert "contrasena" not in datos
    assert "hash_contrasena" not in datos


def test_perfil_propio_token_ausente_retorna_401(cliente: TestClient) -> None:
    """GET /usuarios/yo sin encabezado de autorización responde 401."""
    respuesta = cliente.get("/usuarios/yo")
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == {"mensaje": "Sesión no válida o vencida."}


def test_perfil_propio_token_invalido_o_alterado_retorna_401(
    cliente: TestClient,
) -> None:
    """Token corrupto o manipulado responde 401."""
    token, _ = crear_token_sesion("usr-1234")
    token_alterado = token[:-5] + "XXXXX"
    encabezados = {"Authorization": f"Bearer {token_alterado}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == {"mensaje": "Sesión no válida o vencida."}


def test_perfil_propio_token_firmado_con_otro_secreto_retorna_401(
    cliente: TestClient,
) -> None:
    """Token firmado con un secreto distinto al configurado responde 401."""
    token, _ = crear_token_sesion(
        "usr-1234", secreto="otro-secreto-totalmente-distinto"
    )
    encabezados = {"Authorization": f"Bearer {token}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == {"mensaje": "Sesión no válida o vencida."}


def test_perfil_propio_token_vencido_retorna_401(cliente: TestClient) -> None:
    """Token cuya expiración ha transcurrido responde 401."""
    token, _ = crear_token_sesion("usr-1234", minutos=-5)
    encabezados = {"Authorization": f"Bearer {token}"}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == {"mensaje": "Sesión no válida o vencida."}


def test_perfil_propio_esquema_distinto_a_bearer_retorna_401(
    cliente: TestClient,
) -> None:
    """Esquema de autorización diferente a Bearer (ej. Basic) responde 401."""
    encabezados = {"Authorization": "Basic dXN1YXJpbzpjbGF2ZQ=="}
    respuesta = cliente.get("/usuarios/yo", headers=encabezados)
    assert respuesta.status_code == 401
    assert respuesta.headers.get("www-authenticate") == "Bearer"
    assert respuesta.json() == {"mensaje": "Sesión no válida o vencida."}
