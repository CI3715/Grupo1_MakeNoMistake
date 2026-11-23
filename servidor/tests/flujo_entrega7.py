"""Flujo completo de la Entrega 7 hecho solo con HTTP, como lo haría la app.

Cuatro personas recién registradas (ana, beto, carla y dario). ana crea «Viaje
a Mérida» con beto y carla; dario queda fuera. Se registran los gastos de la
sección 2.6 con clave de idempotencia, se pierde una respuesta y se reintenta,
y cada persona sincroniza con ``GET /cambios`` varias veces y desde cero.

Lo usan ``test_flujo_entrega7.py`` (almacén en memoria) y
``test_integracion_postgres.py`` (PostgreSQL real), para que el mismo recorrido
se compruebe en los dos (RF-COM-10, RNF-17, INV-13, INV-15, INV-16, INV-17,
ESC-09).
"""

from typing import Any

from fastapi.testclient import TestClient
from flujo_http import gasto, grupo_de_tres

PALABRAS_DEL_LIBRO = ("cuenta", "saldo", "movimiento", "categoria", "tasa")

# Lo que cada persona debe recibir, en orden: (concepto, monto, redondeo).
PARTES_ESPERADAS = {
    "ana": [
        ("Hotel", "33.34", True),
        ("Cena", "30.00", False),
        ("Taxi", "10.00", False),
    ],
    "beto": [
        ("Hotel", "33.33", False),
        ("Cena", "15.00", False),
        ("Taxi", "10.00", False),
    ],
    "carla": [("Hotel", "33.33", False), ("Cena", "15.00", False)],
    "dario": [],
}


def sincronizar(
    cliente: TestClient, auth: dict[str, str], desde: int = 0
) -> tuple[list[dict[str, Any]], int]:
    """Pide lotes hasta que ``hay_mas`` es falso, como la app."""
    eventos: list[dict[str, Any]] = []
    cursor = desde
    while True:
        resp = cliente.get(f"/cambios?desde={cursor}", headers=auth)
        assert resp.status_code == 200, resp.text
        assert not any(p in resp.text.lower() for p in PALABRAS_DEL_LIBRO)
        datos = resp.json()
        eventos += datos["eventos"]
        assert datos["cursor"] >= cursor
        cursor = datos["cursor"]
        if not datos["hay_mas"]:
            return eventos, cursor


def recorrer_flujo_entrega_7(cliente: TestClient) -> dict[str, list[dict[str, Any]]]:
    """Recorre el flujo y devuelve los eventos que recibió cada persona."""
    grupo_id, auth = grupo_de_tres(cliente)
    url = f"/grupos/{grupo_id}/gastos"

    def enviar(quien: str, clave: str, cuerpo: dict[str, Any]) -> Any:
        return cliente.post(
            url, json=cuerpo, headers={**auth[quien], "Idempotency-Key": clave}
        )

    hotel = gasto(
        "100.00",
        "ana",
        "partes_iguales",
        [("ana", ""), ("beto", ""), ("carla", "")],
        concepto="Hotel",
        fecha="2026-03-14",
    )
    assert enviar("ana", "hotel-6f1c2a9e", hotel).status_code == 201

    # Primera sincronización de todos: cada uno recibe su parte del hotel.
    for quien in ("ana", "beto", "carla"):
        eventos, cursor = sincronizar(cliente, auth[quien])
        assert [e["gasto"]["concepto"] for e in eventos] == ["Hotel"]
        assert cursor == 1

    # beto registra la cena; la respuesta «se pierde» y la cola la reintenta.
    cena = gasto(
        "60.00",
        "beto",
        "porcentajes",
        [("ana", "50"), ("beto", "25"), ("carla", "25")],
        concepto="Cena",
        fecha="2026-03-15",
    )
    original = enviar("beto", "cena-0b7d4e21", cena)
    assert original.status_code == 201, original.text
    for _ in range(2):
        reintento = enviar("beto", "cena-0b7d4e21", cena)
        assert reintento.status_code == 200, reintento.text
        assert reintento.json() == original.json()
    # Un error de la app que reusa la clave con otro gasto se rechaza.
    otro = enviar("beto", "cena-0b7d4e21", {**cena, "monto": "61.00"})
    assert otro.status_code == 409

    taxi = gasto(
        "20.00",
        "ana",
        "partes_iguales",
        [("ana", ""), ("beto", "")],
        concepto="Taxi",
        fecha="2026-03-15",
    )
    assert enviar("ana", "taxi-93ac55d0", taxi).status_code == 201

    detalle = cliente.get(f"/grupos/{grupo_id}", headers=auth["ana"]).json()
    assert sorted(g["concepto"] for g in detalle["gastos"]) == ["Cena", "Hotel", "Taxi"]

    recibidos: dict[str, list[dict[str, Any]]] = {}
    for quien, esperadas in PARTES_ESPERADAS.items():
        # Sincronizar desde el cursor de antes: solo llega lo nuevo.
        cursor_previo = 1 if quien != "dario" else 0
        nuevos, cursor = sincronizar(cliente, auth[quien], cursor_previo)
        assert [e["gasto"]["concepto"] for e in nuevos] == [
            c for c, _, _ in esperadas[cursor_previo:]
        ]
        # Otra vez, y luego desde cero: nada nuevo y los mismos ids (INV-13).
        assert sincronizar(cliente, auth[quien], cursor) == ([], cursor)
        todos, cursor_cero = sincronizar(cliente, auth[quien])
        assert cursor_cero == cursor == len(esperadas)
        assert [
            (e["gasto"]["concepto"], e["monto"], e["incluye_redondeo"]) for e in todos
        ] == esperadas
        assert [e["posicion"] for e in todos] == list(range(1, len(esperadas) + 1))
        assert all(
            e["grupo"] == {"id": grupo_id, "nombre": "Viaje a Mérida"} for e in todos
        )
        assert all(
            e["moneda"] == "USD" and e["tipo"] == "gasto_de_grupo" for e in todos
        )
        recibidos[quien] = todos

    # Cada evento pertenece a una sola persona (INV-15, INV-17).
    ids = [e["id"] for eventos in recibidos.values() for e in eventos]
    assert len(ids) == len(set(ids)) == 8
    return recibidos
