"""Conexión a PostgreSQL y creación del esquema del servidor."""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
from typing import Any

import psycopg
from psycopg import Connection
from psycopg.rows import dict_row

# El esquema se crea al arrancar. Son sentencias idempotentes: si las tablas y
# los índices ya existen, no hacen nada. No se guardan datos financieros aquí
# (INV-16): solo la identidad de las personas usuarias.
SENTENCIAS_ESQUEMA = (
    """
    CREATE TABLE IF NOT EXISTS usuarios (
        id              UUID PRIMARY KEY,
        nombre_usuario  VARCHAR(30) NOT NULL,
        nombre_visible  VARCHAR(50) NOT NULL,
        correo          VARCHAR(254) NOT NULL,
        hash_contrasena TEXT NOT NULL,
        fecha_alta      TIMESTAMPTZ NOT NULL,
        CONSTRAINT usuarios_correo_en_minusculas CHECK (correo = lower(correo)),
        CONSTRAINT usuarios_nombre_en_minusculas
            CHECK (nombre_usuario = lower(nombre_usuario))
    )
    """,
    """
    CREATE UNIQUE INDEX IF NOT EXISTS usuarios_correo_unico
        ON usuarios (lower(correo))
    """,
    """
    CREATE UNIQUE INDEX IF NOT EXISTS usuarios_nombre_usuario_unico
        ON usuarios (nombre_usuario)
    """,
)


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
