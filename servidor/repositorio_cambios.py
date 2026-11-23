"""Registro de cambios de la Entrega 7 sobre PostgreSQL.

Un cambio es la parte de un gasto de grupo que baja al libro de una persona.
El servidor guarda solo datos compartidos: nunca cuentas, movimientos ni
saldos (INV-16). Cada evento tiene un ``id`` estable, una posición consecutiva
por usuario y no se modifica ni se borra.

``agregar_eventos`` recibe la conexión ya abierta por el repositorio de
gastos: comparte su transacción para que gasto, partes, eventos y posiciones
se guarden juntos o no se guarde nada.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from datetime import date, datetime
from decimal import Decimal
from typing import Any
from uuid import UUID, uuid5

from psycopg import Connection

from base_datos import BaseDatos
from repositorio_gastos import Gasto, ParteGasto

# Espacio de nombres fijo para los UUID de los eventos. Con él, el mismo gasto
# y la misma persona producen siempre el mismo identificador, aunque se reintente
# el registro. No es un secreto: es una constante del contrato del servidor.
ESPACIO_CAMBIOS = UUID("7c1f0e2a-9b4d-4a61-8f0e-2d3c5b7a9e41")

TAMANO_LOTE = 100
# Se pide uno de más para saber si quedan eventos sin devolver.
LIMITE_CONSULTA = TAMANO_LOTE + 1

SQL_POSICION = """
    INSERT INTO posiciones (usuario_id, ultima_posicion)
    VALUES (%s, 1)
    ON CONFLICT (usuario_id) DO UPDATE
    SET ultima_posicion = posiciones.ultima_posicion + 1
    RETURNING ultima_posicion
"""

SQL_INSERTAR_CAMBIO = """
    INSERT INTO cambios (
        id, usuario_id, posicion, grupo_id, nombre_grupo, gasto_id,
        concepto, fecha, moneda, monto, incluye_redondeo
    )
    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
"""

SQL_LEER_LOTE = """
    SELECT id, posicion, usuario_id, grupo_id, nombre_grupo, gasto_id,
           concepto, fecha, moneda, monto, incluye_redondeo, creado_en
    FROM cambios
    WHERE usuario_id = %s AND posicion > %s
    ORDER BY posicion
    LIMIT %s
"""


@dataclass(frozen=True)
class Evento:
    """Un evento del registro de cambios, con el monto exacto en euros."""

    id: UUID
    posicion: int
    usuario_id: UUID
    grupo_id: UUID
    nombre_grupo: str
    gasto_id: UUID
    concepto: str
    fecha: date
    moneda: str
    monto: Decimal
    incluye_redondeo: bool
    creado_en: datetime


@dataclass(frozen=True)
class LoteCambios:
    """Hasta 100 eventos, el cursor para seguir y si queda algo más."""

    eventos: tuple[Evento, ...]
    cursor: int
    hay_mas: bool


class RepositorioCambios:
    """Consultas y escrituras de `cambios` y `posiciones`."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def agregar_eventos(
        self,
        conexion: Connection[dict[str, Any]],
        *,
        gasto: Gasto,
        nombre_grupo: str,
        moneda: str,
        partes: Sequence[ParteGasto],
    ) -> None:
        """Agrega un evento por parte usando la transacción que recibe.

        No abre ni cierra la conexión: eso lo hace quien registra el gasto.
        Las partes se recorren en orden de usuario para fijar el orden de los
        bloqueos del contador y evitar interbloqueos entre transacciones.
        """

        for parte in sorted(partes, key=lambda parte: parte.usuario_id):
            fila = conexion.execute(SQL_POSICION, (parte.usuario_id,)).fetchone()
            assert fila is not None
            posicion = fila["ultima_posicion"]
            conexion.execute(
                SQL_INSERTAR_CAMBIO,
                (
                    uuid5(ESPACIO_CAMBIOS, f"{gasto.id}:{parte.usuario_id}"),
                    parte.usuario_id,
                    posicion,
                    gasto.grupo_id,
                    nombre_grupo,
                    gasto.id,
                    gasto.concepto,
                    gasto.fecha,
                    moneda,
                    parte.monto,
                    parte.lleva_residuo,
                ),
            )

    def leer_lote(self, *, usuario_id: UUID, desde: int) -> LoteCambios:
        """Devuelve los eventos posteriores al cursor, de 100 en 100."""

        if isinstance(desde, bool) or desde < 0:
            raise ValueError("El cursor debe ser un entero no negativo.")

        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(
                SQL_LEER_LOTE, (usuario_id, desde, LIMITE_CONSULTA)
            ).fetchall()

        eventos = tuple(_evento_desde_fila(fila) for fila in filas[:TAMANO_LOTE])
        cursor = eventos[-1].posicion if eventos else desde
        return LoteCambios(
            eventos=eventos,
            cursor=cursor,
            hay_mas=len(filas) > TAMANO_LOTE,
        )


def _evento_desde_fila(fila: dict[str, Any]) -> Evento:
    return Evento(
        id=fila["id"],
        posicion=fila["posicion"],
        usuario_id=fila["usuario_id"],
        grupo_id=fila["grupo_id"],
        nombre_grupo=fila["nombre_grupo"],
        gasto_id=fila["gasto_id"],
        concepto=fila["concepto"],
        fecha=fila["fecha"],
        moneda=fila["moneda"],
        monto=fila["monto"],
        incluye_redondeo=fila["incluye_redondeo"],
        creado_en=fila["creado_en"],
    )
