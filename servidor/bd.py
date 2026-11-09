"""Almacén de usuarios en SQLite.

Solo guarda identidad: id, nombre de usuario, nombre visible, correo, hash de
la contraseña y fecha de alta. Nunca cuentas, movimientos ni saldos (INV-16).

La ruta del archivo sale de ``BD_USUARIOS_PATH`` (``usuarios.db`` por defecto,
en la carpeta desde la que se arranca el servidor). Se abre una conexión por
operación, así que no se comparte estado entre peticiones ni entre pruebas.
"""

import os
import sqlite3
import uuid
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone

RUTA_POR_DEFECTO = "usuarios.db"
FORMATO_FECHA = "%Y-%m-%dT%H:%M:%SZ"

_ESQUEMA = """
CREATE TABLE IF NOT EXISTS usuarios (
    id              TEXT PRIMARY KEY,
    nombre_usuario  TEXT NOT NULL UNIQUE,
    nombre_visible  TEXT NOT NULL,
    correo          TEXT NOT NULL UNIQUE,
    hash_contrasena TEXT NOT NULL,
    fecha_alta      TEXT NOT NULL
)
"""


@dataclass(frozen=True)
class Usuario:
    id: str
    nombre_usuario: str
    nombre_visible: str
    correo: str
    hash_contrasena: str
    fecha_alta: str


class UsuarioDuplicado(Exception):
    """El nombre de usuario o el correo ya existen. ``campos`` dice cuáles."""

    def __init__(self, campos: list[str]) -> None:
        super().__init__(", ".join(campos))
        self.campos = campos


def ruta_bd() -> str:
    return os.environ.get("BD_USUARIOS_PATH") or RUTA_POR_DEFECTO


@contextmanager
def _conexion() -> Iterator[sqlite3.Connection]:
    conexion = sqlite3.connect(ruta_bd())
    try:
        conexion.row_factory = sqlite3.Row
        conexion.execute(_ESQUEMA)
        with conexion:  # confirma o deshace la transacción
            yield conexion
    finally:
        conexion.close()


def inicializar() -> None:
    """Crea el archivo y la tabla si no existen."""
    with _conexion():
        pass


def _a_usuario(fila: sqlite3.Row | None) -> Usuario | None:
    if fila is None:
        return None
    return Usuario(**dict(fila))


def _campos_ocupados(
    conexion: sqlite3.Connection, nombre_usuario: str, correo: str
) -> list[str]:
    campos = []
    if conexion.execute(
        "SELECT 1 FROM usuarios WHERE nombre_usuario = ?", (nombre_usuario,)
    ).fetchone():
        campos.append("nombre_usuario")
    if conexion.execute(
        "SELECT 1 FROM usuarios WHERE correo = ?", (correo,)
    ).fetchone():
        campos.append("correo")
    return campos


def crear_usuario(
    nombre_usuario: str, nombre_visible: str, correo: str, hash_contrasena: str
) -> Usuario:
    """Inserta un usuario. El correo debe llegar ya normalizado.

    Lanza ``UsuarioDuplicado`` con todos los campos repetidos.
    """
    usuario = Usuario(
        id=str(uuid.uuid4()),
        nombre_usuario=nombre_usuario,
        nombre_visible=nombre_visible,
        correo=correo,
        hash_contrasena=hash_contrasena,
        fecha_alta=datetime.now(timezone.utc).strftime(FORMATO_FECHA),
    )
    with _conexion() as conexion:
        ocupados = _campos_ocupados(conexion, nombre_usuario, correo)
        if ocupados:
            raise UsuarioDuplicado(ocupados)
        try:
            conexion.execute(
                "INSERT INTO usuarios (id, nombre_usuario, nombre_visible, correo, "
                "hash_contrasena, fecha_alta) VALUES (?, ?, ?, ?, ?, ?)",
                (
                    usuario.id,
                    usuario.nombre_usuario,
                    usuario.nombre_visible,
                    usuario.correo,
                    usuario.hash_contrasena,
                    usuario.fecha_alta,
                ),
            )
        except sqlite3.IntegrityError as error:
            # Otra petición registró los mismos datos entre la consulta y el
            # INSERT. La restricción UNIQUE lo detecta.
            ocupados = _campos_ocupados(conexion, nombre_usuario, correo)
            raise UsuarioDuplicado(ocupados or ["nombre_usuario"]) from error
    return usuario


def buscar_por_correo(correo: str) -> Usuario | None:
    with _conexion() as conexion:
        fila = conexion.execute(
            "SELECT * FROM usuarios WHERE correo = ?", (correo,)
        ).fetchone()
    return _a_usuario(fila)


def buscar_por_id(id_usuario: str) -> Usuario | None:
    with _conexion() as conexion:
        fila = conexion.execute(
            "SELECT * FROM usuarios WHERE id = ?", (id_usuario,)
        ).fetchone()
    return _a_usuario(fila)
