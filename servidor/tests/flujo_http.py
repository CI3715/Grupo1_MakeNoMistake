"""Pasos del flujo social hechos solo con HTTP, como lo haría la app.

Los usan las pruebas con el almacén en memoria y las de PostgreSQL real, para
que el mismo recorrido (registrarse, hacerse amigos, crear el grupo, invitar,
aceptar y registrar gastos) se compruebe en los dos.
"""

from typing import Any

from fastapi.testclient import TestClient

CONTRASENA = "Password123!"


def nueva_persona(cliente: TestClient, usuario: str) -> dict[str, str]:
    """Registra a alguien desde cero, inicia sesión y devuelve su encabezado."""
    alta = cliente.post(
        "/usuarios",
        json={
            "nombre_usuario": usuario,
            "nombre_visible": usuario.title(),
            "correo": f"{usuario}@ejemplo.com",
            "contrasena": CONTRASENA,
        },
    )
    assert alta.status_code == 201, alta.text
    sesion = cliente.post(
        "/sesiones", json={"correo": f"{usuario}@ejemplo.com", "contrasena": CONTRASENA}
    )
    assert sesion.status_code == 200, sesion.text
    return {"Authorization": f"Bearer {sesion.json()['token']}"}


def hacerse_amigos(
    cliente: TestClient,
    quien_invita: dict[str, str],
    invitado: str,
    auth_invitado: dict[str, str],
) -> None:
    solicitud = cliente.post(
        "/amistades", json={"usuario": invitado}, headers=quien_invita
    )
    assert solicitud.status_code == 201, solicitud.text
    respuesta = cliente.post(
        f"/amistades/{solicitud.json()['id']}/respuesta",
        json={"aceptar": True},
        headers=auth_invitado,
    )
    assert respuesta.status_code == 200, respuesta.text


def crear_grupo(cliente: TestClient, auth: dict[str, str], nombre: str) -> str:
    resp = cliente.post(
        "/grupos",
        json={"nombre": nombre, "moneda": "USD", "actividad": "marzo"},
        headers=auth,
    )
    assert resp.status_code == 201, resp.text
    return str(resp.json()["id"])


def invitar(
    cliente: TestClient, grupo_id: str, admin: dict[str, str], usuario: str
) -> None:
    resp = cliente.post(
        f"/grupos/{grupo_id}/miembros", json={"usuario": usuario}, headers=admin
    )
    assert resp.status_code == 201, resp.text


def aceptar_grupo(cliente: TestClient, grupo_id: str, auth: dict[str, str]) -> None:
    resp = cliente.post(
        f"/grupos/{grupo_id}/miembros/yo/respuesta",
        json={"respuesta": "aceptar"},
        headers=auth,
    )
    assert resp.status_code == 200, resp.text


def gasto(
    monto: str,
    pagador: str,
    regla: str,
    participantes: list[tuple[str, str]],
    concepto: str = "Cena",
    fecha: str = "2026-11-12",
) -> dict[str, Any]:
    """Cuerpo de POST /grupos/{id}/gastos con la forma de ``NuevoGastoGrupoDto``."""
    return {
        "concepto": concepto,
        "monto": monto,
        "fecha": fecha,
        "pagador": pagador,
        "regla": regla,
        "participantes": [{"usuario": u, "valor": v} for u, v in participantes],
    }


def grupo_de_tres(cliente: TestClient) -> tuple[str, dict[str, dict[str, str]]]:
    """ana crea el grupo e invita a beto y carla, que aceptan. dario queda fuera.

    Todo pasa por las rutas, partiendo de cuatro personas recién registradas.
    """
    auth = {u: nueva_persona(cliente, u) for u in ("ana", "beto", "carla", "dario")}
    hacerse_amigos(cliente, auth["ana"], "beto", auth["beto"])
    hacerse_amigos(cliente, auth["ana"], "carla", auth["carla"])
    grupo_id = crear_grupo(cliente, auth["ana"], "Viaje a Mérida")
    for usuario in ("beto", "carla"):
        invitar(cliente, grupo_id, auth["ana"], usuario)
        aceptar_grupo(cliente, grupo_id, auth[usuario])
    return grupo_id, auth


def registrar_ejemplo_2_5(
    cliente: TestClient, grupo_id: str, auth: dict[str, dict[str, str]]
) -> None:
    """Enunciado 2.5: ana paga 100,00 a partes iguales y beto 60,00 al 50/25/25."""
    cena = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json=gasto(
            "100.00",
            "ana",
            "partes_iguales",
            [("ana", ""), ("beto", ""), ("carla", "")],
            concepto="Cena",
            fecha="2026-03-10",
        ),
        headers=auth["ana"],
    )
    assert cena.status_code == 201, cena.text
    taxi = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json=gasto(
            "60.00",
            "beto",
            "porcentajes",
            [("ana", "50"), ("beto", "25"), ("carla", "25")],
            concepto="Taxi",
            fecha="2026-03-11",
        ),
        headers=auth["beto"],
    )
    assert taxi.status_code == 201, taxi.text


BALANCES_2_5 = [
    {"usuario": "ana", "pago": "100.00", "le_corresponde": "63.34", "balance": "36.66"},
    {"usuario": "beto", "pago": "60.00", "le_corresponde": "48.33", "balance": "11.67"},
    {
        "usuario": "carla",
        "pago": "0.00",
        "le_corresponde": "48.33",
        "balance": "-48.33",
    },
]
