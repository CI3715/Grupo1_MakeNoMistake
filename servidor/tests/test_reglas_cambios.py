"""Reglas puras del registro de cambios (``cambios.py``, ADR-015).

Sin base de datos ni HTTP: un evento por participante, posición creciente por
usuario, el excluido no recibe nada ni consume posición, el cursor y los lotes.
Al final, los gastos de ``docs/vectores-sincronizacion.json`` se recorren con
``division.dividir`` y ``cambios.eventos_de_gasto`` y cada evento debe coincidir
campo a campo con ``gastos[].esperado.eventos`` (RF-COM-10, INV-17).
"""

import json
from collections.abc import Callable, Iterator
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import Any
from uuid import UUID, uuid4

import pytest
from hypothesis import given
from hypothesis import strategies as st

from cambios import (
    MAXIMO_CURSOR,
    MENSAJE_DESDE,
    TAMANO_LOTE,
    TIPO_GASTO_DE_GRUPO,
    EventoCambio,
    Lote,
    ParteParaEvento,
    armar_lote,
    evento_a_json,
    eventos_de_gasto,
    leer_desde,
    lote_a_json,
)
from division import Participante, dividir

GRUPO = UUID("600f76c8-e198-5d6f-82b8-ffb04cc3fc13")
GASTO = UUID("ff6edddc-3a5d-55a2-b39c-c50a58649d1f")
RUTA_VECTORES = (
    Path(__file__).resolve().parents[2] / "docs" / "vectores-sincronizacion.json"
)


def ids_fijos(*ids: str) -> Callable[[], UUID]:
    pendientes: Iterator[str] = iter(ids)
    return lambda: UUID(next(pendientes))


def hotel(
    partes: list[ParteParaEvento[str]],
    ultimas: dict[str, int] | None = None,
) -> list[EventoCambio[str]]:
    return eventos_de_gasto(
        gasto_id=GASTO,
        concepto="Hotel",
        fecha=date(2026, 3, 14),
        grupo_id=GRUPO,
        grupo_nombre="Viaje a Mérida",
        moneda="USD",
        partes=partes,
        ultimas_posiciones=ultimas or {},
        nuevo_id=uuid4,
    )


def parte(usuario: str, monto: str, redondeo: bool = False) -> ParteParaEvento[str]:
    return ParteParaEvento(usuario, Decimal(monto), redondeo)


def evento(posicion: int) -> EventoCambio[str]:
    return EventoCambio(
        id=uuid4(),
        usuario="ana",
        posicion=posicion,
        grupo_id=GRUPO,
        grupo_nombre="Viaje a Mérida",
        gasto_id=uuid4(),
        concepto="Hotel",
        fecha=date(2026, 3, 14),
        moneda="USD",
        monto=Decimal("1.00"),
        incluye_redondeo=False,
    )


# ---------------------------------------------------- un evento por participante


def test_cada_participante_recibe_un_evento_con_su_parte() -> None:
    eventos = hotel(
        [parte("ana", "33.34", True), parte("beto", "33.33"), parte("carla", "33.33")]
    )
    assert [(e.usuario, e.monto, e.incluye_redondeo) for e in eventos] == [
        ("ana", Decimal("33.34"), True),
        ("beto", Decimal("33.33"), False),
        ("carla", Decimal("33.33"), False),
    ]
    assert all(e.gasto_id == GASTO and e.grupo_id == GRUPO for e in eventos)
    assert all(e.moneda == "USD" and e.concepto == "Hotel" for e in eventos)
    assert len({e.id for e in eventos}) == 3


def test_la_posicion_sigue_a_la_ultima_de_cada_usuario() -> None:
    eventos = hotel([parte("ana", "50.00"), parte("beto", "50.00")], ultimas={"ana": 7})
    assert {e.usuario: e.posicion for e in eventos} == {"ana": 8, "beto": 1}


def test_quien_no_esta_en_las_partes_no_recibe_evento_ni_consume_posicion() -> None:
    primero = hotel([parte("ana", "60.00"), parte("beto", "40.00")])
    ultimas = {e.usuario: e.posicion for e in primero}
    segundo = hotel([parte("ana", "10.00"), parte("carla", "10.00")], ultimas)
    assert {e.usuario: e.posicion for e in segundo} == {"ana": 2, "carla": 1}
    assert "beto" not in {e.usuario for e in segundo}


def test_una_parte_de_cero_no_genera_evento() -> None:
    """0,02 entre tres: la tercera parte es 0,00 y el libro no admite egresos de 0."""
    division = dividir(
        "0.02", "partes_iguales", [Participante(u) for u in ("ana", "beto", "carla")]
    )
    eventos = hotel(
        [
            ParteParaEvento(p.usuario, Decimal(p.monto), p.incluye_redondeo)
            for p in division.partes
        ]
    )
    assert [e.usuario for e in eventos] == ["ana", "beto"]


@pytest.mark.parametrize(
    "partes",
    [
        [parte("ana", "1.00"), parte("ana", "2.00")],
        [parte("ana", "1.001")],
        [parte("ana", "-1.00")],
    ],
    ids=["repetida", "tres-decimales", "negativa"],
)
def test_partes_imposibles_se_rechazan(partes: list[ParteParaEvento[str]]) -> None:
    with pytest.raises(ValueError):
        hotel(partes)


