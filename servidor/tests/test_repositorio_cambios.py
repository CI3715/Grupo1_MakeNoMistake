"""Registro de cambios y claves de idempotencia contra PostgreSQL real (ADR-015).

Lo que solo puede comprobarse con la base de verdad: que gasto, partes, eventos
y clave quedan en una sola transacción, que los disparadores impiden modificar
o borrar el registro (INV-13), que dos gastos a la vez no repiten posiciones y
que dos reintentos simultáneos con la misma clave dejan un solo gasto. Se
omiten si no está definida ``CUENTAS_CLARAS_DB_URL_PRUEBAS``.
"""

from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import date, datetime, timezone
from decimal import Decimal
from functools import partial
from threading import Barrier
from typing import Any
from uuid import UUID

import pytest
from apoyo_compartido import crear_usuario
from psycopg import errors

from base_datos import BaseDatos
from cambios import armar_lote
from repositorio_cambios import RepositorioCambios
from repositorio_gastos import (
    ClaveYaUsada,
    GastoVista,
    ParteCalculada,
    RepositorioGastos,
)
from repositorio_grupos import RepositorioGrupos
from repositorio_usuarios import RepositorioUsuarios, Usuario

AHORA = datetime(2026, 11, 22, 12, 0, tzinfo=timezone.utc)
HUELLA = "a" * 64


@dataclass
class Escenario:
    base_datos: BaseDatos
    gastos: RepositorioGastos
    cambios: RepositorioCambios
    ana: Usuario
    beto: Usuario
    carla: Usuario
    dario: Usuario
    grupo_id: UUID


@pytest.fixture()
def escenario(base_datos_pruebas: BaseDatos) -> Escenario:
    usuarios = RepositorioUsuarios(base_datos_pruebas)
    grupos = RepositorioGrupos(base_datos_pruebas)
    ana, beto, carla, dario = (
        crear_usuario(usuarios, nombre) for nombre in ("ana", "beto", "carla", "dario")
    )
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
            id_grupo=grupo.id, usuario_id=persona.id, invitado_por=ana.id, ahora=AHORA
        )
        grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=persona.id, ahora=AHORA)
    return Escenario(
        base_datos=base_datos_pruebas,
        gastos=RepositorioGastos(base_datos_pruebas),
        cambios=RepositorioCambios(base_datos_pruebas),
        ana=ana,
        beto=beto,
        carla=carla,
        dario=dario,
        grupo_id=grupo.id,
    )


def registrar(
    e: Escenario,
    partes: list[tuple[Usuario, str, bool]] | None = None,
    concepto: str = "Hotel",
    clave: str | None = None,
    huella: str | None = None,
) -> GastoVista:
    partes = partes or [
        (e.ana, "33.34", True),
        (e.beto, "33.33", False),
        (e.carla, "33.33", False),
    ]
    calculadas = [
        ParteCalculada(usuario_id=u.id, monto=Decimal(m), lleva_residuo=r)
        for u, m, r in partes
    ]
    return e.gastos.registrar(
        grupo_id=e.grupo_id,
        concepto=concepto,
        monto=sum((p.monto for p in calculadas), Decimal("0.00")),
        fecha=date(2026, 3, 14),
        pagador_id=e.ana.id,
        regla="montos",
        partes=calculadas,
        registrado_por=e.ana.id,
        ahora=AHORA,
        clave_idempotencia=clave,
        huella=(huella or HUELLA) if clave else None,
    )


def contar(e: Escenario, tabla: str) -> int:
    with e.base_datos.conexion() as conexion:
        fila = conexion.execute(f"SELECT count(*) AS n FROM {tabla}").fetchone()
    assert fila is not None
    return int(fila["n"])


# ---------------------------------------------------------- eventos del gasto


