"""Comprueba que ``docs/vectores-sincronizacion.json`` es coherente.

El archivo fija qué eventos recibe cada participante de un gasto de grupo y cómo
los aplica el libro personal sin duplicar (RF-COM-10, RNF-17, INV-13, INV-15,
INV-17, INV-18, ESC-05). Lo leen estas pruebas y las de la app, en
``app/src-tauri/tests/vectores_sincronizacion.rs``.

Las primeras pruebas revisan el archivo contra las reglas del enunciado; las
últimas recorren los gastos con ``division.dividir`` del servidor, de modo que el
monto y la marca de redondeo de cada evento coincidan con lo que calcula el
servidor. Los escenarios del libro los reproduce un modelo de referencia
mínimo, escrito aquí solo para comprobar que el archivo no se contradice: el
núcleo de la app tiene su propia prueba sobre los mismos escenarios.
"""

import json
import uuid
from collections.abc import Iterator
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

from division import Participante, dividir

RUTA = Path(__file__).resolve().parents[2] / "docs" / "vectores-sincronizacion.json"
CENTIMO = Decimal("0.01")
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
# Palabras que nunca deben aparecer como clave de lo que viaja al servidor.
CLAVES_DEL_LIBRO = ("cuenta", "saldo", "movimiento", "tasa", "categoria")


def cargar() -> dict[str, Any]:
    datos: dict[str, Any] = json.loads(RUTA.read_text(encoding="utf-8"))
    return datos


VECTORES = cargar()
GASTOS: list[dict[str, Any]] = VECTORES["gastos"]
ESCENARIOS: list[dict[str, Any]] = VECTORES["escenarios"]
EVENTOS_DE_GASTOS: list[tuple[str, dict[str, Any]]] = [
    (g["id"], f["evento"]) for g in GASTOS for f in g["esperado"]["eventos"]
]


def dinero(texto: str) -> Decimal:
    """Un monto del archivo: texto con punto y exactamente dos decimales."""
    assert isinstance(texto, str), "los montos viajan como texto, nunca como número"
    numero = Decimal(texto)
    assert numero == numero.quantize(CENTIMO)
    assert texto.count(".") == 1 and len(texto.split(".")[1]) == 2
    return numero


def claves(valor: Any) -> Iterator[str]:
    """Todas las claves de un JSON, a cualquier profundidad."""
    if isinstance(valor, dict):
        for clave, hijo in valor.items():
            yield str(clave)
            yield from claves(hijo)
    elif isinstance(valor, list):
        for hijo in valor:
            yield from claves(hijo)


def respuestas_del_servidor() -> list[dict[str, Any]]:
    return [
        p["respuesta"]
        for e in ESCENARIOS
        for p in e["pasos"]
        if p["accion"] == "sincronizar"
    ]


# ---------- el archivo contra el enunciado ----------


def test_el_archivo_trae_el_ejemplo_y_los_casos_pedidos() -> None:
    assert {"hotel", "cena", "taxi", "centimos-sin-evento"} <= {g["id"] for g in GASTOS}
    assert {
        "ejemplo-2-6",
        "lote-repetido",
        "tipo-desconocido",
        "parte-sin-cuenta",
        "parte-con-redondeo",
    } <= {e["id"] for e in ESCENARIOS}


def test_los_identificadores_no_se_repiten() -> None:
    ids = [g["id"] for g in GASTOS] + [e["id"] for e in ESCENARIOS]
    assert len(ids) == len(set(ids))
    ids_de_evento = [evento["id"] for _, evento in EVENTOS_DE_GASTOS]
    assert len(ids_de_evento) == len(set(ids_de_evento))