def test_el_json_del_evento_trae_exactamente_los_campos_acordados() -> None:
    (unico,) = eventos_de_gasto(
        gasto_id=GASTO,
        concepto="Hotel",
        fecha=date(2026, 3, 14),
        grupo_id=GRUPO,
        grupo_nombre="Viaje a Mérida",
        moneda="USD",
        partes=[parte("ana", "33.3", True)],
        ultimas_posiciones={},
        nuevo_id=ids_fijos("f16b54ef-4fe0-55cc-80f6-2da2cad68c97"),
    )
    assert evento_a_json(unico) == {
        "id": "f16b54ef-4fe0-55cc-80f6-2da2cad68c97",
        "posicion": 1,
        "tipo": TIPO_GASTO_DE_GRUPO,
        "grupo": {"id": str(GRUPO), "nombre": "Viaje a Mérida"},
        "gasto": {"id": str(GASTO), "concepto": "Hotel"},
        "fecha": "2026-03-14",
        "moneda": "USD",
        "monto": "33.30",
        "incluye_redondeo": True,
    }


def test_nada_del_libro_personal_viaja_en_el_evento() -> None:
    """INV-16: ni cuenta, ni saldo, ni movimiento, ni categoría."""
    (unico,) = hotel([parte("ana", "10.00")])
    texto = json.dumps(evento_a_json(unico)).lower()
    for palabra in ("cuenta", "saldo", "movimiento", "categoria", "tasa"):
        assert palabra not in texto


# ------------------------------------------------------------------- cursor


@pytest.mark.parametrize(
    ("texto", "esperado"),
    [(None, 0), ("0", 0), ("15", 15), ("007", 7), (str(MAXIMO_CURSOR), MAXIMO_CURSOR)],
)
def test_desde_valido(texto: str | None, esperado: int) -> None:
    assert leer_desde(texto) == (esperado, None)


@pytest.mark.parametrize(
    "texto",
    ["", "-1", "+1", "1.0", "1e3", " 1", "uno", "١٢", str(MAXIMO_CURSOR + 1)],
)
def test_desde_invalido_es_error_del_campo(texto: str) -> None:
    valor, error = leer_desde(texto)
    assert valor is None
    assert error is not None
    assert (error.campo, error.mensaje) == ("desde", MENSAJE_DESDE)


# -------------------------------------------------------------------- lotes


def test_lote_vacio_conserva_el_cursor() -> None:
    lote: Lote[str] = armar_lote([], desde=12)
    assert lote_a_json(lote) == {"eventos": [], "cursor": 12, "hay_mas": False}


def test_lote_completo_sin_mas() -> None:
    lote = armar_lote([evento(p) for p in (3, 1, 2)], desde=0)
    assert [e.posicion for e in lote.eventos] == [1, 2, 3]
    assert (lote.cursor, lote.hay_mas) == (3, False)


def test_con_mas_de_cien_se_entregan_cien_y_hay_mas() -> None:
    posteriores = [evento(p) for p in range(1, TAMANO_LOTE + 2)]
    lote = armar_lote(posteriores, desde=0)
    assert len(lote.eventos) == TAMANO_LOTE
    assert (lote.cursor, lote.hay_mas) == (TAMANO_LOTE, True)


def test_un_evento_anterior_al_cursor_es_un_error() -> None:
    with pytest.raises(ValueError):
        armar_lote([evento(3)], desde=3)


@given(
    total=st.integers(min_value=0, max_value=250),
    tamano=st.integers(min_value=1, max_value=60),
)
def test_recorrer_por_lotes_entrega_todo_una_vez_y_en_orden(
    total: int, tamano: int
) -> None:
    registro = [evento(p) for p in range(1, total + 1)]
    recibidos: list[int] = []
    cursor = 0
    while True:
        posteriores = [e for e in registro if e.posicion > cursor][: tamano + 1]
        lote = armar_lote(posteriores, cursor, tamano)
        recibidos += [e.posicion for e in lote.eventos]
        assert lote.cursor >= cursor
        cursor = lote.cursor
        if not lote.hay_mas:
            break
    assert recibidos == list(range(1, total + 1))
    assert cursor == total


# --------------------------------------------- vectores compartidos (Katherine)


def cargar_vectores() -> dict[str, Any]:
    if not RUTA_VECTORES.exists():
        pytest.skip("docs/vectores-sincronizacion.json todavía no está en la rama")
    datos: dict[str, Any] = json.loads(RUTA_VECTORES.read_text(encoding="utf-8"))
    return datos


def test_los_eventos_coinciden_con_los_vectores() -> None:
    """Cada gasto, en orden, produce exactamente los eventos del archivo."""
    ultimas: dict[str, int] = {}
    for gasto in cargar_vectores()["gastos"]:
        esperados = gasto["esperado"]["eventos"]
        division = dividir(
            gasto["monto"],
            gasto["regla"],
            [Participante(p["usuario"], p["valor"]) for p in gasto["participantes"]],
        )
        eventos = eventos_de_gasto(
            gasto_id=UUID(gasto["gasto_id"]),
            concepto=gasto["concepto"],
            fecha=date.fromisoformat(gasto["fecha"]),
            grupo_id=UUID(gasto["grupo"]["id"]),
            grupo_nombre=gasto["grupo"]["nombre"],
            moneda=gasto["moneda"],
            partes=[
                ParteParaEvento(p.usuario, Decimal(p.monto), p.incluye_redondeo)
                for p in division.partes
            ],
            ultimas_posiciones=ultimas,
            # El id lo genera el servidor; aquí se inyectan los del archivo.
            nuevo_id=ids_fijos(*(f["evento"]["id"] for f in esperados)),
        )
        assert [
            {"usuario": e.usuario, "evento": evento_a_json(e)} for e in eventos
        ] == esperados, gasto["id"]
        sin_evento = set(gasto["esperado"]["sin_evento"])
        assert sin_evento.isdisjoint(e.usuario for e in eventos)
        ultimas.update({e.usuario: e.posicion for e in eventos})
