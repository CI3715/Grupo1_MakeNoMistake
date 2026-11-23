"""Clave de idempotencia en ``POST /grupos/{id}/gastos`` (RNF-17, ESC-09).

201 con clave nueva, 200 con el gasto original si se repite clave y cuerpo, 409
si se repite la clave con otro cuerpo. Orden: 401 → 404 → 400 → 200/409. Un 400
o un 404 no consumen la clave, y la clave es por usuario.
"""

from typing import Any

import pytest
from apoyo_compartido import leer_cambios_http, registrar_gasto_idempotente_http
from fastapi.testclient import TestClient

Usuarios = dict[str, dict[str, Any]]
CLAVE = "3f2b8c1e-6a4d-4b7e-9c10-2d5e7a9b0c11"
MENSAJE_409 = (
    "Esa clave ya se usó con otros datos; usa una clave nueva para un gasto distinto."
)
MENSAJE_CLAVE = "La clave de idempotencia debe tener de 8 a 128 caracteres visibles."


def cena(**cambios: Any) -> dict[str, Any]:
    datos: dict[str, Any] = {
        "concepto": "Cena",
        "monto": "60.00",
        "fecha": "2026-03-15",
        "pagador": "beto",
        "regla": "porcentajes",
        "participantes": [
            {"usuario": "ana", "valor": "50"},
            {"usuario": "beto", "valor": "25"},
            {"usuario": "carla", "valor": "25"},
        ],
    }
    datos.update(cambios)
    return datos


def enviar(
    cliente: TestClient,
    grupo: str,
    usuarios: Usuarios,
    quien: str = "ana",
    clave: str | None = CLAVE,
    datos: dict[str, Any] | None = None,
) -> Any:
    return registrar_gasto_idempotente_http(
        cliente, grupo, usuarios[quien]["auth"], datos or cena(), clave
    )


def gastos_del_grupo(cliente: TestClient, grupo: str, usuarios: Usuarios) -> list[Any]:
    resp = cliente.get(f"/grupos/{grupo}", headers=usuarios["ana"]["auth"])
    assert resp.status_code == 200
    gastos: list[Any] = resp.json()["gastos"]
    return gastos


def eventos_de(cliente: TestClient, usuarios: Usuarios, quien: str) -> list[Any]:
    eventos: list[Any] = leer_cambios_http(cliente, usuarios[quien]["auth"]).json()[
        "eventos"
    ]
    return eventos


# ------------------------------------------------------------- 201, 200, 409