@pytest.mark.parametrize(
    "evento", [e for _, e in EVENTOS_DE_GASTOS], ids=lambda e: e["id"][:8]
)
def test_cada_evento_trae_exactamente_los_campos_acordados(
    evento: dict[str, Any],
) -> None:
    assert set(evento) == CAMPOS_DEL_EVENTO
    assert evento["tipo"] == "gasto_de_grupo"
    assert str(uuid.UUID(evento["id"])) == evento["id"]
    assert str(uuid.UUID(evento["gasto"]["id"])) == evento["gasto"]["id"]
    assert str(uuid.UUID(evento["grupo"]["id"])) == evento["grupo"]["id"]
    assert set(evento["grupo"]) == {"id", "nombre"}
    assert set(evento["gasto"]) == {"id", "concepto"}
    assert isinstance(evento["posicion"], int) and evento["posicion"] >= 1
    assert isinstance(evento["incluye_redondeo"], bool)
    assert date.fromisoformat(evento["fecha"]).isoformat() == evento["fecha"]
    dinero(evento["monto"])


def test_las_posiciones_son_crecientes_por_usuario() -> None:
    por_usuario: dict[str, list[int]] = {}
    for gasto in GASTOS:
        for fila in gasto["esperado"]["eventos"]:
            por_usuario.setdefault(fila["usuario"], []).append(
                fila["evento"]["posicion"]
            )
    for usuario, posiciones in por_usuario.items():
        # Un excluido o parte 0,00 no consume posición: las que recibe son 1, 2, 3...
        assert posiciones == list(range(1, len(posiciones) + 1)), usuario


def test_solo_reciben_evento_los_participantes() -> None:
    miembros = {"ana", "beto", "carla"}
    for gasto in GASTOS:
        division = dividir(
            gasto["monto"],
            gasto["regla"],
            [Participante(p["usuario"], p["valor"]) for p in gasto["participantes"]],
        )
        con_parte_positiva = {
            p.usuario for p in division.partes if dinero(p.monto) > Decimal("0.00")
        }
        con_evento = {f["usuario"] for f in gasto["esperado"]["eventos"]}
        assert con_evento == con_parte_positiva, gasto["id"]
        assert set(gasto["esperado"]["sin_evento"]) == miembros - con_parte_positiva
    taxi = next(g for g in GASTOS if g["id"] == "taxi")
    assert taxi["esperado"]["sin_evento"] == ["carla"]
    centimos = next(g for g in GASTOS if g["id"] == "centimos-sin-evento")
    assert centimos["esperado"]["sin_evento"] == ["carla"]


def test_nada_del_libro_viaja_al_servidor() -> None:
    """INV-16, RES-02: ni los eventos ni las respuestas llevan datos del libro."""
    enviado = [g["esperado"] for g in GASTOS] + respuestas_del_servidor()
    for clave in claves(enviado):
        assert not any(palabra in clave.lower() for palabra in CLAVES_DEL_LIBRO), clave


def test_los_tipos_desconocidos_traen_id_posicion_y_tipo() -> None:
    desconocidos = [
        e
        for r in respuestas_del_servidor()
        for e in r["eventos"]
        if e["tipo"] != "gasto_de_grupo"
    ]
    assert len(desconocidos) >= 2
    for evento in desconocidos:
        assert {"id", "posicion", "tipo"} <= set(evento)


# ---------- el servidor calcula lo mismo que el archivo ----------


@pytest.mark.parametrize("gasto", GASTOS, ids=lambda g: g["id"])
def test_el_servidor_divide_como_el_archivo(gasto: dict[str, Any]) -> None:
    division = dividir(
        gasto["monto"],
        gasto["regla"],
        [Participante(p["usuario"], p["valor"]) for p in gasto["participantes"]],
    )
    esperadas = gasto["esperado"]["eventos"]
    partes_con_evento = [
        p for p in division.partes if dinero(p.monto) > Decimal("0.00")
    ]
    partes_en_cero = [
        p.usuario for p in division.partes if dinero(p.monto) == Decimal("0.00")
    ]
    for usuario_cero in partes_en_cero:
        assert usuario_cero in gasto["esperado"]["sin_evento"]
    assert [p.usuario for p in partes_con_evento] == [f["usuario"] for f in esperadas]
    for parte, fila in zip(partes_con_evento, esperadas, strict=True):
        assert parte.monto == fila["evento"]["monto"], parte.usuario
        assert parte.incluye_redondeo == fila["evento"]["incluye_redondeo"]
    assert sum(dinero(p.monto) for p in division.partes) == dinero(gasto["monto"])


