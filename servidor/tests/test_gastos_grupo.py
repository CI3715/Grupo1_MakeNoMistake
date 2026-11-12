"""POST /grupos/{id}/gastos y GET /grupos/{id} (Entrega 6).

Cubre RF-COM-04 (registrar un gasto), RF-COM-05 (división exacta), RF-COM-08
(balances), INV-07 (las partes suman el monto), INV-08 (los balances suman
cero), INV-17 y RNF-07 (solo miembros aceptados, 404 uniforme), RNF-08 (sin
coma flotante), ESC-08 (aislamiento) y ESC-10 (ni un céntimo de más ni de
menos). Todo pasa por HTTP con personas recién registradas.
"""

from decimal import Decimal
from typing import Any
from uuid import uuid4

import pytest
from fastapi.testclient import TestClient
from flujo_http import (
    BALANCES_2_5,
    aceptar_grupo,
    crear_grupo,
    gasto,
    grupo_de_tres,
    hacerse_amigos,
    invitar,
    nueva_persona,
    registrar_ejemplo_2_5,
)

from division import Participante, dividir

TRES_IGUALES = [("ana", ""), ("beto", ""), ("carla", "")]
NO_ENCONTRADO = {"mensaje": "Grupo no encontrado."}


def registrar(
    cliente: TestClient, grupo_id: str, auth: dict[str, str], cuerpo: dict[str, Any]
) -> Any:
    return cliente.post(f"/grupos/{grupo_id}/gastos", json=cuerpo, headers=auth)


def detalle(cliente: TestClient, grupo_id: str, auth: dict[str, str]) -> Any:
    resp = cliente.get(f"/grupos/{grupo_id}", headers=auth)
    assert resp.status_code == 200, resp.text
    return resp.json()


def errores(resp: Any) -> list[tuple[str, str]]:
    assert resp.status_code == 400, resp.text
    return [(e["campo"], e["mensaje"]) for e in resp.json()["errores"]]


class TestRegistrarGasto:
    def test_cien_en_partes_iguales_guarda_33_34_33_33_33_33(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["beto"],
            gasto("100.00", "ana", "partes_iguales", TRES_IGUALES),
        )

        assert resp.status_code == 201, resp.text
        cuerpo = resp.json()
        assert sorted(cuerpo) == [
            "concepto",
            "fecha",
            "id",
            "monto",
            "pagador",
            "partes",
            "regla",
        ]
        assert cuerpo["monto"] == "100.00"
        assert cuerpo["pagador"] == "ana"
        assert cuerpo["regla"] == "partes_iguales"
        assert cuerpo["fecha"] == "2026-11-12"
        assert cuerpo["partes"] == [
            {"usuario": "ana", "monto": "33.34", "incluye_redondeo": True},
            {"usuario": "beto", "monto": "33.33", "incluye_redondeo": False},
            {"usuario": "carla", "monto": "33.33", "incluye_redondeo": False},
        ]

    def test_diez_por_porcentajes_guarda_3_34_3_33_3_33(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(
                "10.00",
                "carla",
                "porcentajes",
                [("ana", "33.33"), ("beto", "33.33"), ("carla", "33.34")],
            ),
        )
        assert resp.status_code == 201, resp.text
        assert [(p["monto"], p["incluye_redondeo"]) for p in resp.json()["partes"]] == [
            ("3.34", True),
            ("3.33", False),
            ("3.33", False),
        ]

    def test_montos_fijos_que_suman_el_total_no_llevan_redondeo(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(
                "120.00",
                "ana",
                "montos_fijos",
                [("carla", "30.00"), ("ana", "50.00"), ("beto", "40.00")],
            ),
        )
        assert resp.status_code == 201, resp.text
        assert resp.json()["regla"] == "montos_fijos"
        assert resp.json()["partes"] == [
            {"usuario": "ana", "monto": "50.00", "incluye_redondeo": False},
            {"usuario": "beto", "monto": "40.00", "incluye_redondeo": False},
            {"usuario": "carla", "monto": "30.00", "incluye_redondeo": False},
        ]

    def test_un_miembro_puede_quedar_fuera_del_reparto(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("0.01", "beto", "partes_iguales", [("Beto", ""), ("ana", "")]),
        )
        assert resp.status_code == 201, resp.text
        assert resp.json()["partes"] == [
            {"usuario": "ana", "monto": "0.01", "incluye_redondeo": True},
            {"usuario": "beto", "monto": "0.00", "incluye_redondeo": False},
        ]

    def test_el_monto_maximo_no_pierde_ningun_centimo(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("9999999999.99", "ana", "partes_iguales", TRES_IGUALES),
        )
        assert resp.status_code == 201, resp.text
        partes = [Decimal(p["monto"]) for p in resp.json()["partes"]]
        assert partes == [
            Decimal("3333333333.33"),
            Decimal("3333333333.33"),
            Decimal("3333333333.33"),
        ]
        assert sum(partes) == Decimal("9999999999.99")

    def test_lo_guardado_coincide_con_la_vista_previa(
        self, cliente: TestClient
    ) -> None:
        """La app previsualiza con la misma regla; el servidor guarda eso mismo."""
        grupo_id, auth = grupo_de_tres(cliente)
        valores = [("carla", "12.5"), ("ana", "37.5"), ("beto", "50")]
        previa = dividir(
            "77.77", "porcentajes", [Participante(u, v) for u, v in valores]
        )
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("77.77", "beto", "porcentajes", valores),
        )
        assert resp.status_code == 201, resp.text
        assert resp.json()["partes"] == [
            {
                "usuario": p.usuario,
                "monto": p.monto,
                "incluye_redondeo": p.incluye_redondeo,
            }
            for p in previa.partes
        ]