def test_cada_participante_recibe_su_evento_en_la_misma_transaccion(
    escenario: Escenario,
) -> None:
    vista = registrar(escenario)
    for usuario, monto, redondeo in (
        (escenario.ana, "33.34", True),
        (escenario.beto, "33.33", False),
        (escenario.carla, "33.33", False),
    ):
        (evento,) = escenario.cambios.listar_desde(usuario.id, 0, 101)
        assert evento.usuario == usuario.id
        assert evento.posicion == 1
        assert evento.gasto_id == vista.gasto.id
        assert (evento.grupo_id, evento.grupo_nombre) == (
            escenario.grupo_id,
            "Viaje a Mérida",
        )
        assert (evento.concepto, evento.fecha, evento.moneda) == (
            "Hotel",
            date(2026, 3, 14),
            "USD",
        )
        assert (evento.monto, evento.incluye_redondeo) == (Decimal(monto), redondeo)


def test_el_excluido_no_recibe_evento_ni_consume_posicion(
    escenario: Escenario,
) -> None:
    e = escenario
    registrar(e)
    registrar(e, [(e.ana, "10.00", False), (e.beto, "10.00", False)], "Taxi")
    registrar(e, concepto="Museo")
    carla = e.cambios.listar_desde(e.carla.id, 0, 101)
    assert [(ev.concepto, ev.posicion) for ev in carla] == [("Hotel", 1), ("Museo", 2)]
    assert [ev.posicion for ev in e.cambios.listar_desde(e.beto.id, 0, 101)] == [
        1,
        2,
        3,
    ]
    assert e.cambios.listar_desde(e.dario.id, 0, 101) == []


def test_una_parte_de_cero_no_se_guarda_en_el_registro(escenario: Escenario) -> None:
    e = escenario
    registrar(
        e, [(e.ana, "0.01", True), (e.beto, "0.01", True), (e.carla, "0.00", False)]
    )
    assert e.cambios.listar_desde(e.carla.id, 0, 101) == []
    assert contar(e, "cambios") == 2


def test_listar_desde_respeta_cursor_limite_y_orden(escenario: Escenario) -> None:
    e = escenario
    for numero in range(5):
        registrar(e, concepto=f"Gasto {numero}")
    posteriores = e.cambios.listar_desde(e.beto.id, 2, 3)
    assert [ev.posicion for ev in posteriores] == [3, 4, 5]
    lote = armar_lote(e.cambios.listar_desde(e.beto.id, 0, 3), 0, 2)
    assert ([ev.posicion for ev in lote.eventos], lote.cursor, lote.hay_mas) == (
        [1, 2],
        2,
        True,
    )


# ------------------------------------------------------- el registro solo crece


@pytest.mark.parametrize(
    "sentencia",
    [
        "UPDATE cambios SET monto = 1",
        "UPDATE cambios SET posicion = posicion + 100",
        "DELETE FROM cambios",
    ],
)
def test_los_disparadores_rechazan_modificar_o_borrar(
    escenario: Escenario, sentencia: str
) -> None:
    registrar(escenario)
    with pytest.raises(errors.RestrictViolation), escenario.base_datos.conexion() as c:
        c.execute(sentencia)
    assert contar(escenario, "cambios") == 3


def test_no_se_repite_posicion_ni_gasto_en_el_registro_de_alguien(
    escenario: Escenario,
) -> None:
    vista = registrar(escenario)
    (evento,) = escenario.cambios.listar_desde(escenario.ana.id, 0, 101)
    duplicado = """
        INSERT INTO cambios (
            id, usuario_id, posicion, gasto_id, grupo_id, grupo_nombre, concepto,
            fecha, moneda, monto, incluye_redondeo, fecha_registro
        )
        VALUES (gen_random_uuid(), %s, %s, %s, %s, 'x', 'x', '2026-03-14', 'USD',
                1, false, now())
    """
    for posicion in (evento.posicion, 99):
        with (
            pytest.raises(errors.UniqueViolation),
            escenario.base_datos.conexion() as c,
        ):
            c.execute(
                duplicado,
                (escenario.ana.id, posicion, vista.gasto.id, escenario.grupo_id),
            )