def test_el_evento_trae_los_datos_del_gasto() -> None:
    for gasto in GASTOS:
        for fila in gasto["esperado"]["eventos"]:
            evento = fila["evento"]
            assert evento["gasto"] == {
                "id": gasto["gasto_id"],
                "concepto": gasto["concepto"],
            }
            assert evento["grupo"] == gasto["grupo"]
            assert evento["fecha"] == gasto["fecha"]
            assert evento["moneda"] == gasto["moneda"]


# ---------- los escenarios del libro ----------


class Libro:
    """Modelo de referencia: solo comprueba que los escenarios no se contradicen."""

    def __init__(self, escenario: dict[str, Any]) -> None:
        self.saldos = {
            c["nombre"]: int(dinero(c["saldo_inicial"]) * 100)
            for c in escenario["cuentas"]
        }
        self.elegida: str | None = None
        self.cursor = 0
        self.cola = 0
        self.movimientos: list[dict[str, Any]] = []
        self.pendientes: list[dict[str, Any]] = []

    def aplicar(self, evento: dict[str, Any]) -> None:
        assert self.elegida is not None
        self.saldos[self.elegida] -= int(dinero(evento["monto"]) * 100)
        self.movimientos.append(evento)

    def ya_conocido(self, evento_id: str) -> bool:
        ids = [m["id"] for m in self.movimientos + self.pendientes]
        return evento_id in ids

    def sincronizar(self, respuesta: dict[str, Any]) -> tuple[int, int]:
        aplicados = ignorados = 0
        for evento in respuesta["eventos"]:
            if evento["tipo"] != "gasto_de_grupo" or self.ya_conocido(evento["id"]):
                ignorados += 1
                continue
            if self.elegida is None:
                self.pendientes.append(evento)
            else:
                self.aplicar(evento)
            aplicados += 1
        self.cursor = respuesta["cursor"]
        return aplicados, ignorados

    def elegir(self, cuenta: str) -> None:
        self.elegida = cuenta
        for evento in sorted(self.pendientes, key=lambda e: e["posicion"]):
            self.aplicar(evento)
        self.pendientes = []

    def estado(self) -> dict[str, Any]:
        return {
            "cursor": self.cursor,
            "cola": self.cola,
            "saldos": {k: f"{v // 100}.{v % 100:02d}" for k, v in self.saldos.items()},
            "movimientos": [m["id"] for m in self.movimientos],
            "pendientes": [p["id"] for p in self.pendientes],
        }


