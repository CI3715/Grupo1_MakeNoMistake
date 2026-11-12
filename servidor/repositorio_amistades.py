"""Firmas del repositorio de amistades (Entrega 6).

Aquí se definen los tipos y las firmas; la implementación SQL llega con los
endpoints de amistades. Igual que en `repositorio_usuarios.py`, el resto del
servidor no debe conocer detalles del motor.

La amistad existe solo cuando el destinatario acepta (INV-10). La pareja es
única en cualquiera de los dos sentidos mientras esté pendiente o aceptada.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from uuid import UUID

from base_datos import BaseDatos


class EstadoAmistad(StrEnum):
    """Estados posibles de una fila de `amistades`."""

    PENDIENTE = "pendiente"
    ACEPTADA = "aceptada"
    RECHAZADA = "rechazada"
    ELIMINADA = "eliminada"


@dataclass(frozen=True)
class Amistad:
    """Fila de la tabla `amistades`."""

    id: UUID
    invitador_id: UUID
    invitado_id: UUID
    estado: EstadoAmistad
    fecha_invitacion: datetime
    fecha_respuesta: datetime | None
    fecha_baja: datetime | None


@dataclass(frozen=True)
class AmistadVista:
    """Amistad vista desde una de las dos personas.

    Nunca lleva el correo del otro: RF-SOC-04 solo permite nombre de usuario
    y nombre visible.
    """

    id: UUID
    estado: EstadoAmistad
    otro_id: UUID
    otro_nombre_usuario: str
    otro_nombre_visible: str
    enviada_por_mi: bool
    fecha_invitacion: datetime
    fecha_respuesta: datetime | None


class AmistadNoEncontrada(Exception):
    """No existe una amistad con ese id."""


class AmistadYaRespondida(Exception):
    """La invitación ya fue aceptada, rechazada o eliminada."""


class InvitacionPendiente(Exception):
    """Ya hay una invitación pendiente entre las dos personas."""


class YaSonAmigos(Exception):
    """Las dos personas ya son amigas."""


class RepositorioAmistades:
    """Consultas de la tabla `amistades`."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def crear(
        self, *, invitador_id: UUID, invitado_id: UUID, ahora: datetime
    ) -> Amistad:
        """Registra una invitación pendiente y devuelve la fila creada.

        Si ya existe una amistad pendiente se lanza `InvitacionPendiente`; si
        ya son amigos, `YaSonAmigos`. Una amistad rechazada o eliminada no
        impide un intento nuevo.
        """
        raise NotImplementedError

    def buscar_por_id(self, id_amistad: UUID) -> Amistad | None:
        """Devuelve la amistad o `None` si no existe."""
        raise NotImplementedError

    def buscar_activa_entre(self, uno: UUID, otro: UUID) -> Amistad | None:
        """Busca la amistad pendiente o aceptada entre dos personas.

        No importa quién invitó a quién: la pareja se consulta en los dos
        sentidos.
        """
        raise NotImplementedError

    def listar_de(self, usuario_id: UUID) -> list[AmistadVista]:
        """Amigos aceptados e invitaciones pendientes, enviadas y recibidas."""
        raise NotImplementedError

    def responder(self, *, id_amistad: UUID, aceptar: bool, ahora: datetime) -> Amistad:
        """Acepta o rechaza una invitación pendiente.

        Solo el destinatario puede responder, y solo mientras esté
        pendiente; en otro caso se lanza `AmistadYaRespondida`. Rechazar
        cierra la invitación sin crear amistad.
        """
        raise NotImplementedError

    def eliminar(self, *, id_amistad: UUID, ahora: datetime) -> None:
        """Marca la amistad como `eliminada`; la fila no se borra.

        Los grupos y gastos ya compartidos no cambian (RF-SOC-06).
        """
        raise NotImplementedError

    def son_amigos(self, uno: UUID, otro: UUID) -> bool:
        """Indica si existe una amistad aceptada entre las dos personas."""
        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(
                """
                SELECT 1 FROM amistades
                WHERE least(invitador_id, invitado_id) = least(%s, %s)
                  AND greatest(invitador_id, invitado_id) = greatest(%s, %s)
                  AND estado = 'aceptada'
                LIMIT 1
                """,
                (uno, otro, uno, otro),
            ).fetchone()
            return fila is not None
