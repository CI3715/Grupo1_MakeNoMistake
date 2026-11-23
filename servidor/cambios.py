"""Reglas del registro de cambios de cada usuario (Entrega 7, RF-COM-10).

Funciones puras: no tocan la base de datos, la red ni el reloj. El
repositorio y la ruta ``GET /cambios`` las usan para que las reglas queden en un
solo sitio y se puedan probar sin PostgreSQL (ADR-015).

- **Un evento por participante.** Al registrar un gasto, cada persona con una
  parte mayor que cero recibe en su registro un evento ``gasto_de_grupo`` con su
  parte. Quien queda fuera del reparto no recibe nada ni consume posición. Una
  parte de 0,00 (por ejemplo, 0,02 entre tres) tampoco: no hay nada que aplicar
  y el libro personal no admite movimientos de cero (INV-17).
- **Posición creciente por usuario.** La primera posición de cada registro es
  1 y crece de uno en uno. La posición es el cursor de la app.
- **El registro solo crece.** Un evento no se modifica ni se borra y su ``id``
  no cambia: pedir otra vez desde 0 devuelve los mismos eventos (INV-13).
- **Lotes.** ``GET /cambios`` entrega como máximo ``TAMANO_LOTE`` eventos en
  orden de posición, el cursor (la posición del último evento, o el mismo
  ``desde`` si no hay nada) y ``hay_mas``.

INV-16: el evento solo describe la parte del gasto de grupo. No dice en qué
cuenta se aplica ni cuál es el saldo de nadie: eso solo existe en la app.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from typing import Any, Generic, TypeVar
from uuid import UUID

from reglas import ErrorCampo

TIPO_GASTO_DE_GRUPO = "gasto_de_grupo"
TAMANO_LOTE = 100
# La posición se guarda como BIGINT: el cursor no puede pasar de ahí.
MAXIMO_CURSOR = 2**63 - 1
CENTIMO = Decimal("0.01")

MENSAJE_DESDE = "El cursor debe ser un número entero no negativo."

_SOLO_DIGITOS = re.compile(r"[0-9]+")

# El usuario es un UUID en el servidor; las pruebas de los vectores usan nombres.
U = TypeVar("U")


@dataclass(frozen=True)
class ParteParaEvento(Generic[U]):
    """Lo que el registro de cambios necesita de cada parte del gasto."""

    usuario: U
    monto: Decimal
    incluye_redondeo: bool


@dataclass(frozen=True)
class EventoCambio(Generic[U]):
    """Una fila del registro de cambios de ``usuario``: su parte de un gasto."""

    id: UUID
    usuario: U
    posicion: int
    grupo_id: UUID
    grupo_nombre: str
    gasto_id: UUID
    concepto: str
    fecha: date
    moneda: str
    monto: Decimal
    incluye_redondeo: bool


@dataclass(frozen=True)
class Lote(Generic[U]):
    """Respuesta de ``GET /cambios``: eventos, cursor siguiente y si hay más."""

    eventos: tuple[EventoCambio[U], ...]
    cursor: int
    hay_mas: bool


def eventos_de_gasto(
    *,
    gasto_id: UUID,
    concepto: str,
    fecha: date,
    grupo_id: UUID,
    grupo_nombre: str,
    moneda: str,
    partes: Sequence[ParteParaEvento[U]],
    ultimas_posiciones: Mapping[U, int],
    nuevo_id: Callable[[], UUID],
) -> list[EventoCambio[U]]:
    """Arma el evento de cada participante de un gasto recién registrado.

    ``ultimas_posiciones`` trae la última posición usada en el registro de cada
    persona (0 o ausente si aún no tiene eventos). Cada evento toma la
    siguiente. Los eventos salen en el orden de ``partes``; una parte de cero no
    genera evento. ``nuevo_id`` genera el identificador inmutable de cada uno.
    """
    eventos: list[EventoCambio[U]] = []
    vistos: set[U] = set()
    for parte in partes:
        if parte.usuario in vistos:
            raise ValueError("Hay una persona repetida entre las partes.")
        vistos.add(parte.usuario)
        monto = parte.monto.quantize(CENTIMO)
        if monto != parte.monto:
            raise ValueError("El monto de una parte tiene más de dos decimales.")
        if monto < 0:
            raise ValueError("El monto de una parte no puede ser negativo.")
        if monto == 0:
            continue
        ultima = ultimas_posiciones.get(parte.usuario, 0)
        if ultima < 0:
            raise ValueError("La última posición de un registro no puede ser negativa.")
        eventos.append(
            EventoCambio(
                id=nuevo_id(),
                usuario=parte.usuario,
                posicion=ultima + 1,
                grupo_id=grupo_id,
                grupo_nombre=grupo_nombre,
                gasto_id=gasto_id,
                concepto=concepto,
                fecha=fecha,
                moneda=moneda,
                monto=monto,
                incluye_redondeo=parte.incluye_redondeo,
            )
        )
    return eventos


def leer_desde(valor: str | None) -> tuple[int | None, ErrorCampo | None]:
    """Valida el parámetro ``desde`` de ``GET /cambios``.

    Sin el parámetro se toma 0. Solo se aceptan dígitos ASCII (sin signo, sin
    decimales ni espacios) y hasta el máximo de un BIGINT; cualquier otra cosa
    es un 400 con el campo ``desde``, nunca el 422 de FastAPI.
    """
    if valor is None:
        return 0, None
    if not _SOLO_DIGITOS.fullmatch(valor):
        return None, ErrorCampo("desde", MENSAJE_DESDE)
    numero = int(valor)
    if numero > MAXIMO_CURSOR:
        return None, ErrorCampo("desde", MENSAJE_DESDE)
    return numero, None


def armar_lote(
    posteriores: Sequence[EventoCambio[U]], desde: int, tamano: int = TAMANO_LOTE
) -> Lote[U]:
    """Corta el lote a partir de los eventos posteriores a ``desde``.

    El repositorio pide ``tamano + 1`` eventos en orden de posición: si llegan
    más de ``tamano``, ``hay_mas`` es verdadero y el sobrante se queda para la
    siguiente petición. El cursor es la posición del último evento entregado o,
    si no hay ninguno, el mismo ``desde``.
    """
    if tamano < 1:
        raise ValueError("El tamaño del lote debe ser al menos 1.")
    ordenados = sorted(posteriores, key=lambda evento: evento.posicion)
    if any(evento.posicion <= desde for evento in ordenados):
        raise ValueError("El lote solo puede traer eventos posteriores al cursor.")
    entregados = tuple(ordenados[:tamano])
    cursor = entregados[-1].posicion if entregados else desde
    return Lote(eventos=entregados, cursor=cursor, hay_mas=len(ordenados) > tamano)


def evento_a_json(evento: EventoCambio[Any]) -> dict[str, Any]:
    """El evento con la forma acordada en ``openapi.yaml`` (``EventoCambio``).

    El ``tipo`` no se guarda en la base: hoy todos los eventos son
    ``gasto_de_grupo``. El monto viaja como texto con dos decimales.
    """
    return {
        "id": str(evento.id),
        "posicion": evento.posicion,
        "tipo": TIPO_GASTO_DE_GRUPO,
        "grupo": {"id": str(evento.grupo_id), "nombre": evento.grupo_nombre},
        "gasto": {"id": str(evento.gasto_id), "concepto": evento.concepto},
        "fecha": evento.fecha.isoformat(),
        "moneda": evento.moneda,
        "monto": f"{evento.monto.quantize(CENTIMO):f}",
        "incluye_redondeo": evento.incluye_redondeo,
    }


def lote_a_json(lote: Lote[Any]) -> dict[str, Any]:
    """Cuerpo de la respuesta 200 de ``GET /cambios``."""
    return {
        "eventos": [evento_a_json(evento) for evento in lote.eventos],
        "cursor": lote.cursor,
        "hay_mas": lote.hay_mas,
    }
