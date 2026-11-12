"""Firmas e implementación del repositorio de grupos y miembros (Entrega 6).

Un grupo nace con su creador como miembro administrador y aceptado
(RF-COM-01). Nadie más entra sin aceptar la invitación (RF-COM-02): mientras
el estado sea `invitado`, la persona no ve gastos ni balances (INV-17).
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime
from enum import StrEnum
from typing import Any
from uuid import UUID, uuid4

from psycopg import errors

from base_datos import BaseDatos


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


class YaEsMiembro(Exception):
    """La persona ya pertenece al grupo o tiene una invitación pendiente."""


class InvitacionDeGrupoNoPendiente(Exception):
    """No hay una invitación pendiente para esa persona en ese grupo."""


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
        """Crea el grupo y su miembro administrador en una sola transacción."""
        grupo = Grupo(
            id=uuid4(),
            nombre=nombre.strip(),
            descripcion=descripcion.strip() if descripcion else None,
            moneda=moneda.strip().upper(),
            actividad=actividad.strip() if actividad else None,
            fecha_actividad=fecha_actividad,
            creado_por=creado_por,
            fecha_creacion=ahora,
        )

        with self._base_datos.conexion() as conexion:
            conexion.execute(
                """
                INSERT INTO grupos (
                    id, nombre, descripcion, moneda, actividad,
                    fecha_actividad, creado_por, fecha_creacion
                )
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                """,
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
            )
            conexion.execute(
                """
                INSERT INTO miembros (
                    grupo_id, usuario_id, rol, estado,
                    invitado_por, fecha_invitacion, fecha_respuesta
                )
                VALUES (%s, %s, %s, %s, %s, %s, %s)
                """,
                (
                    grupo.id,
                    creado_por,
                    Rol.ADMINISTRADOR.value,
                    EstadoMiembro.ACEPTADO.value,
                    None,
                    ahora,
                    ahora,
                ),
            )
        return grupo

    def buscar_por_id(self, id_grupo: UUID) -> Grupo | None:
        """Devuelve el grupo o `None` si no existe."""
        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                """
                SELECT id, nombre, descripcion, moneda, actividad,
                       fecha_actividad, creado_por, fecha_creacion
                FROM grupos WHERE id = %s
                """,
                (id_grupo,),
            ).fetchone()
            if fila is None:
                return None
            return _grupo_desde_fila(fila)

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None:
        """Devuelve la pertenencia o `None` si la persona no está en el grupo."""
        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                """
                SELECT grupo_id, usuario_id, rol, estado,
                       invitado_por, fecha_invitacion, fecha_respuesta
                FROM miembros WHERE grupo_id = %s AND usuario_id = %s
                """,
                (id_grupo, usuario_id),
            ).fetchone()
            if fila is None:
                return None
            return _miembro_desde_fila(fila)

    def listar_de(self, usuario_id: UUID) -> list[GrupoVista]:
        """Grupos aceptados e invitaciones pendientes de una persona."""
        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(
                """
                SELECT g.id, g.nombre, g.descripcion, g.moneda, g.actividad,
                       g.fecha_actividad, g.creado_por, g.fecha_creacion,
                       m.rol, m.estado, u.nombre_visible
                FROM miembros m
                JOIN grupos g ON g.id = m.grupo_id
                JOIN usuarios u ON u.id = g.creado_por
                WHERE m.usuario_id = %s
                ORDER BY g.fecha_creacion DESC
                """,
                (usuario_id,),
            ).fetchall()
            return [
                GrupoVista(
                    grupo=_grupo_desde_fila(f),
                    mi_rol=Rol(f["rol"]),
                    mi_estado=EstadoMiembro(f["estado"]),
                    nombre_creador=f["nombre_visible"],
                )
                for f in filas
            ]

    def listar_miembros(self, id_grupo: UUID) -> list[MiembroVista]:
        """Miembros e invitados de un grupo."""
        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(
                """
                SELECT m.usuario_id, u.nombre_usuario, u.nombre_visible,
                       m.rol, m.estado
                FROM miembros m
                JOIN usuarios u ON u.id = m.usuario_id
                WHERE m.grupo_id = %s
                ORDER BY u.nombre_usuario ASC
                """,
                (id_grupo,),
            ).fetchall()
            return [
                MiembroVista(
                    usuario_id=f["usuario_id"],
                    nombre_usuario=f["nombre_usuario"],
                    nombre_visible=f["nombre_visible"],
                    rol=Rol(f["rol"]),
                    estado=EstadoMiembro(f["estado"]),
                )
                for f in filas
            ]

    def invitar(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        invitado_por: UUID,
        ahora: datetime,
    ) -> Miembro:
        """Invita a una persona al grupo en estado `invitado`."""
        with self._base_datos.conexion() as conexion:
            try:
                conexion.execute(
                    """
                    INSERT INTO miembros (
                        grupo_id, usuario_id, rol, estado,
                        invitado_por, fecha_invitacion, fecha_respuesta
                    )
                    VALUES (%s, %s, %s, %s, %s, %s, NULL)
                    """,
                    (
                        id_grupo,
                        usuario_id,
                        Rol.MIEMBRO.value,
                        EstadoMiembro.INVITADO.value,
                        invitado_por,
                        ahora,
                    ),
                )
            except errors.UniqueViolation as exc:
                raise YaEsMiembro() from exc

        return Miembro(
            grupo_id=id_grupo,
            usuario_id=usuario_id,
            rol=Rol.MIEMBRO,
            estado=EstadoMiembro.INVITADO,
            invitado_por=invitado_por,
            fecha_invitacion=ahora,
            fecha_respuesta=None,
        )

    def aceptar_invitacion(
        self, *, id_grupo: UUID, usuario_id: UUID, ahora: datetime
    ) -> Miembro:
        """Pasa la pertenencia de `invitado` a `aceptado`."""
        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                """
                UPDATE miembros
                SET estado = %s, fecha_respuesta = %s
                WHERE grupo_id = %s AND usuario_id = %s AND estado = %s
                RETURNING rol, invitado_por, fecha_invitacion
                """,
                (
                    EstadoMiembro.ACEPTADO.value,
                    ahora,
                    id_grupo,
                    usuario_id,
                    EstadoMiembro.INVITADO.value,
                ),
            ).fetchone()
            if fila is None:
                raise InvitacionDeGrupoNoPendiente()
            return Miembro(
                grupo_id=id_grupo,
                usuario_id=usuario_id,
                rol=Rol(fila["rol"]),
                estado=EstadoMiembro.ACEPTADO,
                invitado_por=fila["invitado_por"],
                fecha_invitacion=fila["fecha_invitacion"],
                fecha_respuesta=ahora,
            )

    def rechazar_invitacion(self, *, id_grupo: UUID, usuario_id: UUID) -> None:
        """Elimina la fila de la persona rechazada."""
        with self._base_datos.conexion() as conexion:
            cursor = conexion.execute(
                """
                DELETE FROM miembros
                WHERE grupo_id = %s AND usuario_id = %s AND estado = %s
                """,
                (id_grupo, usuario_id, EstadoMiembro.INVITADO.value),
            )
            if cursor.rowcount == 0:
                raise InvitacionDeGrupoNoPendiente()


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
