"""Gestión de persistencia SQLite para los usuarios del servidor.

Cumple con el invariante INV-16: solo almacena información de identidad
(usuarios), nunca datos financieros como cuentas, saldos, movimientos o tasas.
"""

import sqlite3
from pathlib import Path
from typing import Any

from config import obtener_ruta_bd


def conectar(ruta: Path | None = None) -> sqlite3.Connection:
    """Abre y configura una conexión con la base de datos SQLite."""
    destino = ruta or obtener_ruta_bd()
    if str(destino.parent) not in ("", "."):
        destino.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(destino)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON;")
    return conn


def inicializar_base_datos(ruta: Path | None = None) -> None:
    """Crea la tabla de usuarios si aún no existe."""
    with conectar(ruta) as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS usuarios (
                id TEXT PRIMARY KEY,
                nombre_usuario TEXT NOT NULL UNIQUE,
                nombre_visible TEXT NOT NULL,
                correo TEXT NOT NULL UNIQUE COLLATE NOCASE,
                hash_contrasena TEXT NOT NULL,
                fecha_alta TEXT NOT NULL
            );
            """
        )
        conn.commit()


def buscar_usuario_por_id(
    usuario_id: str, ruta: Path | None = None
) -> dict[str, Any] | None:
    """Busca un usuario por su identificador único."""
    inicializar_base_datos(ruta)
    with conectar(ruta) as conn:
        cursor = conn.execute(
            """
            SELECT id, nombre_usuario, nombre_visible,
                   correo, hash_contrasena, fecha_alta
            FROM usuarios
            WHERE id = ?;
            """,
            (usuario_id,),
        )
        fila = cursor.fetchone()
        if fila is None:
            return None
        return dict(fila)


def buscar_usuario_por_correo(
    correo: str, ruta: Path | None = None
) -> dict[str, Any] | None:
    """Busca un usuario por su correo electrónico (insensible a mayúsculas)."""
    inicializar_base_datos(ruta)
    with conectar(ruta) as conn:
        cursor = conn.execute(
            """
            SELECT id, nombre_usuario, nombre_visible,
                   correo, hash_contrasena, fecha_alta
            FROM usuarios
            WHERE lower(correo) = lower(?);
            """,
            (correo.strip(),),
        )
        fila = cursor.fetchone()
        if fila is None:
            return None
        return dict(fila)


def buscar_usuario_por_nombre(
    nombre_usuario: str, ruta: Path | None = None
) -> dict[str, Any] | None:
    """Busca un usuario por su nombre de usuario."""
    inicializar_base_datos(ruta)
    with conectar(ruta) as conn:
        cursor = conn.execute(
            """
            SELECT id, nombre_usuario, nombre_visible,
                   correo, hash_contrasena, fecha_alta
            FROM usuarios
            WHERE nombre_usuario = ?;
            """,
            (nombre_usuario,),
        )
        fila = cursor.fetchone()
        if fila is None:
            return None
        return dict(fila)


def insertar_usuario(
    id_usuario: str,
    nombre_usuario: str,
    nombre_visible: str,
    correo: str,
    hash_contrasena: str,
    fecha_alta: str,
    ruta: Path | None = None,
) -> dict[str, Any]:
    """Inserta un nuevo usuario en la base de datos y retorna sus datos públicos."""
    inicializar_base_datos(ruta)
    correo_limpio = correo.strip().lower()
    with conectar(ruta) as conn:
        conn.execute(
            """
            INSERT INTO usuarios (
                id, nombre_usuario, nombre_visible,
                correo, hash_contrasena, fecha_alta
            )
            VALUES (?, ?, ?, ?, ?, ?);
            """,
            (
                id_usuario,
                nombre_usuario,
                nombre_visible,
                correo_limpio,
                hash_contrasena,
                fecha_alta,
            ),
        )
        conn.commit()
    return {
        "id": id_usuario,
        "nombre_usuario": nombre_usuario,
        "nombre_visible": nombre_visible,
        "correo": correo_limpio,
        "fecha_alta": fecha_alta,
    }
