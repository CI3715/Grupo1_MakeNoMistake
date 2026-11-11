"""Firmas del repositorio de grupos y miembros (Entrega 6).

Igual que `repositorio_amistades.py`, aquí solo viven los tipos y las firmas:
la implementación SQL llega con los endpoints de grupos.

Un grupo nace con su creador como miembro administrador y aceptado
(RF-COM-01). Nadie más entra sin aceptar la invitación (RF-COM-02): mientras
el estado sea `invitado`, la persona no ve gastos ni balances (INV-17).
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime
from enum import StrEnum
from uuid import UUID

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
        """Crea el grupo y su miembro administrador en una sola transacción.

        La moneda llega ya normalizada a mayúsculas y con tres letras.
        """
        raise NotImplementedError

    def buscar_por_id(self, id_grupo: UUID) -> Grupo | None:
        """Devuelve el grupo o `None` si no existe."""
        raise NotImplementedError

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None:
        """Devuelve la pertenencia o `None` si la persona no está en el grupo."""
        raise NotImplementedError

    def listar_de(self, usuario_id: UUID) -> list[GrupoVista]:
        """Grupos aceptados e invitaciones pendientes de una persona."""
        raise NotImplementedError

    def listar_miembros(self, id_grupo: UUID) -> list[MiembroVista]:
        """Miembros e invitados de un grupo."""
        raise NotImplementedError

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
        raise NotImplementedError

    def aceptar_invitacion(
        self, *, id_grupo: UUID, usuario_id: UUID, ahora: datetime
    ) -> Miembro:
        """Pasa la pertenencia de `invitado` a `aceptado`.

        Solo el invitado puede hacerlo; si no hay invitación pendiente se
        lanza `InvitacionDeGrupoNoPendiente`.
        """
        raise NotImplementedError

    def rechazar_invitacion(self, *, id_grupo: UUID, usuario_id: UUID) -> None:
        """Elimina la fila de la persona rechazada.

        La tabla solo admite los estados `invitado` y `aceptado`, así que un
        rechazo no deja fila.
        """
        raise NotImplementedError
