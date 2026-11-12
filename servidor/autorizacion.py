"""Autorización de las rutas de un grupo (INV-17, RNF-07, ESC-08).

Solo un miembro **aceptado** puede ver o escribir en un grupo. Todo lo demás
(identificador mal formado, grupo inexistente, persona ajena, invitado que aún
no acepta) recibe exactamente el mismo 404 con el mismo cuerpo, para que nadie
pueda averiguar si un grupo existe probando identificadores.

La decisión es una función pura sobre lo que devuelve el almacén: no sabe de
HTTP ni de PostgreSQL, y la ruta solo traduce ``None`` en la respuesta 404.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol
from uuid import UUID

from fastapi.responses import JSONResponse

from repositorio_grupos import EstadoMiembro, Grupo, Miembro

MENSAJE_GRUPO_NO_ENCONTRADO = "Grupo no encontrado."


class AlmacenMiembros(Protocol):
    """Lo mínimo que la autorización necesita del almacén de grupos."""

    def buscar_por_id(self, id_grupo: UUID) -> Grupo | None: ...

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None: ...


@dataclass(frozen=True)
class Pertenencia:
    """El grupo y la fila de miembro de quien hace la petición."""

    grupo: Grupo
    miembro: Miembro


def pertenencia_aceptada(
    almacen: AlmacenMiembros, id_grupo: str, usuario_id: UUID
) -> Pertenencia | None:
    """El grupo y la membresía si ``usuario_id`` es miembro aceptado; si no, None.

    Devuelve ``None`` en los cuatro casos que deben ser indistinguibles: un
    identificador que no es UUID, un grupo que no existe, una persona que no
    pertenece y una invitación todavía sin aceptar.
    """
    try:
        uuid_grupo = UUID(id_grupo)
    except ValueError:
        return None

    grupo = almacen.buscar_por_id(uuid_grupo)
    if grupo is None:
        return None

    miembro = almacen.buscar_miembro(id_grupo=uuid_grupo, usuario_id=usuario_id)
    if miembro is None or miembro.estado != EstadoMiembro.ACEPTADO:
        return None

    return Pertenencia(grupo=grupo, miembro=miembro)


def grupo_no_encontrado() -> JSONResponse:
    """El 404 uniforme: mismo código y mismo cuerpo en todos los casos."""
    return JSONResponse(
        status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
    )
