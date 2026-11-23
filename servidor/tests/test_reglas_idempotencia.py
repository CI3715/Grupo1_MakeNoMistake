"""Reglas puras de la clave de idempotencia (``idempotencia.py``, ADR-015)."""

from datetime import date
from decimal import Decimal
from uuid import UUID

import pytest

from idempotencia import (
    MENSAJE_CLAVE_INVALIDA,
    ClaveGuardada,
    Decision,
    ParteParaHuella,
    decidir,
    huella_gasto,
    validar_clave,
)

GRUPO = UUID("600f76c8-e198-5d6f-82b8-ffb04cc3fc13")
ANA = UUID("00000000-0000-4000-8000-00000000000a")
BETO = UUID("00000000-0000-4000-8000-00000000000b")
GASTO = UUID("ff6edddc-3a5d-55a2-b39c-c50a58649d1f")


def huella(**cambios: object) -> str:
    datos: dict[str, object] = {
        "grupo_id": GRUPO,
        "concepto": "Cena",
        "monto": Decimal("60.00"),
        "fecha": date(2026, 3, 15),
        "pagador_id": BETO,
        "regla": "porcentajes",
        "partes": [
            ParteParaHuella(ANA, Decimal("30.00"), 5000),
            ParteParaHuella(BETO, Decimal("30.00"), 5000),
        ],
    }
    datos.update(cambios)
    return huella_gasto(**datos)  # type: ignore[arg-type]


# -------------------------------------------------------------------- clave


@pytest.mark.parametrize(
    "clave",
    [
        "3f2b8c1e-6a4d-4b7e-9c10-2d5e7a9b0c11",
        "abcdefgh",
        "!" * 8,
        "~" * 128,
        "clave-uuid-12345",
    ],
)
def test_clave_valida_se_devuelve_tal_cual(clave: str) -> None:
    assert validar_clave(clave) == (clave, None)


def test_sin_clave_no_es_error() -> None:
    assert validar_clave(None) == (None, None)


@pytest.mark.parametrize(
    "clave",
    ["", "corta", "a" * 7, "a" * 129, "con espacio", "tab\tclave", "clave-ñandú"],
)
def test_clave_invalida_es_error_del_campo(clave: str) -> None:
    valor, error = validar_clave(clave)
    assert valor is None
    assert error is not None
    assert (error.campo, error.mensaje) == ("idempotency_key", MENSAJE_CLAVE_INVALIDA)


def test_el_error_no_repite_la_clave() -> None:
    _, error = validar_clave("clave con espacios")
    assert error is not None and "clave con espacios" not in error.mensaje


# ------------------------------------------------------------------- huella


def test_la_huella_es_sha256_hexadecimal() -> None:
    valor = huella()
    assert len(valor) == 64 and int(valor, 16) >= 0


def test_la_huella_no_depende_del_orden_de_las_partes_ni_de_la_escala() -> None:
    reordenada = huella(
        partes=[
            ParteParaHuella(BETO, Decimal("30"), 5000),
            ParteParaHuella(ANA, Decimal("30.0"), 5000),
        ],
        monto=Decimal("60"),
    )
    assert reordenada == huella()


@pytest.mark.parametrize(
    "cambio",
    [
        {"grupo_id": UUID("11111111-1111-4111-8111-111111111111")},
        {"concepto": "Cena del viernes"},
        {"monto": Decimal("60.01")},
        {"fecha": date(2026, 3, 16)},
        {"pagador_id": ANA},
        {"regla": "montos_fijos"},
        {
            "partes": [
                ParteParaHuella(ANA, Decimal("30.01"), 5001),
                ParteParaHuella(BETO, Decimal("29.99"), 4999),
            ]
        },
    ],
    ids=["grupo", "concepto", "monto", "fecha", "pagador", "regla", "partes"],
)
def test_cualquier_dato_distinto_cambia_la_huella(cambio: dict[str, object]) -> None:
    assert huella(**cambio) != huella()


def test_la_huella_rechaza_montos_con_mas_de_dos_decimales() -> None:
    with pytest.raises(ValueError):
        huella(monto=Decimal("60.001"))


# ------------------------------------------------------------------ decisión


def test_clave_nueva_se_registra() -> None:
    assert decidir(None, huella()) == Decision.REGISTRAR


def test_misma_clave_y_huella_devuelve_el_existente() -> None:
    guardada = ClaveGuardada(gasto_id=GASTO, huella=huella())
    assert decidir(guardada, huella()) == Decision.DEVOLVER_EXISTENTE


def test_misma_clave_con_otra_huella_es_conflicto() -> None:
    guardada = ClaveGuardada(gasto_id=GASTO, huella=huella())
    assert decidir(guardada, huella(concepto="Otra")) == Decision.CONFLICTO
