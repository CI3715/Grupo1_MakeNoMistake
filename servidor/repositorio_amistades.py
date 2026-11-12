"""Repositorio de amistades sobre PostgreSQL (Entrega 6).

La amistad existe solo cuando el destinatario acepta (INV-10). La pareja es
única en cualquiera de los dos sentidos mientras esté pendiente o aceptada: lo
garantiza el índice único parcial ``amistades_pareja_activa`` sobre
``least/greatest``. Un rechazo o una baja no impiden volver a intentarlo.

Eliminar una amistad **no** borra la fila ni toca grupos, miembros o gastos:
solo cambia el estado a ``eliminada`` y guarda la fecha de baja.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from typing import Any
from uuid import UUID, uuid4

from psycopg import Connection, errors

from base_datos import BaseDatos

COLUMNAS = (
    "id, invitador_id, invitado_id, estado, fecha_invitacion, "
    "fecha_respuesta, fecha_baja"
)

SQL_INSERTAR = f"""
    INSERT INTO amistades ({COLUMNAS})
    VALUES (%s, %s, %s, %s, %s, %s, %s)
    RETURNING {COLUMNAS}
"""

SQL_BUSCAR_POR_ID = f"""
    SELECT {COLUMNAS} FROM amistades WHERE id = %s
"""

SQL_BUSCAR_ACTIVA_ENTRE = f"""
    SELECT {COLUMNAS}
    FROM amistades
    WHERE (
        (invitador_id = %s AND invitado_id = %s)
        OR (invitador_id = %s AND invitado_id = %s)
    )
      AND estado IN ('pendiente', 'aceptada')
    ORDER BY fecha_invitacion DESC, id
    LIMIT 1
"""

SQL_LISTAR_DE = """
    SELECT a.id, a.estado, a.fecha_invitacion, a.fecha_respuesta,
           CASE
               WHEN a.invitador_id = %s THEN a.invitado_id
               ELSE a.invitador_id
           END AS otro_id,
           u.nombre_usuario AS otro_nombre_usuario,
           u.nombre_visible AS otro_nombre_visible,
           (a.invitador_id = %s) AS enviada_por_mi
    FROM amistades a
    JOIN usuarios u ON u.id = CASE
        WHEN a.invitador_id = %s THEN a.invitado_id
        ELSE a.invitador_id
    END
    WHERE (a.invitador_id = %s OR a.invitado_id = %s)
      AND a.estado IN ('pendiente', 'aceptada')
    ORDER BY a.fecha_invitacion DESC, a.id
"""

SQL_RESPONDER = f"""
    UPDATE amistades
    SET estado = %s, fecha_respuesta = %s
    WHERE id = %s AND estado = 'pendiente'
    RETURNING {COLUMNAS}
"""

SQL_ELIMINAR = f"""
    UPDATE amistades
    SET estado = 'eliminada',
        fecha_respuesta = COALESCE(fecha_respuesta, %s),
        fecha_baja = %s
    WHERE id = %s AND estado IN ('pendiente', 'aceptada')
    RETURNING {COLUMNAS}
"""

SQL_SON_AMIGOS = """
    SELECT 1 FROM amistades
    WHERE (
        (invitador_id = %s AND invitado_id = %s)
        OR (invitador_id = %s AND invitado_id = %s)
    )
      AND estado = 'aceptada'
    LIMIT 1
