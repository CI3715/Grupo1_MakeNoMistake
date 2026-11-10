"""Repositorio de personas usuarias sobre PostgreSQL.

Aquí vive todo el SQL relacionado con la tabla `usuarios`. El resto del
servidor no debería conocer detalles del motor: si algún día cambia la
conexión, solo se toca este archivo y `base_datos.py`.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any
from uuid import UUID, uuid4

from psycopg import Connection, errors

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
SQL_EXISTE_CORREO = "SELECT 1 FROM usuarios WHERE lower(correo) = %s LIMIT 1"
SQL_EXISTE_NOMBRE_USUARIO = "SELECT 1 FROM usuarios WHERE nombre_usuario = %s LIMIT 1"


class UsuarioDuplicado(Exception):
    """El nombre de usuario, el correo o ambos ya existen.

    `campos` enumera todos los campos en conflicto para que la capa que
    atiende la petición responda 409 sin mirar detalles del driver. `campo`
    se conserva para las comprobaciones de un único campo.
    """

    def __init__(self, campos: Sequence[str]) -> None:
        self.campos = list(campos)
        super().__init__(", ".join(self.campos))

    @property
    def campo(self) -> str:
        return self.campos[0]


class CorreoYaRegistrado(UsuarioDuplicado):
    def __init__(self) -> None:
        super().__init__(["correo"])


class NombreUsuarioYaRegistrado(UsuarioDuplicado):
    def __init__(self) -> None:
        super().__init__(["nombre_usuario"])


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

        El correo se guarda en minúsculas. Si el correo, el nombre de usuario o
        ambos ya existen, se lanza ``UsuarioDuplicado`` con todos los campos en
        conflicto para que la capa de arriba responda 409.
        """

        usuario = Usuario(
            id=uuid4(),
            nombre_usuario=nombre_usuario.strip(),
            nombre_visible=nombre_visible.strip(),
            correo=correo.strip().lower(),
            hash_contrasena=hash_contrasena,
            fecha_alta=datetime.now(timezone.utc).replace(microsecond=0),
        )

        with self._base_datos.conexion() as conexion:
            ocupados = _campos_ocupados(
                conexion, usuario.nombre_usuario, usuario.correo
            )
            if ocupados:
                raise _error_duplicado(ocupados)

            try:
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
                # Otra petición registró los mismos datos entre la consulta y
                # el INSERT. Los índices únicos lo detectan.
                ocupados = _campos_ocupados(
                    conexion, usuario.nombre_usuario, usuario.correo
                )
                if not ocupados:
                    ocupados = [_campo_de(error)]
                raise _error_duplicado(ocupados) from error

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


def _campos_ocupados(
    conexion: Connection[dict[str, Any]], nombre_usuario: str, correo: str
) -> list[str]:
    """Devuelve los campos que ya existen en la tabla."""

    campos: list[str] = []
    if conexion.execute(SQL_EXISTE_NOMBRE_USUARIO, (nombre_usuario,)).fetchone():
        campos.append("nombre_usuario")
    if conexion.execute(SQL_EXISTE_CORREO, (correo.lower(),)).fetchone():
        campos.append("correo")
    return campos


def _error_duplicado(campos: list[str]) -> UsuarioDuplicado:
    """Elige la excepción concreta según los campos en conflicto."""

    if campos == ["correo"]:
        return CorreoYaRegistrado()
    if campos == ["nombre_usuario"]:
        return NombreUsuarioYaRegistrado()
    return UsuarioDuplicado(campos)


def _campo_de(error: errors.UniqueViolation) -> str:
    """Traduce la restricción violada al nombre del campo."""

    restriccion = error.diag.constraint_name
    if restriccion == "usuarios_correo_unico":
        return "correo"
    if restriccion == "usuarios_nombre_usuario_unico":
        return "nombre_usuario"
    raise error