class TestErroresDelGasto:
    def test_porcentajes_que_suman_95_dicen_faltan_5(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(
                "100.00",
                "ana",
                "porcentajes",
                [("ana", "50"), ("beto", "25"), ("carla", "20")],
            ),
        )
        assert errores(resp) == [("porcentajes", "Los porcentajes suman 95; faltan 5.")]

    def test_porcentajes_que_suman_105_dicen_sobran_5(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(
                "100.00",
                "ana",
                "porcentajes",
                [("ana", "50"), ("beto", "30"), ("carla", "25")],
            ),
        )
        assert errores(resp) == [
            ("porcentajes", "Los porcentajes suman 105; sobran 5.")
        ]

    def test_montos_que_no_llegan_al_total_dicen_la_diferencia_con_coma(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(
                "120.00",
                "ana",
                "montos_fijos",
                [("ana", "50.00"), ("beto", "40.00"), ("carla", "28.00")],
            ),
        )
        assert errores(resp) == [("montos", "Los montos suman 118,00; faltan 2,00.")]

    def test_montos_que_se_pasan_dicen_sobran(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10.00", "ana", "montos_fijos", [("ana", "6.00"), ("beto", "4.50")]),
        )
        assert errores(resp) == [("montos", "Los montos suman 10,50; sobran 0,50.")]

    def test_un_gasto_vacio_informa_cada_campo(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(cliente, grupo_id, auth["ana"], {})
        assert [campo for campo, _ in errores(resp)] == [
            "concepto",
            "fecha",
            "pagador",
            "monto",
            "regla",
            "participantes",
        ]

    @pytest.mark.parametrize("monto", ["0", "0.00", "-5.00", "10.001", "abc", "1e3"])
    def test_monto_invalido_falla_en_el_campo_monto(
        self, cliente: TestClient, monto: str
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto(monto, "ana", "partes_iguales", TRES_IGUALES),
        )
        assert errores(resp) == [
            ("monto", "El monto debe ser mayor que 0 y tener hasta dos decimales.")
        ]

    def test_monto_por_encima_del_maximo(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10000000000.00", "ana", "partes_iguales", TRES_IGUALES),
        )
        assert errores(resp) == [("monto", "El monto es demasiado grande.")]

    def test_un_monto_como_numero_json_se_rechaza(self, cliente: TestClient) -> None:
        """Los montos viajan como texto: un número JSON sería coma flotante."""
        grupo_id, auth = grupo_de_tres(cliente)
        cuerpo = gasto("100.00", "ana", "partes_iguales", TRES_IGUALES)
        cuerpo["monto"] = 100.0
        resp = registrar(cliente, grupo_id, auth["ana"], cuerpo)
        assert [campo for campo, _ in errores(resp)] == ["monto"]

    def test_regla_desconocida(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10.00", "ana", "iguales", TRES_IGUALES),
        )
        assert errores(resp) == [("regla", "Elige una regla de división válida.")]

    def test_fecha_imposible(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        cuerpo = gasto(
            "10.00", "ana", "partes_iguales", TRES_IGUALES, fecha="2026-02-30"
        )
        resp = registrar(cliente, grupo_id, auth["ana"], cuerpo)
        assert errores(resp) == [
            ("fecha", "La fecha debe tener el formato AAAA-MM-DD.")
        ]

    def test_participante_repetido(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10.00", "ana", "partes_iguales", [("ana", ""), ("ANA", "")]),
        )
        assert errores(resp) == [
            ("participantes", "ana está repetido entre los participantes.")
        ]

    def test_pagador_y_participante_ajenos_al_grupo(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10.00", "dario", "partes_iguales", [("ana", ""), ("dario", "")]),
        )
        assert errores(resp) == [
            ("pagador", "El pagador debe ser miembro del grupo."),
            ("participantes", "dario no es miembro del grupo."),
        ]

    def test_un_invitado_sin_aceptar_no_participa(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        hacerse_amigos(cliente, auth["ana"], "dario", auth["dario"])
        invitar(cliente, grupo_id, auth["ana"], "dario")
        resp = registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("10.00", "ana", "partes_iguales", [("ana", ""), ("dario", "")]),
        )
        assert errores(resp) == [("participantes", "dario no es miembro del grupo.")]

    def test_un_gasto_rechazado_no_deja_rastro(self, cliente: TestClient) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        registrar(
            cliente,
            grupo_id,
            auth["ana"],
            gasto("100.00", "ana", "porcentajes", [("ana", "50"), ("beto", "40")]),
        )
        datos = detalle(cliente, grupo_id, auth["ana"])
        assert datos["gastos"] == []
        assert {b["balance"] for b in datos["balances"]} == {"0.00"}


class TestAutorizacion:
    """El mismo 404 para grupo inexistente, mal formado, ajeno o no aceptado."""

    @pytest.fixture
    def escenario(self, cliente: TestClient) -> tuple[str, dict[str, dict[str, str]]]:
        grupo_id, auth = grupo_de_tres(cliente)
        auth["eva"] = nueva_persona(cliente, "eva")
        hacerse_amigos(cliente, auth["ana"], "eva", auth["eva"])
        invitar(cliente, grupo_id, auth["ana"], "eva")  # eva no acepta
        return grupo_id, auth

    @pytest.mark.parametrize(
        ("quien", "que_grupo"),
        [
            ("dario", "real"),
            ("eva", "real"),
            ("ana", "inventado"),
            ("ana", "mal-formado"),
        ],
        ids=["ajeno", "invitado-sin-aceptar", "inexistente", "id-mal-formado"],
    )
    def test_detalle_y_gasto_responden_el_mismo_404(
        self,
        cliente: TestClient,
        escenario: tuple[str, dict[str, dict[str, str]]],
        quien: str,
        que_grupo: str,
    ) -> None:
        grupo_id, auth = escenario
        objetivo = {"real": grupo_id, "inventado": str(uuid4()), "mal-formado": "abc"}[
            que_grupo
        ]
        cuerpo = gasto("10.00", quien, "partes_iguales", [(quien, "")])

        lectura = cliente.get(f"/grupos/{objetivo}", headers=auth[quien])
        escritura = registrar(cliente, objetivo, auth[quien], cuerpo)
        invitacion = cliente.post(
            f"/grupos/{objetivo}/miembros",
            json={"usuario": "beto"},
            headers=auth[quien],
        )

        for resp in (lectura, escritura, invitacion):
            assert resp.status_code == 404
            assert resp.json() == NO_ENCONTRADO
            assert resp.headers["content-length"] == str(len(resp.content))
        assert lectura.content == escritura.content == invitacion.content

    def test_el_404_va_antes_que_la_validacion_del_cuerpo(
        self, cliente: TestClient, escenario: tuple[str, dict[str, dict[str, str]]]
    ) -> None:
        """Un ajeno con datos inválidos no aprende nada: recibe 404, no 400."""
        grupo_id, auth = escenario
        resp = registrar(cliente, grupo_id, auth["dario"], {"monto": "-1"})
        assert resp.status_code == 404
        assert resp.json() == NO_ENCONTRADO

    def test_sin_sesion_responde_401(self, cliente: TestClient) -> None:
        grupo_id = str(uuid4())
        assert cliente.get(f"/grupos/{grupo_id}").status_code == 401
        assert cliente.post(f"/grupos/{grupo_id}/gastos", json={}).status_code == 401
        assert (
            cliente.get(
                f"/grupos/{grupo_id}", headers={"Authorization": "Bearer no-vale"}
            ).status_code
            == 401
        )

    def test_al_aceptar_la_invitacion_ya_ve_el_grupo(
        self, cliente: TestClient, escenario: tuple[str, dict[str, dict[str, str]]]
    ) -> None:
        grupo_id, auth = escenario
        assert (
            cliente.get(f"/grupos/{grupo_id}", headers=auth["eva"]).status_code == 404
        )
        aceptar_grupo(cliente, grupo_id, auth["eva"])
        datos = detalle(cliente, grupo_id, auth["eva"])
        assert "eva" in [m["usuario"] for m in datos["miembros"]]


class TestDetalleDelGrupo:
    def test_tiene_la_forma_que_usa_la_app(self, cliente: TestClient) -> None:
        """Las claves de ``GrupoDetalleDto`` en app/src-tauri/src/dto_social.rs."""
        grupo_id, auth = grupo_de_tres(cliente)
        registrar_ejemplo_2_5(cliente, grupo_id, auth)
        datos = detalle(cliente, grupo_id, auth["carla"])

        assert sorted(datos) == ["balances", "gastos", "grupo", "miembros"]
        assert sorted(datos["grupo"]) == [
            "actividad",
            "cantidad_miembros",
            "descripcion",
            "id",
            "moneda",
            "nombre",
            "rol",
        ]
        assert sorted(datos["miembros"][0]) == ["nombre_visible", "rol", "usuario"]
        assert sorted(datos["gastos"][0]) == [
            "concepto",
            "fecha",
            "id",
            "monto",
            "pagador",
            "partes",
            "regla",
        ]
        assert sorted(datos["gastos"][0]["partes"][0]) == [
            "incluye_redondeo",
            "monto",
            "usuario",
        ]
        assert sorted(datos["balances"][0]) == [
            "balance",
            "le_corresponde",
            "pago",
            "usuario",
        ]

    def test_ejemplo_2_5_da_36_66_11_67_y_menos_48_33(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        registrar_ejemplo_2_5(cliente, grupo_id, auth)

        for quien in ("ana", "beto", "carla"):
            datos = detalle(cliente, grupo_id, auth[quien])
            assert datos["balances"] == BALANCES_2_5
            assert sum(Decimal(b["balance"]) for b in datos["balances"]) == 0

        datos = detalle(cliente, grupo_id, auth["carla"])
        assert datos["grupo"]["rol"] == "miembro"
        assert datos["grupo"]["cantidad_miembros"] == 3
        assert [m["usuario"] for m in datos["miembros"]] == ["ana", "beto", "carla"]
        assert [m["rol"] for m in datos["miembros"]] == [
            "administrador",
            "miembro",
            "miembro",
        ]
        # Del más reciente al más antiguo.
        assert [g["concepto"] for g in datos["gastos"]] == ["Taxi", "Cena"]
        assert [g["regla"] for g in datos["gastos"]] == [
            "porcentajes",
            "partes_iguales",
        ]
        assert datos["gastos"][0]["partes"] == [
            {"usuario": "ana", "monto": "30.00", "incluye_redondeo": False},
            {"usuario": "beto", "monto": "15.00", "incluye_redondeo": False},
            {"usuario": "carla", "monto": "15.00", "incluye_redondeo": False},
        ]

    def test_un_grupo_sin_gastos_muestra_a_todos_en_cero(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        datos = detalle(cliente, grupo_id, auth["ana"])
        assert datos["gastos"] == []
        assert datos["balances"] == [
            {"usuario": u, "pago": "0.00", "le_corresponde": "0.00", "balance": "0.00"}
            for u in ("ana", "beto", "carla")
        ]

    def test_los_invitados_pendientes_no_aparecen_como_miembros(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        hacerse_amigos(cliente, auth["ana"], "dario", auth["dario"])
        invitar(cliente, grupo_id, auth["ana"], "dario")
        datos = detalle(cliente, grupo_id, auth["ana"])
        assert "dario" not in [m["usuario"] for m in datos["miembros"]]
        assert "dario" not in [b["usuario"] for b in datos["balances"]]

    def test_muchos_gastos_con_residuo_siguen_sumando_cero(
        self, cliente: TestClient
    ) -> None:
        grupo_id, auth = grupo_de_tres(cliente)
        for i, (monto, pagador) in enumerate(
            [("0.01", "ana"), ("0.02", "beto"), ("10.00", "carla"), ("99.99", "ana")]
        ):
            resp = registrar(
                cliente,
                grupo_id,
                auth["ana"],
                gasto(monto, pagador, "partes_iguales", TRES_IGUALES, concepto=f"g{i}"),
            )
            assert resp.status_code == 201, resp.text
        datos = detalle(cliente, grupo_id, auth["beto"])
        assert sum(Decimal(b["balance"]) for b in datos["balances"]) == 0
        assert sum(Decimal(b["pago"]) for b in datos["balances"]) == Decimal("110.02")

    def test_no_expone_datos_del_libro_personal_ni_correos(
        self, cliente: TestClient
    ) -> None:
        """INV-16 y RF-SOC-04: ni cuentas ni saldos ni correos en el detalle."""
        grupo_id, auth = grupo_de_tres(cliente)
        registrar_ejemplo_2_5(cliente, grupo_id, auth)
        texto = cliente.get(f"/grupos/{grupo_id}", headers=auth["ana"]).text
        for prohibido in ("cuenta", "saldo", "movimiento", "categoria", "tasa", "@"):
            assert prohibido not in texto


def test_flujo_completo_como_usuario_nuevo(cliente: TestClient) -> None:
    """De cero: registrarse, hacerse amigos, crear el grupo, invitar, aceptar,
    registrar los dos gastos del enunciado 2.5 y ver los mismos balances desde
    las tres cuentas. Nadie ajeno puede ver nada en ningún momento.
    """
    ana = nueva_persona(cliente, "ana")
    beto = nueva_persona(cliente, "beto")
    carla = nueva_persona(cliente, "carla")
    intrusa = nueva_persona(cliente, "intrusa")

    hacerse_amigos(cliente, ana, "beto", beto)
    hacerse_amigos(cliente, beto, "carla", carla)  # carla es amiga de beto, no de ana
    grupo_id = crear_grupo(cliente, ana, "Viaje a Mérida")

    # ana no puede invitar a carla: no son amigas.
    resp = cliente.post(
        f"/grupos/{grupo_id}/miembros", json={"usuario": "carla"}, headers=ana
    )
    assert resp.status_code == 403
    hacerse_amigos(cliente, carla, "ana", ana)

    for nombre, auth in (("beto", beto), ("carla", carla)):
        invitar(cliente, grupo_id, ana, nombre)
        assert cliente.get(f"/grupos/{grupo_id}", headers=auth).status_code == 404
        aceptar_grupo(cliente, grupo_id, auth)

    registrar_ejemplo_2_5(cliente, grupo_id, {"ana": ana, "beto": beto})

    vistas = [detalle(cliente, grupo_id, auth) for auth in (ana, beto, carla)]
    assert all(v["balances"] == BALANCES_2_5 for v in vistas)
    assert all(v["gastos"] == vistas[0]["gastos"] for v in vistas)
    assert vistas[0]["gastos"][1]["partes"][0] == {
        "usuario": "ana",
        "monto": "33.34",
        "incluye_redondeo": True,
    }

    listado = cliente.get("/grupos", headers=carla).json()
    assert [g["id"] for g in listado["grupos"]] == [grupo_id]

    ajeno = cliente.get(f"/grupos/{grupo_id}", headers=intrusa)
    assert ajeno.status_code == 404
    assert ajeno.json() == NO_ENCONTRADO
    assert cliente.get("/grupos", headers=intrusa).json()["grupos"] == []