"""


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


class AmistadInvalida(Exception):
    """La invitación no cumple una regla básica, como invitarse a uno mismo."""


class NoEsDestinatario(Exception):
    """Quien intenta responder no es la persona invitada."""


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
        if invitador_id == invitado_id:
            raise AmistadInvalida("Una persona no puede invitarse a sí misma.")

        fecha = _sin_microsegundos(ahora)
        amistad = Amistad(
            id=uuid4(),
            invitador_id=invitador_id,
            invitado_id=invitado_id,
            estado=EstadoAmistad.PENDIENTE,
            fecha_invitacion=fecha,
            fecha_respuesta=None,
            fecha_baja=None,
        )

        with self._base_datos.conexion() as conexion:
            activa = _buscar_activa(conexion, invitador_id, invitado_id)
            if activa is not None:
                _rechazar_si_activa(activa)

            try:
                fila = conexion.execute(
                    SQL_INSERTAR,
                    (
                        amistad.id,
                        amistad.invitador_id,
                        amistad.invitado_id,
                        amistad.estado.value,
                        amistad.fecha_invitacion,
                        amistad.fecha_respuesta,
                        amistad.fecha_baja,
                    ),
                ).fetchone()
            except errors.UniqueViolation as error:
                # Otra petición creó la pareja entre la consulta y el INSERT.
                activa = _buscar_activa(conexion, invitador_id, invitado_id)
                if activa is not None:
                    _rechazar_si_activa(activa)
                raise InvitacionPendiente(
                    "Ya existe una invitación pendiente entre estas personas."
                ) from error

        assert fila is not None
        return _amistad_desde_fila(fila)

    def buscar_por_id(self, id_amistad: UUID) -> Amistad | None:
        """Devuelve la amistad o `None` si no existe."""

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(SQL_BUSCAR_POR_ID, (id_amistad,)).fetchone()

        return None if fila is None else _amistad_desde_fila(fila)

    def buscar_activa_entre(self, uno: UUID, otro: UUID) -> Amistad | None:
        """Busca la amistad pendiente o aceptada entre dos personas.

        No importa quién invitó a quién: la pareja se consulta en los dos
        sentidos.
        """

        with self._base_datos.conexion() as conexion:
            return _buscar_activa(conexion, uno, otro)

    def listar_de(self, usuario_id: UUID) -> list[AmistadVista]:
        """Amigos aceptados e invitaciones pendientes, enviadas y recibidas."""

        with self._base_datos.conexion() as conexion:
            filas = conexion.execute(
                SQL_LISTAR_DE,
                (usuario_id, usuario_id, usuario_id, usuario_id, usuario_id),
            ).fetchall()

        return [
            AmistadVista(
                id=fila["id"],
                estado=EstadoAmistad(fila["estado"]),
                otro_id=fila["otro_id"],
                otro_nombre_usuario=fila["otro_nombre_usuario"],
                otro_nombre_visible=fila["otro_nombre_visible"],
                enviada_por_mi=bool(fila["enviada_por_mi"]),
                fecha_invitacion=fila["fecha_invitacion"],
                fecha_respuesta=fila["fecha_respuesta"],
            )
            for fila in filas
        ]

    def responder(
        self,
        *,
        id_amistad: UUID,
        aceptar: bool,
        ahora: datetime,
        usuario_id: UUID | None = None,
    ) -> Amistad:
        """Acepta o rechaza una invitación pendiente.

        Si se pasa ``usuario_id``, solo la persona invitada puede responder.
        En cualquier caso, la fila debe estar pendiente; si no, se lanza
        `AmistadYaRespondida`. Rechazar cierra la invitación sin crear
        amistad.
        """

        fecha = _sin_microsegundos(ahora)
        nuevo_estado = EstadoAmistad.ACEPTADA if aceptar else EstadoAmistad.RECHAZADA

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(SQL_BUSCAR_POR_ID, (id_amistad,)).fetchone()
            if fila is None:
                raise AmistadNoEncontrada("No existe esa amistad.")
            amistad = _amistad_desde_fila(fila)
            if usuario_id is not None and amistad.invitado_id != usuario_id:
                raise NoEsDestinatario("Solo la persona invitada puede responder.")
            if amistad.estado is not EstadoAmistad.PENDIENTE:
                raise AmistadYaRespondida("La invitación ya fue respondida.")

            fila = conexion.execute(
                SQL_RESPONDER, (nuevo_estado.value, fecha, id_amistad)
            ).fetchone()

        if fila is None:
            raise AmistadYaRespondida("La invitación ya fue respondida.")
        return _amistad_desde_fila(fila)

    def eliminar(self, *, id_amistad: UUID, ahora: datetime) -> None:
        """Marca la amistad como `eliminada`; la fila no se borra.

        Los grupos y gastos ya compartidos no cambian (RF-SOC-06).
        """

        fecha = _sin_microsegundos(ahora)

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(SQL_BUSCAR_POR_ID, (id_amistad,)).fetchone()
            if fila is None:
                raise AmistadNoEncontrada("No existe esa amistad.")
            amistad = _amistad_desde_fila(fila)
            if amistad.estado in (
                EstadoAmistad.RECHAZADA,
                EstadoAmistad.ELIMINADA,
            ):
                raise AmistadYaRespondida("La amistad ya está cerrada.")

            fila = conexion.execute(SQL_ELIMINAR, (fecha, fecha, id_amistad)).fetchone()

        if fila is None:
            raise AmistadYaRespondida("La amistad ya está cerrada.")

    def son_amigos(self, uno: UUID, otro: UUID) -> bool:
        """Indica si existe una amistad aceptada entre las dos personas."""

        with self._base_datos.conexion() as conexion:
            fila = conexion.execute(SQL_SON_AMIGOS, (uno, otro, otro, uno)).fetchone()

        return fila is not None


def _buscar_activa(
    conexion: Connection[dict[str, Any]], uno: UUID, otro: UUID
) -> Amistad | None:
    fila = conexion.execute(SQL_BUSCAR_ACTIVA_ENTRE, (uno, otro, otro, uno)).fetchone()
    return None if fila is None else _amistad_desde_fila(fila)


def _rechazar_si_activa(amistad: Amistad) -> None:
    if amistad.estado is EstadoAmistad.ACEPTADA:
        raise YaSonAmigos("Estas dos personas ya son amigas.")
    raise InvitacionPendiente(
        "Ya existe una invitación pendiente entre estas personas."
    )


def _sin_microsegundos(ahora: datetime) -> datetime:
    return ahora.replace(microsecond=0)


def _amistad_desde_fila(fila: dict[str, Any]) -> Amistad:
    return Amistad(
        id=fila["id"],
        invitador_id=fila["invitador_id"],
        invitado_id=fila["invitado_id"],
        estado=EstadoAmistad(fila["estado"]),
        fecha_invitacion=fila["fecha_invitacion"],
        fecha_respuesta=fila["fecha_respuesta"],
        fecha_baja=fila["fecha_baja"],
    )
