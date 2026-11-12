"""Repositorio de grupos y miembros sobre PostgreSQL (Entrega 6).

Un grupo nace con su creador como miembro administrador y aceptado
(RF-COM-01). Nadie más entra sin aceptar la invitación (RF-COM-02): mientras
el estado sea `invitado`, la persona no ve gastos ni balances (INV-17).

La tabla solo admite los estados ``invitado`` y ``aceptado``. Rechazar una
invitación borra la fila; queda anotado como deuda en ADR-011 porque la
especificación no permite un estado ``rechazado``.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime
from enum import StrEnum
from typing import Any
from uuid import UUID, uuid4

from psycopg import errors

from base_datos import BaseDatos

COLUMNAS_GRUPO = (
    "id, nombre, descripcion, moneda, actividad, fecha_actividad, "
    "creado_por, fecha_creacion"
)
COLUMNAS_MIEMBRO = (
    "grupo_id, usuario_id, rol, estado, invitado_por, fecha_invitacion, fecha_respuesta"
)

SQL_INSERTAR_GRUPO = f"""
    INSERT INTO grupos ({COLUMNAS_GRUPO})
    VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
    RETURNING {COLUMNAS_GRUPO}
"""

SQL_INSERTAR_MIEMBRO = f"""
    INSERT INTO miembros ({COLUMNAS_MIEMBRO})
    VALUES (%s, %s, %s, %s, %s, %s, %s)
    RETURNING {COLUMNAS_MIEMBRO}
"""

SQL_BUSCAR_GRUPO = f"""
    SELECT {COLUMNAS_GRUPO} FROM grupos WHERE id = %s
"""

SQL_BUSCAR_MIEMBRO = f"""
    SELECT {COLUMNAS_MIEMBRO}
    FROM miembros
    WHERE grupo_id = %s AND usuario_id = %s
"""

SQL_LISTAR_DE = """
    SELECT g.id, g.nombre, g.descripcion, g.moneda, g.actividad,
           g.fecha_actividad, g.creado_por, g.fecha_creacion,
           m.rol, m.estado, u.nombre_usuario AS nombre_creador
    FROM grupos g
    JOIN miembros m ON m.grupo_id = g.id AND m.usuario_id = %s
    JOIN usuarios u ON u.id = g.creado_por
    WHERE m.estado IN ('invitado', 'aceptado')
    ORDER BY g.fecha_creacion DESC, g.id
"""

SQL_LISTAR_MIEMBROS = """
    SELECT m.usuario_id, u.nombre_usuario, u.nombre_visible,
           m.rol, m.estado
    FROM miembros m
    JOIN usuarios u ON u.id = m.usuario_id
    WHERE m.grupo_id = %s
    ORDER BY (m.estado = 'aceptado') DESC, u.nombre_usuario
"""

SQL_EXISTE_GRUPO = "SELECT 1 FROM grupos WHERE id = %s"
SQL_EXISTE_USUARIO = "SELECT 1 FROM usuarios WHERE id = %s"
SQL_ACEPTAR_INVITACION = f"""
    UPDATE miembros
    SET estado = 'aceptado', fecha_respuesta = %s
    WHERE grupo_id = %s AND usuario_id = %s AND estado = 'invitado'
    RETURNING {COLUMNAS_MIEMBRO}
"""
SQL_RECHAZAR_INVITACION = """
    DELETE FROM miembros
    WHERE grupo_id = %s AND usuario_id = %s AND estado = 'invitado'
    RETURNING 1
