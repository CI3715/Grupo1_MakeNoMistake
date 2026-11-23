"""Prueba de filtraciones en la sincronización y gastos (INV-16, INV-17, RES-02).

Sigue la estructura de ``tests/test_datos_privados_no_se_filtran.py`` sin
modificarlo y comprueba cinco propiedades sobre las rutas de la Entrega 7:

1. las respuestas de ``POST /grupos/{id}/gastos`` y ``GET /cambios`` no traen
   correos, contraseñas, hashes ni tokens;
2. cada evento de ``GET /cambios`` contiene exactamente los 9 campos acordados;
3. ``GET /cambios`` solo entrega a cada persona sus propias partes positivas
   (excluidos, partes ``0.00`` y terceros ajenos no reciben datos);
4. ninguna petición, respuesta, esquema OpenAPI ni definición SQL incluye
   cuentas, movimientos, saldos ni tasas del libro personal;
5. ni los logs ni la salida estándar contienen secretos al registrar gastos
   con idempotencia o consultar ``GET /cambios``.
"""

import json
import logging
from collections.abc import Generator
from pathlib import Path

import pytest
from flujo_http import (
    CONTRASENA,
    grupo_de_tres,
    hacerse_amigos,
    invitar,
    registrar_ejemplo_2_5,
)
from test_datos_privados_no_se_filtran import (
    CAMPOS_PRIVADOS,
    USUARIOS,
    ClienteConHistorial,
    claves,
)

from main import app

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
CLAVES_DEL_LIBRO = (
    "cuenta",
    "saldo",
    "movimiento",
    "tasa",
    "categoria",
    "de_grupo",
)


@pytest.fixture
def cliente() -> Generator[ClienteConHistorial, None, None]:
    instancia = ClienteConHistorial(app)
    with instancia:
        yield instancia


def ejecutar_flujo_entrega_7(
    cliente: ClienteConHistorial,
) -> tuple[str, dict[str, dict[str, str]]]:
    """Registra grupo, gastos (con exclusión y parte 0,00) y consulta /cambios."""
    grupo_id, auth = grupo_de_tres(cliente)
    registrar_ejemplo_2_5(cliente, grupo_id, auth)

    # dario es amigo e invitado pendiente, no miembro aceptado.
    hacerse_amigos(cliente, auth["ana"], "dario", auth["dario"])
    invitar(cliente, grupo_id, auth["ana"], "dario")

    # Gasto donde carla queda excluida (INV-17).
    taxi = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json={
            "concepto": "Taxi al aeropuerto",
            "monto": "40.00",
            "fecha": "2026-03-16",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": ""},
                {"usuario": "beto", "valor": ""},
            ],
        },
        headers={**auth["ana"], "Idempotency-Key": "privacidad-taxi-0001"},
    )
    assert taxi.status_code == 201, taxi.text

    # Reintento idempotente (200) y conflicto (409) para cubrir logs de idempotencia.
    reintento = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json={
            "concepto": "Taxi al aeropuerto",
            "monto": "40.00",
            "fecha": "2026-03-16",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": ""},
                {"usuario": "beto", "valor": ""},
            ],
        },
        headers={**auth["ana"], "Idempotency-Key": "privacidad-taxi-0001"},
    )
    assert reintento.status_code == 200, reintento.text

    # Gasto de 0,02 entre tres: carla tiene parte 0,00 y no recibe evento.
    peaje = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json={
            "concepto": "Peaje mínimo",
            "monto": "0.02",
            "fecha": "2026-03-17",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": ""},
                {"usuario": "beto", "valor": ""},
                {"usuario": "carla", "valor": ""},
            ],
        },
        headers={**auth["ana"], "Idempotency-Key": "privacidad-peaje-0001"},
    )
    assert peaje.status_code == 201, peaje.text

    for usuario in USUARIOS:
        assert cliente.get("/cambios", headers=auth[usuario]).status_code == 200
    for usuario in ("ana", "beto", "carla"):
        assert (
            cliente.get(f"/grupos/{grupo_id}", headers=auth[usuario]).status_code == 200
        )

    return grupo_id, auth


def test_gastos_y_cambios_no_filtran_correos_hashes_contrasenas_ni_tokens(
    cliente: ClienteConHistorial,
) -> None:
    """Ninguna respuesta de gastos o /cambios trae secretos de autenticación."""
    _, auth = ejecutar_flujo_entrega_7(cliente)
    tokens = [auth[u]["Authorization"].split(" ", 1)[1] for u in USUARIOS]

    for llamada in cliente.historial:
        if (llamada.metodo, llamada.ruta) in {
            ("POST", "/usuarios"),
            ("GET", "/usuarios/yo"),
            ("POST", "/sesiones"),
        }:
            continue
        assert "@" not in llamada.texto, f"{llamada.metodo} {llamada.ruta} trae correo"
        assert "argon2" not in llamada.texto
        assert CONTRASENA not in llamada.texto
        for token in tokens:
            assert token not in llamada.texto
        if llamada.texto:
            privados = claves(json.loads(llamada.texto)) & CAMPOS_PRIVADOS
            assert not privados, f"{llamada.metodo} {llamada.ruta} trae {privados}"


