"""Lectura del registro de cambios de cada usuario sobre PostgreSQL (Entrega 7).

Por ahora solo la firma que usa ``GET /cambios`` (``AlmacenCambios`` en
``main.py``). Las tablas, el contador de posiciones y la lectura por cursor
pertenecen al repositorio de cambios de la Entrega 7; las reglas que aplica
(lotes, cursor, forma del evento) ya están en ``cambios.py`` (ADR-015).
"""

from __future__ import annotations

from uuid import UUID

from base_datos import BaseDatos
from cambios import EventoCambio

PENDIENTE_CAMBIOS = (
    "Pendiente: registro de cambios en PostgreSQL (repositorio de cambios, Entrega 7)."
)


class RepositorioCambios:
    """Lectura del registro de cambios de cada usuario."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def listar_desde(
        self, usuario_id: UUID, desde: int, limite: int
    ) -> list[EventoCambio[UUID]]:
        """Hasta ``limite`` eventos de ``usuario_id`` posteriores a ``desde``.

        Deben salir en orden de posición y ser solo de esa persona (INV-17).
        """
        raise NotImplementedError(PENDIENTE_CAMBIOS)