def test_clave_nueva_registra_y_responde_201(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    resp = enviar(cliente, grupo_merida, usuarios_flujo)
    assert resp.status_code == 201
    assert [p["monto"] for p in resp.json()["partes"]] == ["30.00", "15.00", "15.00"]


def test_reintento_con_la_misma_clave_y_cuerpo_responde_200_sin_duplicar(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    primero = enviar(cliente, grupo_merida, usuarios_flujo)
    eventos_antes = {u: eventos_de(cliente, usuarios_flujo, u) for u in ("ana", "beto")}

    for _ in range(3):
        repetido = enviar(cliente, grupo_merida, usuarios_flujo)
        assert repetido.status_code == 200
        assert repetido.json() == primero.json()

    assert len(gastos_del_grupo(cliente, grupo_merida, usuarios_flujo)) == 1
    for usuario, antes in eventos_antes.items():
        assert eventos_de(cliente, usuarios_flujo, usuario) == antes


def test_el_mismo_gasto_escrito_de_otra_forma_es_el_mismo_cuerpo(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    """La huella se calcula sobre los datos ya validados, no sobre el texto."""
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201
    variante = cena(
        concepto="  Cena ",
        monto="60",
        pagador="BETO",
        participantes=[
            {"usuario": "carla", "valor": "25.00"},
            {"usuario": "ana", "valor": "50.0"},
            {"usuario": "beto", "valor": "25"},
        ],
    )
    resp = enviar(cliente, grupo_merida, usuarios_flujo, datos=variante)
    assert resp.status_code == 200


@pytest.mark.parametrize(
    "cambio",
    [
        {"concepto": "Cena del viernes"},
        {"monto": "61.00"},
        {"fecha": "2026-03-16"},
        {"pagador": "ana"},
        {"regla": "partes_iguales"},
        {
            "participantes": [
                {"usuario": "ana", "valor": "50"},
                {"usuario": "beto", "valor": "50"},
            ]
        },
    ],
    ids=["concepto", "monto", "fecha", "pagador", "regla", "participantes"],
)
def test_la_misma_clave_con_otro_cuerpo_responde_409(
    cliente: TestClient,
    usuarios_flujo: Usuarios,
    grupo_merida: str,
    cambio: dict[str, Any],
) -> None:
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201
    resp = enviar(cliente, grupo_merida, usuarios_flujo, datos=cena(**cambio))
    assert resp.status_code == 409
    assert resp.json() == {
        "errores": [{"campo": "idempotency_key", "mensaje": MENSAJE_409}]
    }
    assert len(gastos_del_grupo(cliente, grupo_merida, usuarios_flujo)) == 1
    assert len(eventos_de(cliente, usuarios_flujo, "ana")) == 1


def test_la_misma_clave_en_otro_grupo_responde_409(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    from flujo_http import crear_grupo

    otro = crear_grupo(cliente, usuarios_flujo["ana"]["auth"], "Solo ana")
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201
    solo_ana = cena(pagador="ana", participantes=[{"usuario": "ana", "valor": "100"}])
    assert enviar(cliente, otro, usuarios_flujo, datos=solo_ana).status_code == 409


def test_la_clave_es_por_usuario(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    """La misma clave de ana y de beto no choca: son dos operaciones."""
    assert enviar(cliente, grupo_merida, usuarios_flujo, "ana").status_code == 201
    assert enviar(cliente, grupo_merida, usuarios_flujo, "beto").status_code == 201
    assert len(gastos_del_grupo(cliente, grupo_merida, usuarios_flujo)) == 2


def test_sin_clave_se_comporta_como_en_la_entrega_6(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    for _ in range(2):
        resp = enviar(cliente, grupo_merida, usuarios_flujo, clave=None)
        assert resp.status_code == 201
    assert len(gastos_del_grupo(cliente, grupo_merida, usuarios_flujo)) == 2
    assert len(eventos_de(cliente, usuarios_flujo, "carla")) == 2


def test_dos_claves_distintas_son_dos_gastos(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201
    otra = enviar(cliente, grupo_merida, usuarios_flujo, clave="otra-clave-0001")
    assert otra.status_code == 201
    assert len(gastos_del_grupo(cliente, grupo_merida, usuarios_flujo)) == 2


# --------------------------------------------------------------------- 400


@pytest.mark.parametrize("clave", ["", "corta", "a" * 129, "con espacios 123"])
def test_clave_invalida_responde_400_y_no_se_consume(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str, clave: str
) -> None:
    resp = enviar(cliente, grupo_merida, usuarios_flujo, clave=clave)
    assert resp.status_code == 400
    assert resp.json() == {
        "errores": [{"campo": "idempotency_key", "mensaje": MENSAJE_CLAVE}]
    }
    assert gastos_del_grupo(cliente, grupo_merida, usuarios_flujo) == []
    assert eventos_de(cliente, usuarios_flujo, "ana") == []


def test_clave_invalida_y_datos_invalidos_se_informan_juntos(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    resp = enviar(
        cliente, grupo_merida, usuarios_flujo, clave="x", datos=cena(fecha="")
    )
    assert resp.status_code == 400
    campos = [e["campo"] for e in resp.json()["errores"]]
    assert campos[0] == "idempotency_key" and "fecha" in campos


def test_datos_invalidos_no_consumen_la_clave(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    invalido = enviar(cliente, grupo_merida, usuarios_flujo, datos=cena(monto="0"))
    assert invalido.status_code == 400
    # Con la clave intacta, el cuerpo corregido se registra (201, no 409).
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201


def test_el_400_va_antes_que_el_409(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    assert enviar(cliente, grupo_merida, usuarios_flujo).status_code == 201
    resp = enviar(cliente, grupo_merida, usuarios_flujo, datos=cena(concepto=""))
    assert resp.status_code == 400


# --------------------------------------------------------------- 401 y 404


def test_sin_sesion_responde_401_aunque_la_clave_sea_invalida(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    resp = cliente.post(
        f"/grupos/{grupo_merida}/gastos",
        json=cena(),
        headers={"Idempotency-Key": "x"},
    )
    assert resp.status_code == 401


def test_quien_no_es_del_grupo_recibe_404_y_no_consume_la_clave(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    ajeno = enviar(cliente, grupo_merida, usuarios_flujo, "dario", clave="x")
    assert ajeno.status_code == 404
    assert ajeno.json() == {"mensaje": "Grupo no encontrado."}
    # dario conserva su clave: en su propio grupo se registra con 201.
    from flujo_http import crear_grupo

    propio = crear_grupo(cliente, usuarios_flujo["dario"]["auth"], "De dario")
    solo_dario = cena(
        pagador="dario", participantes=[{"usuario": "dario", "valor": "100"}]
    )
    assert enviar(cliente, grupo_merida, usuarios_flujo, "dario").status_code == 404
    resp = enviar(cliente, propio, usuarios_flujo, "dario", datos=solo_dario)
    assert resp.status_code == 201


def test_el_404_va_antes_que_el_409(
    cliente: TestClient, usuarios_flujo: Usuarios, grupo_merida: str
) -> None:
    from flujo_http import crear_grupo

    assert enviar(cliente, grupo_merida, usuarios_flujo, "beto").status_code == 201
    ajeno = crear_grupo(cliente, usuarios_flujo["dario"]["auth"], "De dario")
    resp = enviar(cliente, ajeno, usuarios_flujo, "beto")
    assert resp.status_code == 404