def test_cada_evento_de_cambios_trae_solo_los_campos_acordados(
    cliente: ClienteConHistorial,
) -> None:
    """El evento de GET /cambios contiene únicamente las 9 claves del contrato."""
    _, auth = ejecutar_flujo_entrega_7(cliente)

    for usuario in ("ana", "beto", "carla"):
        lote = cliente.get("/cambios", headers=auth[usuario]).json()
        assert set(lote) == {"eventos", "cursor", "hay_mas"}
        assert len(lote["eventos"]) >= 1
        for evento in lote["eventos"]:
            assert set(evento) == CAMPOS_DEL_EVENTO
            assert set(evento["grupo"]) == {"id", "nombre"}
            assert set(evento["gasto"]) == {"id", "concepto"}


def test_cambios_no_filtra_datos_de_otros_usuarios_ni_partes_cero(
    cliente: ClienteConHistorial,
) -> None:
    """INV-17: excluidos, partes 0,00 y terceros ajenos no reciben eventos."""
    _, auth = ejecutar_flujo_entrega_7(cliente)

    cambios_ana = cliente.get("/cambios", headers=auth["ana"]).json()
    cambios_carla = cliente.get("/cambios", headers=auth["carla"]).json()
    cambios_dario = cliente.get("/cambios", headers=auth["dario"]).json()

    conceptos_ana = [e["gasto"]["concepto"] for e in cambios_ana["eventos"]]
    conceptos_carla = [e["gasto"]["concepto"] for e in cambios_carla["eventos"]]

    assert "Taxi al aeropuerto" in conceptos_ana
    assert "Peaje mínimo" in conceptos_ana
    assert "Taxi al aeropuerto" not in conceptos_carla
    assert "Peaje mínimo" not in conceptos_carla
    assert len(cambios_carla["eventos"]) == len(cambios_ana["eventos"]) - 2
    assert cambios_dario == {"eventos": [], "cursor": 0, "hay_mas": False}


def test_ninguna_respuesta_ni_contrato_ni_sql_tiene_cuentas_saldos_o_tasas(
    cliente: ClienteConHistorial,
) -> None:
    """INV-16, RES-02: ni respuestas HTTP, ni OpenAPI, ni SQL del servidor."""
    grupo_id, auth = ejecutar_flujo_entrega_7(cliente)

    # Petición con campos privados del libro personal: el servidor los ignora.
    marcadores = ("Billetera Secreta Local", "987654.32", "Categoria Privada Local")
    creacion = cliente.post(
        f"/grupos/{grupo_id}/gastos",
        json={
            "concepto": "Entradas museo",
            "monto": "30.00",
            "fecha": "2026-03-17",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": ""},
                {"usuario": "beto", "valor": ""},
                {"usuario": "carla", "valor": ""},
            ],
            "cuenta": marcadores[0],
            "saldo": marcadores[1],
            "categoria": marcadores[2],
            "tasa": "36.50",
        },
        headers={**auth["ana"], "Idempotency-Key": "privacidad-extra-0001"},
    )
    assert creacion.status_code == 201, creacion.text

    for usuario in ("ana", "beto"):
        assert cliente.get("/cambios", headers=auth[usuario]).status_code == 200
        assert (
            cliente.get(f"/grupos/{grupo_id}", headers=auth[usuario]).status_code == 200
        )

    for llamada in cliente.historial:
        if not llamada.texto:
            continue
        for marcador in marcadores:
            assert marcador not in llamada.texto
        for clave in claves(json.loads(llamada.texto)):
            minusculas = clave.lower()
            for prohibida in CLAVES_DEL_LIBRO:
                assert prohibida not in minusculas, (
                    f"{llamada.metodo} {llamada.ruta} expone la clave {clave!r}"
                )

    for clave in claves(app.openapi()):
        minusculas = clave.lower()
        for prohibida in CLAVES_DEL_LIBRO:
            assert prohibida not in minusculas, (
                f"openapi.json contiene la clave prohibida {clave!r}"
            )

    servidor_dir = Path(__file__).resolve().parents[1]
    sql_textos = [
        (servidor_dir / "base_datos.py").read_text(encoding="utf-8"),
        *[
            ruta.read_text(encoding="utf-8")
            for ruta in sorted((servidor_dir / "migraciones").glob("*.sql"))
        ],
    ]
    esquema_sql = "\n".join(sql_textos).lower()
    for prohibida in ("cuentas", "movimientos", "saldos", "tasas", "de_grupo"):
        assert prohibida not in esquema_sql, (
            f"El esquema SQL contiene el término prohibido {prohibida!r}"
        )


def test_logs_y_salida_de_gastos_y_cambios_no_contienen_secretos(
    cliente: ClienteConHistorial,
    caplog: pytest.LogCaptureFixture,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Ni los logs ni stdout/stderr filtran correos, hashes, claves o tokens."""
    caplog.set_level(logging.DEBUG)

    _, auth = ejecutar_flujo_entrega_7(cliente)
    salida = capsys.readouterr()
    visto = caplog.text + salida.out + salida.err

    tokens = [auth[u]["Authorization"].split(" ", 1)[1] for u in USUARIOS]
    secretos = [CONTRASENA, "argon2", *tokens]
    secretos += [f"{usuario}@ejemplo.com" for usuario in USUARIOS]
    for secreto in secretos:
        assert secreto not in visto, f"El log o la salida contiene {secreto!r}"
