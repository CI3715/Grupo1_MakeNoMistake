"""Comprueba que ``docs/vectores-division.json`` es coherente consigo mismo.

El archivo fija los números que deben dar la división exacta y los balances
(RNF-08, INV-07, INV-08, ESC-10). Estas pruebas no calculan nada con el código
del servidor: solo revisan que los propios vectores cumplan las reglas del
enunciado, de modo que un vector mal escrito falle aquí y no en la defensa.
La prueba que compara el cálculo del servidor con estos vectores se añade
junto con las funciones de división.
"""

import json
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

RUTA = Path(__file__).resolve().parents[2] / "docs" / "vectores-division.json"
CENTIMO = Decimal("0.01")
REGLAS = {"partes_iguales", "porcentajes", "montos_fijos"}


def cargar() -> dict[str, Any]:
    datos: dict[str, Any] = json.loads(RUTA.read_text(encoding="utf-8"))
    return datos


VECTORES = cargar()
EXITOSAS = [d for d in VECTORES["divisiones"] if "esperado" in d]
ERRORES = [d for d in VECTORES["divisiones"] if "error" in d]


def dinero(texto: str) -> Decimal:
    """Un monto del archivo: texto con punto y exactamente dos decimales."""
    assert isinstance(texto, str), "los montos viajan como texto, nunca como número"
    numero = Decimal(texto)
    assert numero == numero.quantize(CENTIMO)
    assert texto.count(".") == 1 and len(texto.split(".")[1]) == 2
    return numero


def test_el_archivo_trae_los_casos_del_enunciado() -> None:
    ids = {d["id"] for d in VECTORES["divisiones"]}
    assert {
        "enunciado-100-partes-iguales",
        "enunciado-10-porcentajes",
        "enunciado-120-montos-fijos",
    } <= ids
    assert "enunciado-2-5" in {b["id"] for b in VECTORES["balances"]}


def test_los_identificadores_no_se_repiten() -> None:
    ids = [d["id"] for d in VECTORES["divisiones"]]
    ids += [b["id"] for b in VECTORES["balances"]]
    assert len(ids) == len(set(ids))


def test_cubre_residuo_un_participante_y_montos_grandes() -> None:
    con_residuo = [d for d in EXITOSAS if dinero(d["esperado"]["residuo"]) > 0]
    un_participante = [d for d in EXITOSAS if len(d["participantes"]) == 1]
    grandes = [d for d in EXITOSAS if dinero(d["monto"]) >= Decimal("1000000000")]
    assert len(con_residuo) >= 3
    assert len(un_participante) >= 1
    assert len(grandes) >= 1


@pytest.mark.parametrize("caso", EXITOSAS, ids=lambda c: c["id"])
def test_las_partes_suman_exactamente_el_monto(caso: dict[str, Any]) -> None:
    partes = caso["esperado"]["partes"]
    assert sum(dinero(p["monto"]) for p in partes) == dinero(caso["monto"])


@pytest.mark.parametrize("caso", EXITOSAS, ids=lambda c: c["id"])
def test_el_residuo_va_a_los_primeros_por_orden_alfabetico(
    caso: dict[str, Any],
) -> None:
    partes = caso["esperado"]["partes"]
    usuarios = [p["usuario"] for p in partes]
    assert usuarios == sorted(usuarios)
    assert sorted(usuarios) == sorted(p["usuario"] for p in caso["participantes"])

    marcadas = [p["incluye_redondeo"] for p in partes]
    residuo = dinero(caso["esperado"]["residuo"])
    assert sum(marcadas) == int(residuo / CENTIMO)
    # Las marcadas son un prefijo: primero las que reciben, luego las demás.
    assert marcadas == sorted(marcadas, reverse=True)
    # Nadie recibe más de un céntimo de residuo.
    assert residuo < CENTIMO * len(partes) or residuo == 0


@pytest.mark.parametrize("caso", EXITOSAS, ids=lambda c: c["id"])
def test_montos_fijos_y_porcentajes_de_los_vectores_cuadran(
    caso: dict[str, Any],
) -> None:
    assert caso["regla"] in REGLAS
    valores = [p["valor"] for p in caso["participantes"]]
    if caso["regla"] == "porcentajes":
        assert sum(Decimal(v) for v in valores) == 100
    elif caso["regla"] == "montos_fijos":
        assert sum(dinero(v) for v in valores) == dinero(caso["monto"])
        assert caso["esperado"]["residuo"] == "0.00"
    else:
        assert all(v == "" for v in valores)


@pytest.mark.parametrize("caso", ERRORES, ids=lambda c: c["id"])
def test_los_errores_indican_campo_y_mensaje(caso: dict[str, Any]) -> None:
    error = caso["error"]
    assert error["errores"]
    assert all(e["campo"] and e["mensaje"] for e in error["errores"])
    assert "esperado" not in caso
    if error["no_cuadra"]:
        assert len(error["errores"]) == 1
        assert error["mensaje_exacto"] is True
        assert error["errores"][0]["campo"] in {"porcentajes", "montos"}
        assert any(p in error["errores"][0]["mensaje"] for p in ("faltan", "sobran"))


@pytest.mark.parametrize("balance", VECTORES["balances"], ids=lambda b: b["id"])
def test_los_balances_suman_exactamente_cero(balance: dict[str, Any]) -> None:
    filas = balance["esperado"]["balances"]
    assert [f["usuario"] for f in filas] == balance["miembros"]
    assert sum(dinero(f["balance"]) for f in filas) == 0
    assert balance["esperado"]["suma"] == "0.00"
    for fila in filas:
        assert dinero(fila["balance"]) == dinero(fila["pago"]) - dinero(
            fila["le_corresponde"]
        )
    # Lo pagado y lo que corresponde suman lo mismo: no se pierde ningún céntimo.
    assert sum(dinero(f["pago"]) for f in filas) == sum(
        dinero(f["le_corresponde"]) for f in filas
    )


@pytest.mark.parametrize("balance", VECTORES["balances"], ids=lambda b: b["id"])
def test_los_balances_salen_de_las_partes_de_los_gastos(
    balance: dict[str, Any],
) -> None:
    pago = {m: Decimal(0) for m in balance["miembros"]}
    corresponde = {m: Decimal(0) for m in balance["miembros"]}
    for gasto in balance["gastos"]:
        assert sum(dinero(p["monto"]) for p in gasto["partes"]) == dinero(
            gasto["monto"]
        )
        pago[gasto["pagador"]] += dinero(gasto["monto"])
        for parte in gasto["partes"]:
            corresponde[parte["usuario"]] += dinero(parte["monto"])
    for fila in balance["esperado"]["balances"]:
        usuario = fila["usuario"]
        assert dinero(fila["pago"]) == pago[usuario]
        assert dinero(fila["le_corresponde"]) == corresponde[usuario]