@pytest.mark.parametrize("escenario", ESCENARIOS, ids=lambda e: e["id"])
def test_los_pasos_de_cada_escenario_dan_el_estado_esperado(
    escenario: dict[str, Any],
) -> None:
    libro = Libro(escenario)
    for paso in escenario["pasos"]:
        esperado = dict(paso["esperado"])
        accion = paso["accion"]
        if accion == "elegir_cuenta":
            libro.elegir(paso["cuenta"])
        elif accion == "sincronizar":
            respuesta = paso["respuesta"]
            posiciones = [e["posicion"] for e in respuesta["eventos"]]
            # El cursor es la posición del último evento, o «desde» si no hay nada.
            assert respuesta["cursor"] == max(posiciones, default=paso["desde"])
            assert paso["desde"] == libro.cursor
            assert respuesta["hay_mas"] is False
            aplicados, ignorados = libro.sincronizar(respuesta)
            assert (aplicados, ignorados) == (
                esperado.pop("aplicados"),
                esperado.pop("ignorados"),
            ), paso["id"]
        elif accion == "reiniciar_cursor":
            libro.cursor = 0
        elif accion == "encolar_gasto":
            libro.cola += 1
        elif accion == "enviar_cola":
            libro.cola -= 1
        else:
            pytest.fail(f"acción desconocida: {accion}")
        assert libro.estado() == esperado, f"{escenario['id']} · {paso['id']}"

    detalle = escenario["movimientos_finales"]
    assert [m["evento_id"] for m in detalle] == [m["id"] for m in libro.movimientos]
    for movimiento, evento in zip(detalle, libro.movimientos, strict=True):
        assert movimiento["concepto"] == (
            f"{evento['grupo']['nombre']} · {evento['gasto']['concepto']}"
        )
        assert movimiento["tipo"] == "egreso"
        assert movimiento["categoria"] == "Otros gastos"
        assert movimiento["de_grupo"] is True
        assert movimiento["monto"] == evento["monto"]
        assert movimiento["fecha"] == evento["fecha"]


def test_el_ejemplo_de_la_seccion_2_6_da_los_saldos_del_enunciado() -> None:
    ejemplo = next(e for e in ESCENARIOS if e["id"] == "ejemplo-2-6")
    saldos = [p["esperado"]["saldos"]["Efectivo USD"] for p in ejemplo["pasos"]]
    assert saldos == [
        "200.00",  # elige la cuenta
        "166.67",  # sincronización 1: el hotel
        "166.67",  # sincronizaciones 2 y 3
        "166.67",
        "166.67",  # sin conexión: la cena queda pendiente de envío
        "166.67",  # vuelve la conexión: se envió, aún no bajó
        "151.67",  # baja la parte de la cena
        "151.67",  # cursor reiniciado a cero
        "151.67",  # bajan de nuevo los dos eventos y se ignoran
    ]
    colas = [p["esperado"]["cola"] for p in ejemplo["pasos"]]
    assert colas == [0, 0, 0, 0, 1, 0, 0, 0, 0]
    assert ejemplo["pasos"][-1]["esperado"]["ignorados"] == 2


def test_solo_se_ofrecen_cuentas_activas_en_la_moneda_del_grupo() -> None:
    """INV-18."""
    for escenario in ESCENARIOS:
        ofrecidas = [
            c["nombre"]
            for c in escenario["cuentas"]
            if c["activa"] and c["moneda"] == escenario["moneda"]
        ]
        if "cuentas_ofrecidas" in escenario:
            assert escenario["cuentas_ofrecidas"] == ofrecidas
        for paso in escenario["pasos"]:
            if paso["accion"] == "elegir_cuenta":
                assert paso["cuenta"] in ofrecidas


def test_el_redondeo_y_los_pendientes_estan_cubiertos() -> None:
    redondeo = next(e for e in ESCENARIOS if e["id"] == "parte-con-redondeo")
    assert redondeo["movimientos_finales"][0]["monto"] == "33.34"
    sin_cuenta = next(e for e in ESCENARIOS if e["id"] == "parte-sin-cuenta")
    assert len(sin_cuenta["pasos"][0]["esperado"]["pendientes"]) == 1
    assert sin_cuenta["pasos"][0]["esperado"]["movimientos"] == []
    assert sin_cuenta["pasos"][-1]["esperado"]["pendientes"] == []


def test_el_rechazo_de_corregir_un_movimiento_de_grupo_dice_donde_corregir() -> None:
    ejemplo = next(e for e in ESCENARIOS if e["id"] == "ejemplo-2-6")
    rechazo = ejemplo["corregir_movimiento_de_grupo"]
    assert rechazo["error"] == "Se corrige en el grupo."
    aplicados = [m["evento_id"] for m in ejemplo["movimientos_finales"]]
    assert rechazo["evento_id"] in aplicados
