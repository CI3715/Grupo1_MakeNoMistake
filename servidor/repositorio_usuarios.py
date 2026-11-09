"""Repositorio de personas usuarias sobre PostgreSQL.

Aquí vive todo el SQL relacionado con la tabla `usuarios`. El resto del
servidor no debería conocer detalles del motor: si algún día cambia la
conexión, solo se toca este archivo y `base_datos.py`.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, NoReturn
from uuid import UUID, uuid4

from psycopg import errors

from base_datos import BaseDatos

CAMPOS = "id, nombre_usuario, nombre_visible, correo, hash_contrasena, fecha_alta"

SQL_INSERTAR = f"""
    INSERT INTO usuarios ({CAMPOS})
    VALUES (%s, %s, %s, %s, %s, %s)
"""
SQL_BUSCAR_POR_CORREO = f"""
    SELECT {CAMPOS} FROM usuarios WHERE lower(correo) = %s
"""
SQL_BUSCAR_POR_NOMBRE_USUARIO = f"""
    SELECT {CAMPOS} FROM usuarios WHERE nombre_usuario = %s
"""
SQL_BUSCAR_POR_ID = f"""
    SELECT {CAMPOS} FROM usuarios WHERE id = %s
"""


class UsuarioDuplicado(Exception):
    """El nombre de usuario o el correo ya existen.

    `campo` permite que el comando que atiende la petición sepa qué campo
    informar en la respuesta 409, sin mirar detalles del driver.
    """

    campo: str


class CorreoYaRegistrado(UsuarioDuplicado):
    campo = "correo"


class NombreUsuarioYaRegistrado(UsuarioDuplicado):
    campo = "nombre_usuario"


@dataclass(frozen=True)
class Usuario:
    """Fila de la tabla `usuarios`, incluido el hash para validar el login."""

    id: UUID
    nombre_usuario: str
    nombre_visible: str
    correo: str
    hash_contrasena: str
    fecha_alta: datetime


class RepositorioUsuarios:
    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def crear(
        self,
        *,
        nombre_usuario: str,
        nombre_visible: str,
        correo: str,
        hash_contrasena: str,
    ) -> Usuario:
        """Inserta un usuario y devuelve la fila creada.

        El correo se guarda en minúsculas. Si el correo o el nombre de usuario
        ya existen, se lanza la excepción correspondiente para que la capa de
        arriba responda 409 indicando el campo.
        """

        usuario = Usuario(
            id=uuid4(),
            nombre_usuario=nombre_usuario.strip(),
            nombre_visible=nombre_visible.strip(),
            correo=correo.strip().lower(),
            hash_contrasena=hash_contrasena,
            fecha_alta=datetime.now(timezone.utc).replace(microsecond=0),
        )

        try:
            with self._base_datos.conexion() as conexion:
                conexion.execute(
                    SQL_INSERTAR,
                    (
                        usuario.id,
                        usuario.nombre_usuario,
                        usuario.nombre_visible,
                        usuario.correo,
                        usuario.hash_contrasena,
                        usuario.fecha_alta,
                    ),
                )
        except errors.UniqueViolation as error:
            _traducir_duplicado(error)

        return usuario

    def buscar_por_correo(self, correo: str) -> Usuario | None:
        """Busca sin distinguir mayúsculas en el correo."""

        return self._buscar_uno(SQL_BUSCAR_POR_CORREO, (correo.strip().lower(),))

    def buscar_por_nombre_usuario(self, nombre_usuario: str) -> Usuario | None:
        return self._buscar_uno(
            SQL_BUSCAR_POR_NOMBRE_USUARIO, (nombre_usuario.strip(),)
        )

    def buscar_por_id(self, id_usuario: UUID) -> Usuario | None:
        return self._buscar_uno(SQL_BUSCAR_POR_ID, (id_usuario,))

    def _buscar_uno(self, sql: str, parametros: tuple[object, ...]) -> Usuario | None:
        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(sql, parametros).fetchone()

        if fila is None:
            return None

        return _usuario_desde_fila(fila)


def _usuario_desde_fila(fila: dict[str, Any]) -> Usuario:
    return Usuario(
        id=fila["id"],
        nombre_usuario=fila["nombre_usuario"],
        nombre_visible=fila["nombre_visible"],
        correo=fila["correo"],
        hash_contrasena=fila["hash_contrasena"],
        fecha_alta=fila["fecha_alta"],
    )


def _traducir_duplicado(error: errors.UniqueViolation) -> NoReturn:
    """Convierte la violación de unicidad de PostgreSQL en un error de campo."""

    restriccion = error.diag.constraint_name
    if restriccion == "usuarios_correo_unico":
        raise CorreoYaRegistrado("El correo ya está registrado.") from error
    if restriccion == "usuarios_nombre_usuario_unico":
        raise NombreUsuarioYaRegistrado(
            "El nombre de usuario ya está registrado."
        ) from error

    raise error
