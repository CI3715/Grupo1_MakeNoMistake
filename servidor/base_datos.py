"""Conexión a PostgreSQL y creación del esquema del servidor.

El DDL vive en `esquema.py`; aquí solo se abre la conexión y se aplica. Las
sentencias son idempotentes, así que `crear_esquema` se puede llamar en cada
arranque.
"""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
from typing import Any

import psycopg
from psycopg import Connection
from psycopg.rows import dict_row

from esquema import SENTENCIAS_ESQUEMA


class BaseDatos:
    """Fábrica de conexiones a la base de datos del servidor.

    Se abre una conexión por operación. Para el volumen de este servidor
    (registro, inicio de sesión y perfil) es suficiente y evita compartir una
    conexión entre los hilos que usa FastAPI.
    """

    def __init__(self, url: str) -> None:
        self._url = url

    @contextmanager
    def conexion(self) -> Iterator[Connection[dict[str, Any]]]:
        with psycopg.connect(self._url, row_factory=dict_row) as conexion:
            yield conexion

    def crear_esquema(self) -> None:
        """Crea la tabla y los índices si no existen."""

        with self.conexion() as conexion:
            for sentencia in SENTENCIAS_ESQUEMA:
                conexion.execute(sentencia)