"""


class Rol(StrEnum):
    """Rol de una persona dentro de un grupo."""

    ADMINISTRADOR = "administrador"
    MIEMBRO = "miembro"


class EstadoMiembro(StrEnum):
    """Estado de la pertenencia a un grupo."""

    INVITADO = "invitado"
    ACEPTADO = "aceptado"


@dataclass(frozen=True)
class Grupo:
    """Fila de la tabla `grupos`."""

    id: UUID
    nombre: str
    descripcion: str | None
    moneda: str
    actividad: str | None
    fecha_actividad: date | None
    creado_por: UUID
    fecha_creacion: datetime


@dataclass(frozen=True)
class Miembro:
    """Fila de la tabla `miembros`."""

    grupo_id: UUID
    usuario_id: UUID
    rol: Rol
    estado: EstadoMiembro
    invitado_por: UUID | None
    fecha_invitacion: datetime
    fecha_respuesta: datetime | None


@dataclass(frozen=True)
class GrupoVista:
    """Grupo junto con la pertenencia de quien consulta."""

    grupo: Grupo
    mi_rol: Rol
    mi_estado: EstadoMiembro
    nombre_creador: str


@dataclass(frozen=True)
class MiembroVista:
    """Miembro con los datos públicos de su usuario (nunca el correo)."""

    usuario_id: UUID
    nombre_usuario: str
    nombre_visible: str
    rol: Rol
    estado: EstadoMiembro


class GrupoNoEncontrado(Exception):
    """No existe el grupo o quien consulta no es miembro."""


class UsuarioNoEncontrado(Exception):
    """No existe la persona que se quiere invitar."""


class YaEsMiembro(Exception):
    """La persona ya pertenece al grupo o tiene una invitación pendiente."""


class InvitacionDeGrupoNoPendiente(Exception):
    """No hay una invitación pendiente para esa persona en ese grupo."""


class GrupoInvalido(Exception):
    """El grupo incumple una regla de sus datos."""

    def __init__(self, campo: str, mensaje: str) -> None:
        self.campo = campo
        self.mensaje = mensaje
        super().__init__(mensaje)


class RepositorioGrupos:
    """Consultas de las tablas `grupos` y `miembros`."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def crear(
        self,
        *,
        creado_por: UUID,
        nombre: str,
        descripcion: str | None,
        moneda: str,
        actividad: str | None,
        fecha_actividad: date | None,
        ahora: datetime,
    ) -> Grupo:
        """Crea el grupo y su miembro administrador en una sola transacción.

        La moneda llega ya normalizada a mayúsculas y con tres letras.
        """

        nombre_limpio = nombre.strip()
        if not nombre_limpio:
            raise GrupoInvalido("nombre", "El nombre del grupo no puede estar vacío.")
        moneda_limpia = moneda.strip()
        if len(moneda_limpia) != 3 or not moneda_limpia.isalpha():
            raise GrupoInvalido(
                "moneda",
                "La moneda debe ser un código ISO de tres letras.",
            )

        fecha = _sin_microsegundos(ahora)
        grupo = Grupo(
            id=uuid4(),
            nombre=nombre_limpio,
            descripcion=_opcional(descripcion),
            moneda=moneda_limpia,
            actividad=_opcional(actividad),
            fecha_actividad=fecha_actividad,
            creado_por=creado_por,
            fecha_creacion=fecha,
        )

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                SQL_INSERTAR_GRUPO,
                (
                    grupo.id,
                    grupo.nombre,
                    grupo.descripcion,
                    grupo.moneda,
                    grupo.actividad,
                    grupo.fecha_actividad,
                    grupo.creado_por,
                    grupo.fecha_creacion,
                ),
            ).fetchone()
            assert fila is not None
            conexion.execute(
                SQL_INSERTAR_MIEMBRO,
                (
                    grupo.id,
                    creado_por,
                    Rol.ADMINISTRADOR.value,
                    EstadoMiembro.ACEPTADO.value,
                    None,
                    fecha,
                    fecha,
                ),
            )

        return _grupo_desde_fila(fila)

    def buscar_por_id(self, id_grupo: UUID) -> Grupo | None:
        """Devuelve el grupo o `None` si no existe."""

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(SQL_BUSCAR_GRUPO, (id_grupo,)).fetchone()

        return None if fila is None else _grupo_desde_fila(fila)

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None:
        """Devuelve la pertenencia o `None` si la persona no está en el grupo."""

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                SQL_BUSCAR_MIEMBRO, (id_grupo, usuario_id)
            ).fetchone()

        return None if fila is None else _miembro_desde_fila(fila)

    def listar_de(self, usuario_id: UUID) -> list[GrupoVista]:
        """Grupos aceptados e invitaciones pendientes de una persona."""

        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(SQL_LISTAR_DE, (usuario_id,)).fetchall()

        return [
            GrupoVista(
                grupo=_grupo_desde_fila(fila),
                mi_rol=Rol(fila["rol"]),
                mi_estado=EstadoMiembro(fila["estado"]),
                nombre_creador=fila["nombre_creador"],
            )
            for fila in filas
        ]

    def listar_miembros(self, id_grupo: UUID) -> list[MiembroVista]:
        """Miembros e invitados de un grupo."""

        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(SQL_LISTAR_MIEMBROS, (id_grupo,)).fetchall()

        return [
            MiembroVista(
                usuario_id=fila["usuario_id"],
                nombre_usuario=fila["nombre_usuario"],
                nombre_visible=fila["nombre_visible"],
                rol=Rol(fila["rol"]),
                estado=EstadoMiembro(fila["estado"]),
            )
            for fila in filas
        ]

    def invitar(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        invitado_por: UUID,
        ahora: datetime,
    ) -> Miembro:
        """Invita a una persona al grupo en estado `invitado`.

        Si ya pertenece al grupo o ya tiene una invitación pendiente se lanza
        `YaEsMiembro`.
        """

        fecha = _sin_microsegundos(ahora)

        with self._base_datos.conexion() as conexion:
            if conexion.execute(SQL_EXISTE_GRUPO, (id_grupo,)).fetchone() is None:
                raise GrupoNoEncontrado("No existe ese grupo.")
            if conexion.execute(SQL_EXISTE_USUARIO, (usuario_id,)).fetchone() is None:
                raise UsuarioNoEncontrado("No existe la persona que se quiere invitar.")
            if (
                conexion.execute(SQL_BUSCAR_MIEMBRO, (id_grupo, usuario_id)).fetchone()
                is not None
            ):
                raise YaEsMiembro("Esa persona ya pertenece al grupo o está invitada.")

            try:
                fila = conexion.execute(
                    SQL_INSERTAR_MIEMBRO,
                    (
                        id_grupo,
                        usuario_id,
                        Rol.MIEMBRO.value,
                        EstadoMiembro.INVITADO.value,
                        invitado_por,
                        fecha,
                        None,
                    ),
                ).fetchone()
            except errors.UniqueViolation as error:
                raise YaEsMiembro(
                    "Esa persona ya pertenece al grupo o está invitada."
                ) from error
            except errors.ForeignKeyViolation as error:
                raise GrupoNoEncontrado("No existe ese grupo.") from error

        assert fila is not None
        return _miembro_desde_fila(fila)

    def aceptar_invitacion(
        self, *, id_grupo: UUID, usuario_id: UUID, ahora: datetime
    ) -> Miembro:
        """Pasa la pertenencia de `invitado` a `aceptado`.

        Solo el invitado puede hacerlo; si no hay invitación pendiente se
        lanza `InvitacionDeGrupoNoPendiente`.
        """

        fecha = _sin_microsegundos(ahora)

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                SQL_ACEPTAR_INVITACION, (fecha, id_grupo, usuario_id)
            ).fetchone()

        if fila is None:
            raise InvitacionDeGrupoNoPendiente(
                "No hay una invitación pendiente para esa persona."
            )
        return _miembro_desde_fila(fila)

    def rechazar_invitacion(self, *, id_grupo: UUID, usuario_id: UUID) -> None:
        """Elimina la fila de la persona rechazada.

        La tabla solo admite los estados `invitado` y `aceptado`, así que un
        rechazo no deja fila.
        """

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                SQL_RECHAZAR_INVITACION, (id_grupo, usuario_id)
            ).fetchone()

        if fila is None:
            raise InvitacionDeGrupoNoPendiente(
                "No hay una invitación pendiente para esa persona."
            )


def _opcional(texto: str | None) -> str | None:
    if texto is None:
        return None
    limpio = texto.strip()
    return limpio or None


def _sin_microsegundos(ahora: datetime) -> datetime:
    return ahora.replace(microsecond=0)


def _grupo_desde_fila(fila: dict[str, Any]) -> Grupo:
    return Grupo(
        id=fila["id"],
        nombre=fila["nombre"],
        descripcion=fila["descripcion"],
        moneda=fila["moneda"],
        actividad=fila["actividad"],
        fecha_actividad=fila["fecha_actividad"],
        creado_por=fila["creado_por"],
        fecha_creacion=fila["fecha_creacion"],
    )


def _miembro_desde_fila(fila: dict[str, Any]) -> Miembro:
    return Miembro(
        grupo_id=fila["grupo_id"],
        usuario_id=fila["usuario_id"],
        rol=Rol(fila["rol"]),
        estado=EstadoMiembro(fila["estado"]),
        invitado_por=fila["invitado_por"],
        fecha_invitacion=fila["fecha_invitacion"],
        fecha_respuesta=fila["fecha_respuesta"],
    )
