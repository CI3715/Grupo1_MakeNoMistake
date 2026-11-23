"""Registro de cambios y cursor contra PostgreSQL real (Entrega 7).

Comprueba que un gasto genera un evento por participante en la misma
transacción, que el contador por usuario no salta posiciones y que la lectura
por cursor entrega lotes de 100 con indicación de si quedan más. El registro
solo crece: también se prueba que UPDATE y DELETE están prohibidos.
"""

from collections.abc import Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import date, datetime, timezone
from decimal import Decimal
from threading import Barrier
from typing import Any
from uuid import UUID, uuid5

import pytest
from apoyo_compartido import crear_usuario
from psycopg import Connection, errors

import repositorio_cambios as modulo_cambios
from base_datos import BaseDatos
from repositorio_cambios import ESPACIO_CAMBIOS, RepositorioCambios
from repositorio_gastos import (
    Gasto,
    GastoVista,
    ParteCalculada,
    ParteGasto,
    RepositorioGastos,
)
from repositorio_grupos import RepositorioGrupos
from repositorio_usuarios import RepositorioUsuarios, Usuario

AHORA = datetime(2026, 11, 22, 12, 0, tzinfo=timezone.utc)


@dataclass
class Escenario:
    base_datos: BaseDatos
    usuarios: RepositorioUsuarios
    grupos: RepositorioGrupos
    gastos: RepositorioGastos
    cambios: RepositorioCambios
    ana: Usuario
    beto: Usuario
    carla: Usuario
    grupo_id: UUID


@pytest.fixture()
def escenario(base_datos_pruebas: BaseDatos) -> Escenario:
    usuarios = RepositorioUsuarios(base_datos_pruebas)
    grupos = RepositorioGrupos(base_datos_pruebas)
    cambios = RepositorioCambios(base_datos_pruebas)
    gastos = RepositorioGastos(base_datos_pruebas, cambios)

    ana = crear_usuario(usuarios, "ana")
    beto = crear_usuario(usuarios, "beto")
    carla = crear_usuario(usuarios, "carla")

    grupo = grupos.crear(
        creado_por=ana.id,
        nombre="Viaje a Mérida",
        descripcion=None,
        moneda="USD",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    for persona in (beto, carla):
        grupos.invitar(
            id_grupo=grupo.id,
            usuario_id=persona.id,
            invitado_por=ana.id,
            ahora=AHORA,
        )
        grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=persona.id, ahora=AHORA)

    return Escenario(
        base_datos=base_datos_pruebas,
        usuarios=usuarios,
        grupos=grupos,
        gastos=gastos,
        cambios=cambios,
        ana=ana,
        beto=beto,
        carla=carla,
        grupo_id=grupo.id,
    )


def _partes_iguales(escenario: Escenario) -> list[ParteCalculada]:
    return [
        ParteCalculada(
            usuario_id=escenario.ana.id, monto=Decimal("33.34"), lleva_residuo=True
        ),
        ParteCalculada(usuario_id=escenario.beto.id, monto=Decimal("33.33")),
        ParteCalculada(usuario_id=escenario.carla.id, monto=Decimal("33.33")),
    ]


def _registrar(
    escenario: Escenario,
    *,
    concepto: str = "Hotel",
    gastos: RepositorioGastos | None = None,
    partes: Sequence[ParteCalculada] | None = None,
    regla: str = "iguales",
) -> GastoVista:
    repositorio = gastos if gastos is not None else escenario.gastos
    return repositorio.registrar(
        grupo_id=escenario.grupo_id,
        concepto=concepto,
        monto=Decimal("100.00"),
        fecha=date(2026, 11, 20),
        pagador_id=escenario.ana.id,
        regla=regla,
        partes=partes if partes is not None else _partes_iguales(escenario),
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )


def test_un_evento_por_participante_con_datos_copiados(
    escenario: Escenario,
) -> None:
    vista = _registrar(escenario)

    with escenario.base_datos.conexion() as conexion:
        filas = conexion.execute("SELECT * FROM cambios ORDER BY posicion").fetchall()

    assert len(filas) == 3
    # Cada persona arranca su propio contador en 1.
    assert [fila["posicion"] for fila in filas] == [1, 1, 1]
    montos = {fila["usuario_id"]: fila["monto"] for fila in filas}
    assert montos == {
        escenario.ana.id: Decimal("33.34"),
        escenario.beto.id: Decimal("33.33"),
        escenario.carla.id: Decimal("33.33"),
    }
    for fila in filas:
        assert fila["id"] == uuid5(
            ESPACIO_CAMBIOS, f"{vista.gasto.id}:{fila['usuario_id']}"
        )
        assert fila["grupo_id"] == escenario.grupo_id
        assert fila["nombre_grupo"] == "Viaje a Mérida"
        assert fila["gasto_id"] == vista.gasto.id
        assert fila["concepto"] == "Hotel"
        assert fila["fecha"] == date(2026, 11, 20)
        assert fila["moneda"] == "USD"
        assert fila["creado_en"] is not None
    assert [fila["incluye_redondeo"] for fila in filas].count(True) == 1


