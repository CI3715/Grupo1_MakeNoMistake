"""``GET /cambios`` con el almacén en memoria (RF-COM-10, INV-13, INV-17).

El grupo «Viaje a Mérida» lo forman ana, beto y carla; dario está registrado
pero no pertenece. Las mismas reglas contra PostgreSQL real están en
``test_integracion_postgres.py``.
"""

from typing import Any
from uuid import UUID

import pytest
from apoyo_compartido import leer_cambios_http, registrar_gasto_idempotente_http
from fastapi.testclient import TestClient

from cambios import TAMANO_LOTE

Usuarios = dict[str, dict[str, Any]]
CAMPOS_DEL_EVENTO = {
    "id",
    "posicion",
    "tipo",
    "grupo",
    "gasto",
    "fecha",
    "moneda",
    "monto",
    "incluye_redondeo",
}


def cuerpo(
    concepto: str,
    monto: str,
    regla: str = "partes_iguales",
    participantes: tuple[tuple[str, str], ...] = (
        ("ana", ""),
        ("beto", ""),
        ("carla", ""),
    ),
    pagador: str = "ana",
    fecha: str = "2026-03-14",
) -> dict[str, Any]:
    return {
        "concepto": concepto,
        "monto": monto,
        "fecha": fecha,
        "pagador": pagador,
        "regla": regla,
        "participantes": [{"usuario": u, "valor": v} for u, v in participantes],
    }


def registrar(
    cliente: TestClient, grupo: str, auth: dict[str, str], datos: dict[str, Any]
) -> dict[str, Any]:
    resp = registrar_gasto_idempotente_http(cliente, grupo, auth, datos)
    assert resp.status_code == 201, resp.text
    respuesta: dict[str, Any] = resp.json()
    return respuesta


def cambios_de(
    cliente: TestClient, usuarios: Usuarios, quien: str, desde: int | str = 0
) -> dict[str, Any]:
    resp = leer_cambios_http(cliente, usuarios[quien]["auth"], desde)
    assert resp.status_code == 200, resp.text
    respuesta: dict[str, Any] = resp.json()
    return respuesta


# ------------------------------------------------------------------ sesión


def test_sin_token_responde_401(cliente: TestClient) -> None:
    resp = cliente.get("/cambios")
    assert resp.status_code == 401
    assert resp.headers["WWW-Authenticate"] == "Bearer"
    assert resp.json() == {"mensaje": "Sesión no válida o vencida."}


def test_token_invalido_responde_401_antes_que_desde_invalido(
    cliente: TestClient,
) -> None:
    resp = cliente.get(
        "/cambios?desde=-1", headers={"Authorization": "Bearer no-es-un-token"}
    )
    assert resp.status_code == 401


# --------------------------------------------------------------- cursor 400


@pytest.mark.parametrize(
    "desde", ["-1", "1.5", "abc", "", "9223372036854775808", "1e3", "%2B1"]
)
def test_desde_invalido_responde_400_y_no_422(
    cliente: TestClient, usuarios_flujo: Usuarios, desde: str
) -> None:
    resp = cliente.get(f"/cambios?desde={desde}", headers=usuarios_flujo["ana"]["auth"])
    assert resp.status_code == 400
    assert resp.json() == {
        "errores": [
            {
                "campo": "desde",
                "mensaje": "El cursor debe ser un número entero no negativo.",
            }
        ]
    }


def test_sin_desde_se_toma_cero_y_sin_eventos_el_lote_esta_vacio(
    cliente: TestClient, usuarios_flujo: Usuarios
) -> None:
    resp = cliente.get("/cambios", headers=usuarios_flujo["dario"]["auth"])
    assert resp.status_code == 200
    assert resp.json() == {"eventos": [], "cursor": 0, "hay_mas": False}