# ---------------------------------------------------------- idempotencia


def test_la_clave_se_guarda_con_el_gasto_y_se_puede_buscar(
    escenario: Escenario,
) -> None:
    vista = registrar(escenario, clave="clave-0001")
    guardada = escenario.gastos.buscar_clave(escenario.ana.id, "clave-0001")
    assert guardada is not None
    assert (guardada.gasto_id, guardada.huella) == (vista.gasto.id, HUELLA)
    assert escenario.gastos.buscar_clave(escenario.beto.id, "clave-0001") is None
    assert escenario.gastos.buscar(vista.gasto.id) == vista


def test_una_clave_repetida_no_deja_nada_del_segundo_registro(
    escenario: Escenario,
) -> None:
    registrar(escenario, clave="clave-0001")
    antes = {t: contar(escenario, t) for t in ("gastos", "partes_gasto", "cambios")}
    with pytest.raises(ClaveYaUsada):
        registrar(escenario, concepto="Otro", clave="clave-0001", huella="b" * 64)
    despues = {t: contar(escenario, t) for t in ("gastos", "partes_gasto", "cambios")}
    assert despues == antes
    posiciones = [
        ev.posicion for ev in escenario.cambios.listar_desde(escenario.beto.id, 0, 9)
    ]
    assert posiciones == [1]


def test_la_misma_clave_de_otra_persona_no_choca(escenario: Escenario) -> None:
    e = escenario
    registrar(e, clave="compartida")
    e.gastos.registrar(
        grupo_id=e.grupo_id,
        concepto="De beto",
        monto=Decimal("10.00"),
        fecha=date(2026, 3, 14),
        pagador_id=e.beto.id,
        regla="montos",
        partes=[ParteCalculada(usuario_id=e.beto.id, monto=Decimal("10.00"))],
        registrado_por=e.beto.id,
        ahora=AHORA,
        clave_idempotencia="compartida",
        huella=HUELLA,
    )
    assert contar(e, "claves_idempotencia") == 2


# ------------------------------------------------------------ concurrencia


def en_paralelo(tareas: list[Callable[[], Any]]) -> list[Any]:
    """Lanza las tareas a la vez (una barrera las suelta juntas)."""
    barrera = Barrier(len(tareas))

    def correr(tarea: Callable[[], Any]) -> Any:
        barrera.wait()
        try:
            return tarea()
        except Exception as error:  # se devuelve para revisarlo después
            return error

    with ThreadPoolExecutor(max_workers=len(tareas)) as hilos:
        return list(hilos.map(correr, tareas))


def test_gastos_simultaneos_no_repiten_ni_saltan_posiciones(
    escenario: Escenario,
) -> None:
    resultados = en_paralelo(
        [partial(registrar, escenario, concepto=f"Gasto {n}") for n in range(8)]
    )
    assert all(isinstance(r, GastoVista) for r in resultados), resultados
    for usuario in (escenario.ana, escenario.beto, escenario.carla):
        eventos = escenario.cambios.listar_desde(usuario.id, 0, 101)
        assert [ev.posicion for ev in eventos] == list(range(1, 9))
        assert len({ev.gasto_id for ev in eventos}) == 8


def test_reintentos_simultaneos_con_la_misma_clave_dejan_un_solo_gasto(
    escenario: Escenario,
) -> None:
    resultados = en_paralelo(
        [(lambda: registrar(escenario, clave="reintento-01")) for _ in range(4)]
    )
    guardados = [r for r in resultados if isinstance(r, GastoVista)]
    assert len(guardados) == 1
    assert all(isinstance(r, ClaveYaUsada) for r in resultados if r not in guardados)
    assert contar(escenario, "gastos") == 1
    assert contar(escenario, "cambios") == 3
    guardada = escenario.gastos.buscar_clave(escenario.ana.id, "reintento-01")
    assert guardada is not None and guardada.gasto_id == guardados[0].gasto.id
