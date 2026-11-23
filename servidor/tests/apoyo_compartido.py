"""Utilidades compartidas por las pruebas de la Entrega 6 y Entrega 7."""

from typing import Any

import pytest
from fastapi.testclient import TestClient

from repositorio_usuarios import RepositorioUsuarios, Usuario


def crear_usuario(repositorio: RepositorioUsuarios, nombre: str) -> Usuario:
    """Crea una persona con datos mínimos y nombres visibles legibles."""

    return repositorio.crear(
        nombre_usuario=nombre,
        nombre_visible=nombre.replace(".", " ").title(),
        correo=f"{nombre}@example.com",
        hash_contrasena="hash",
    )


# ---------- Guarda anti-borrado para Supabase (Angel - Tarea 3) ----------

MENSAJE_ERROR_SUPABASE = (
    "CUENTAS_CLARAS_DB_URL_PRUEBAS no puede apuntar a Supabase (haría TRUNCATE "
    "y borraría los datos compartidos del equipo). Usa PostgreSQL local con "
    "Docker Compose en el puerto 55432."
)


def es_host_supabase(url: str | None) -> bool:
    """Detecta si la URL apunta a un host o pooler de Supabase."""
    if not url:
        return False
    url_baja = url.lower()
    return (
        "supabase.co" in url_baja
        or "pooler.supabase.com" in url_baja
        or "supabase" in url_baja
    )


def verificar_no_supabase(url: str | None) -> None:
    """Impide que las pruebas de integración con TRUNCATE corran contra Supabase."""
    if es_host_supabase(url):
        pytest.fail(MENSAJE_ERROR_SUPABASE)


# ---------- Ayudantes del flujo Entrega 7 (Angel - Tarea 2) ----------


def crear_grupo_merida_http(
    cliente: TestClient, usuarios: dict[str, dict[str, Any]]
) -> str:
    """Crea el grupo «Viaje a Mérida» en USD con ana, beto y carla aceptados.

    dario queda registrado pero fuera del grupo.
    """
    from flujo_http import aceptar_grupo, hacerse_amigos, invitar

    auth_ana = usuarios["ana"]["auth"]
    auth_beto = usuarios["beto"]["auth"]
    auth_carla = usuarios["carla"]["auth"]

    # ana se hace amiga de beto y carla
    hacerse_amigos(cliente, auth_ana, "beto", auth_beto)
    hacerse_amigos(cliente, auth_ana, "carla", auth_carla)

    # ana crea el grupo «Viaje a Mérida» en USD
    resp_grupo = cliente.post(
        "/grupos",
        json={"nombre": "Viaje a Mérida", "moneda": "USD", "actividad": "vacaciones"},
        headers=auth_ana,
    )
    assert resp_grupo.status_code == 201, resp_grupo.text
    grupo_id = str(resp_grupo.json()["id"])

    # ana invita a beto y carla, y ambos aceptan
    invitar(cliente, grupo_id, auth_ana, "beto")
    aceptar_grupo(cliente, grupo_id, auth_beto)

    invitar(cliente, grupo_id, auth_ana, "carla")
    aceptar_grupo(cliente, grupo_id, auth_carla)

    return grupo_id


def registrar_gasto_idempotente_http(
    cliente: TestClient,
    grupo_id: str,
    auth: dict[str, str],
    cuerpo: dict[str, Any],
    clave_idempotencia: str | None = None,
) -> Any:
    """Registra un gasto en un grupo con o sin encabezado Idempotency-Key."""
    encabezados = dict(auth)
    if clave_idempotencia is not None:
        encabezados["Idempotency-Key"] = clave_idempotencia
    return cliente.post(f"/grupos/{grupo_id}/gastos", json=cuerpo, headers=encabezados)


def leer_cambios_http(
    cliente: TestClient,
    auth: dict[str, str],
    desde: int | str = 0,
) -> Any:
    """Consulta GET /cambios?desde=<cursor> con el token del usuario."""
    return cliente.get(f"/cambios?desde={desde}", headers=auth)
