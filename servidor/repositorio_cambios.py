"""Registro de cambios de cada usuario sobre PostgreSQL (Entrega 7, ADR-015).

Las reglas (un evento por participante, posición creciente, lotes y cursor)
viven en ``cambios.py``; aquí solo está el SQL. ``agregar_eventos_de_gasto`` se
ejecuta dentro de la transacción que guarda el gasto (lo llama
``RepositorioGastos.registrar``): o quedan el gasto, sus partes, sus eventos y su
clave de idempotencia, o no queda nada.

La tabla ``cambios`` solo crece: aquí no hay ningún ``UPDATE`` ni ``DELETE``
sobre ella, y los disparadores del esquema los rechazan (INV-13).
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from datetime import date, datetime
from decimal import Decimal
from typing import Any
from uuid import UUID, uuid4

from psycopg import Connection

from base_datos import BaseDatos
from cambios import EventoCambio, ParteParaEvento, eventos_de_gasto

SQL_CREAR_POSICIONES = """
    INSERT INTO posiciones_usuario (usuario_id, ultima_posicion)
    VALUES (%s, 0)
    ON CONFLICT (usuario_id) DO NOTHING
"""

# Bloquea las filas de los participantes, siempre en el mismo orden, para que
# dos gastos a la vez no repitan posiciones ni se bloqueen entre sí.
SQL_BLOQUEAR_POSICIONES = """
    SELECT usuario_id, ultima_posicion
    FROM posiciones_usuario
    WHERE usuario_id = ANY(%s)
    ORDER BY usuario_id
    FOR UPDATE
"""

SQL_ACTUALIZAR_POSICION = """
    UPDATE posiciones_usuario SET ultima_posicion = %s WHERE usuario_id = %s
"""

SQL_INSERTAR_CAMBIO = """
    INSERT INTO cambios (
        id, usuario_id, posicion, gasto_id, grupo_id, grupo_nombre,
        concepto, fecha, moneda, monto, incluye_redondeo, fecha_registro
    )
    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
"""

SQL_LISTAR_DESDE = """
    SELECT id, usuario_id, posicion, gasto_id, grupo_id, grupo_nombre,
           concepto, fecha, moneda, monto, incluye_redondeo
    FROM cambios
    WHERE usuario_id = %s AND posicion > %s
    ORDER BY posicion
    LIMIT %s
"""


def agregar_eventos_de_gasto(
    conexion: Connection[dict[str, Any]],
    *,
    gasto_id: UUID,
    concepto: str,
    fecha: date,
    grupo_id: UUID,
    grupo_nombre: str,
    moneda: str,
    partes: Sequence[ParteParaEvento[UUID]],
    ahora: datetime,
    nuevo_id: Callable[[], UUID] = uuid4,
) -> list[EventoCambio[UUID]]:
    """Agrega al registro de cada participante el evento con su parte.

    Usa la conexión (y la transacción) de quien guarda el gasto.
    """
    con_parte = sorted({parte.usuario for parte in partes if parte.monto > 0}, key=str)
    if not con_parte:
        return []

    with conexion.cursor() as cursor:
        cursor.executemany(SQL_CREAR_POSICIONES, [(u,) for u in con_parte])
    filas = conexion.execute(SQL_BLOQUEAR_POSICIONES, (con_parte,)).fetchall()
    ultimas = {fila["usuario_id"]: int(fila["ultima_posicion"]) for fila in filas}

    eventos = eventos_de_gasto(
        gasto_id=gasto_id,
        concepto=concepto,
        fecha=fecha,
        grupo_id=grupo_id,
        grupo_nombre=grupo_nombre,
        moneda=moneda,
        partes=partes,
        ultimas_posiciones=ultimas,
        nuevo_id=nuevo_id,
    )
    with conexion.cursor() as cursor:
        cursor.executemany(
            SQL_INSERTAR_CAMBIO,
            [
                (
                    evento.id,
                    evento.usuario,
                    evento.posicion,
                    evento.gasto_id,
                    evento.grupo_id,
                    evento.grupo_nombre,
                    evento.concepto,
                    evento.fecha,
                    evento.moneda,
                    evento.monto,
                    evento.incluye_redondeo,
                    ahora,
                )
                for evento in eventos
            ],
        )
        cursor.executemany(
            SQL_ACTUALIZAR_POSICION,
            [(evento.posicion, evento.usuario) for evento in eventos],
        )
    return eventos


class RepositorioCambios:
    """Lectura del registro de cambios de cada usuario."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def listar_desde(
        self, usuario_id: UUID, desde: int, limite: int
    ) -> list[EventoCambio[UUID]]:
        """Hasta ``limite`` eventos de ``usuario_id`` posteriores a ``desde``.

        Solo los de esa persona: el filtro por ``usuario_id`` es lo que impide
        ver eventos ajenos (INV-17).
        """
        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(
                SQL_LISTAR_DESDE, (usuario_id, desde, limite)
            ).fetchall()
        return [_evento_desde_fila(fila) for fila in filas]


def _evento_desde_fila(fila: dict[str, Any]) -> EventoCambio[UUID]:
    monto: Decimal = fila["monto"]
    return EventoCambio(
        id=fila["id"],
        usuario=fila["usuario_id"],
        posicion=int(fila["posicion"]),
        grupo_id=fila["grupo_id"],
        grupo_nombre=fila["grupo_nombre"],
        gasto_id=fila["gasto_id"],
        concepto=fila["concepto"],
        fecha=fila["fecha"],
        moneda=fila["moneda"],
        monto=monto,
        incluye_redondeo=fila["incluye_redondeo"],
    )