def test_el_desde_maximo_de_int64_es_valido(
    cliente: TestClient, usuarios_flujo: Usuarios
) -> None:
    datos = cambios_de(cliente, usuarios_flujo, "ana", "9223372036854775807")
    assert datos == {"eventos": [], "cursor": 9223372036854775807, "hay_mas": False}


# ------------------------------------------------------ eventos y privacidad


def test_cada_participante_recibe_su_parte_con_los_campos_acordados(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    gasto = registrar(
        cliente, grupo_merida, usuarios_flujo["ana"]["auth"], cuerpo("Hotel", "100.00")
    )
    partes = {
        "ana": ("33.34", True),
        "beto": ("33.33", False),
        "carla": ("33.33", False),
    }
    ids = set()
    for usuario, (monto, redondeo) in partes.items():
        datos = cambios_de(cliente, usuarios_flujo, usuario)
        assert (datos["cursor"], datos["hay_mas"]) == (1, False)
        (evento,) = datos["eventos"]
        assert set(evento) == CAMPOS_DEL_EVENTO
        assert evento["posicion"] == 1
        assert evento["tipo"] == "gasto_de_grupo"
        assert evento["grupo"] == {"id": grupo_merida, "nombre": "Viaje a Mérida"}
        assert evento["gasto"] == {"id": gasto["id"], "concepto": "Hotel"}
        assert (evento["fecha"], evento["moneda"]) == ("2026-03-14", "USD")
        assert (evento["monto"], evento["incluye_redondeo"]) == (monto, redondeo)
        UUID(evento["id"])
        ids.add(evento["id"])
    assert len(ids) == 3, "cada participante recibe su propio evento"


def test_el_excluido_no_recibe_evento_ni_consume_posicion(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    auth = usuarios_flujo["ana"]["auth"]
    registrar(cliente, grupo_merida, auth, cuerpo("Hotel", "100.00"))
    registrar(
        cliente,
        grupo_merida,
        auth,
        cuerpo("Taxi", "20.00", participantes=(("ana", ""), ("beto", ""))),
    )
    registrar(cliente, grupo_merida, auth, cuerpo("Museo", "30.00"))

    carla = cambios_de(cliente, usuarios_flujo, "carla")
    assert [e["gasto"]["concepto"] for e in carla["eventos"]] == ["Hotel", "Museo"]
    assert [e["posicion"] for e in carla["eventos"]] == [1, 2]
    beto = cambios_de(cliente, usuarios_flujo, "beto")
    assert [e["posicion"] for e in beto["eventos"]] == [1, 2, 3]


def test_quien_no_es_del_grupo_no_ve_nada(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    registrar(
        cliente, grupo_merida, usuarios_flujo["ana"]["auth"], cuerpo("Hotel", "100.00")
    )
    assert cambios_de(cliente, usuarios_flujo, "dario") == {
        "eventos": [],
        "cursor": 0,
        "hay_mas": False,
    }


def test_nadie_recibe_eventos_de_otros_ni_de_grupos_ajenos(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    """beto y carla tienen otro grupo sin ana: ana no ve nada de él (INV-17)."""
    from flujo_http import aceptar_grupo, crear_grupo, hacerse_amigos, invitar

    beto, carla = usuarios_flujo["beto"]["auth"], usuarios_flujo["carla"]["auth"]
    hacerse_amigos(cliente, beto, "carla", carla)
    otro = crear_grupo(cliente, beto, "Cena del viernes")
    invitar(cliente, otro, beto, "carla")
    aceptar_grupo(cliente, otro, carla)
    registrar(
        cliente,
        otro,
        beto,
        cuerpo(
            "Postre",
            "12.00",
            pagador="beto",
            participantes=(("beto", ""), ("carla", "")),
        ),
    )
    registrar(
        cliente, grupo_merida, usuarios_flujo["ana"]["auth"], cuerpo("Hotel", "9.00")
    )

    ana = cambios_de(cliente, usuarios_flujo, "ana")["eventos"]
    assert [e["grupo"]["id"] for e in ana] == [grupo_merida]
    carla_eventos = cambios_de(cliente, usuarios_flujo, "carla")["eventos"]
    assert {e["grupo"]["id"] for e in carla_eventos} == {otro, grupo_merida}
    # Cada evento es solo de quien consulta: ningún id se repite entre personas.
    ids_ana = {e["id"] for e in ana}
    assert ids_ana.isdisjoint(e["id"] for e in carla_eventos)


def test_la_respuesta_no_lleva_datos_del_libro_personal(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    """INV-16: el servidor solo entrega partes; no sabe de cuentas ni saldos."""
    registrar(
        cliente, grupo_merida, usuarios_flujo["ana"]["auth"], cuerpo("Hotel", "100.00")
    )
    resp = leer_cambios_http(cliente, usuarios_flujo["beto"]["auth"])
    texto = resp.text.lower()
    for palabra in ("cuenta", "saldo", "movimiento", "categoria", "tasa"):
        assert palabra not in texto


# ------------------------------------------------- orden, cursor y lotes


def test_pedir_otra_vez_desde_cero_devuelve_lo_mismo(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    """El registro solo crece: mismos eventos, mismos ids (INV-13)."""
    auth = usuarios_flujo["ana"]["auth"]
    for concepto in ("Hotel", "Cena", "Taxi"):
        registrar(cliente, grupo_merida, auth, cuerpo(concepto, "30.00"))
    primera = cambios_de(cliente, usuarios_flujo, "beto")
    segunda = cambios_de(cliente, usuarios_flujo, "beto")
    assert primera == segunda
    assert [e["posicion"] for e in primera["eventos"]] == [1, 2, 3]


def test_desde_el_cursor_solo_llega_lo_nuevo(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    auth = usuarios_flujo["ana"]["auth"]
    registrar(cliente, grupo_merida, auth, cuerpo("Hotel", "100.00"))
    cursor = cambios_de(cliente, usuarios_flujo, "carla")["cursor"]
    assert cambios_de(cliente, usuarios_flujo, "carla", cursor) == {
        "eventos": [],
        "cursor": cursor,
        "hay_mas": False,
    }
    registrar(cliente, grupo_merida, auth, cuerpo("Cena", "60.00"))
    nuevo = cambios_de(cliente, usuarios_flujo, "carla", cursor)
    assert [e["gasto"]["concepto"] for e in nuevo["eventos"]] == ["Cena"]
    assert nuevo["cursor"] == 2
    # Un cursor mayor que lo que existe no falla: simplemente no hay nada.
    assert cambios_de(cliente, usuarios_flujo, "carla", 50)["eventos"] == []


def test_lotes_de_cien_con_cursor_y_hay_mas(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    auth = usuarios_flujo["ana"]["auth"]
    solo_ana_y_beto = (("ana", ""), ("beto", ""))
    for numero in range(TAMANO_LOTE + 5):
        registrar(
            cliente,
            grupo_merida,
            auth,
            cuerpo(f"Gasto {numero}", "2.00", participantes=solo_ana_y_beto),
        )

    primero = cambios_de(cliente, usuarios_flujo, "beto")
    assert len(primero["eventos"]) == TAMANO_LOTE
    assert (primero["cursor"], primero["hay_mas"]) == (TAMANO_LOTE, True)
    posiciones = [e["posicion"] for e in primero["eventos"]]
    assert posiciones == list(range(1, TAMANO_LOTE + 1))

    segundo = cambios_de(cliente, usuarios_flujo, "beto", primero["cursor"])
    assert [e["posicion"] for e in segundo["eventos"]] == list(
        range(TAMANO_LOTE + 1, TAMANO_LOTE + 6)
    )
    assert (segundo["cursor"], segundo["hay_mas"]) == (TAMANO_LOTE + 5, False)
    assert cambios_de(cliente, usuarios_flujo, "carla")["eventos"] == []