def test_quien_no_esta_en_la_division_no_recibe_evento(
    escenario: Escenario,
) -> None:
    _registrar(
        escenario,
        regla="montos",
        partes=[
            ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("60.00")),
            ParteCalculada(usuario_id=escenario.beto.id, monto=Decimal("40.00")),
        ],
    )

    with escenario.base_datos.conexion() as conexion:
        filas = conexion.execute("SELECT usuario_id FROM cambios").fetchall()

    assert {fila["usuario_id"] for fila in filas} == {
        escenario.ana.id,
        escenario.beto.id,
    }


def test_si_falla_el_registro_no_quedan_eventos_ni_posiciones(
    escenario: Escenario,
) -> None:
    class CambiosQueFallan(RepositorioCambios):
        def agregar_eventos(
            self,
            conexion: Connection[dict[str, Any]],
            *,
            gasto: Gasto,
            nombre_grupo: str,
            moneda: str,
            partes: Sequence[ParteGasto],
        ) -> None:
            super().agregar_eventos(
                conexion,
                gasto=gasto,
                nombre_grupo=nombre_grupo,
                moneda=moneda,
                partes=partes,
            )
            raise RuntimeError("fallo a propósito al agregar eventos")

    cambios_que_fallan = CambiosQueFallan(escenario.base_datos)
    gastos = RepositorioGastos(escenario.base_datos, cambios_que_fallan)
    with pytest.raises(RuntimeError, match="fallo a propósito"):
        _registrar(escenario, gastos=gastos)

    with escenario.base_datos.conexion() as conexion:
        totales = {}
        for tabla in ("gastos", "partes_gasto", "cambios", "posiciones_usuario"):
            fila = conexion.execute(f"SELECT count(*) AS total FROM {tabla}").fetchone()
            assert fila is not None
            totales[tabla] = fila["total"]

    assert all(total == 0 for total in totales.values())


def test_dos_registros_concurrentes_no_saltan_posiciones(
    escenario: Escenario,
) -> None:
    barrera = Barrier(2)

    def registrar() -> None:
        barrera.wait()
        _registrar(escenario)

    with ThreadPoolExecutor(max_workers=2) as ejecutor:
        futuros = [ejecutor.submit(registrar) for _ in range(2)]
        for futuro in futuros:
            futuro.result()

    with escenario.base_datos.conexion() as conexion:
        filas = conexion.execute(
            "SELECT usuario_id, posicion FROM cambios ORDER BY usuario_id, posicion"
        ).fetchall()

    posiciones: dict[UUID, list[int]] = {}
    for fila in filas:
        posiciones.setdefault(fila["usuario_id"], []).append(fila["posicion"])
    assert set(posiciones) == {escenario.ana.id, escenario.beto.id, escenario.carla.id}
    # Dos gastos: cada usuario recibe sus posiciones 1 y 2, sin saltos ni repetidas.
    assert all(valores == [1, 2] for valores in posiciones.values())


def test_leer_lote_devuelve_cursor_y_avisa_si_quedan_mas(
    escenario: Escenario, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(modulo_cambios, "TAMANO_LOTE", 2)
    monkeypatch.setattr(modulo_cambios, "LIMITE_CONSULTA", 3)

    for concepto in ("Hotel", "Cena", "Taxi"):
        _registrar(escenario, concepto=concepto)

    primero = escenario.cambios.leer_lote(usuario_id=escenario.ana.id, desde=0)
    assert len(primero.eventos) == 2
    assert primero.hay_mas is True
    assert primero.cursor == primero.eventos[-1].posicion
    assert all(evento.usuario_id == escenario.ana.id for evento in primero.eventos)

    segundo = escenario.cambios.leer_lote(
        usuario_id=escenario.ana.id, desde=primero.cursor
    )
    assert len(segundo.eventos) == 1
    assert segundo.hay_mas is False
    assert segundo.cursor > primero.cursor

    vacio = escenario.cambios.leer_lote(
        usuario_id=escenario.ana.id, desde=segundo.cursor
    )
    assert vacio.eventos == ()
    assert vacio.cursor == segundo.cursor
    assert vacio.hay_mas is False


def test_cursor_negativo_se_rechaza(escenario: Escenario) -> None:
    with pytest.raises(ValueError, match="entero no negativo"):
        escenario.cambios.leer_lote(usuario_id=escenario.ana.id, desde=-1)


@pytest.mark.parametrize(
    "sentencia",
    [
        "UPDATE cambios SET concepto = 'otro'",
        "DELETE FROM cambios",
    ],
)
def test_el_registro_solo_crece(escenario: Escenario, sentencia: str) -> None:
    _registrar(escenario)

    with pytest.raises(errors.RaiseException):
        with escenario.base_datos.conexion() as conexion:
            conexion.execute(sentencia)
