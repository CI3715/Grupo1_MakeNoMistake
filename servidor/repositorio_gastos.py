"""Firmas del repositorio de gastos compartidos (Entrega 6).

Aquí solo viven los tipos y las firmas: la implementación SQL llega con los
endpoints de gastos.

Los montos son enteros en céntimos y los porcentajes enteros en centésimas
de punto. Ni `float` ni redondeos implícitos: la suma de las partes tiene que
ser exactamente el monto del gasto (INV-07, RNF-08).
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from datetime import date, datetime
from uuid import UUID

from base_datos import BaseDatos


@dataclass(frozen=True)
class Gasto:
    """Fila de la tabla `gastos`, con el monto en céntimos."""

    id: UUID
    grupo_id: UUID
    concepto: str
    monto: int
    fecha: date
    pagador_id: UUID
    regla: str
    registrado_por: UUID
    fecha_registro: datetime


@dataclass(frozen=True)
class ParteGasto:
    """Importe exacto que le corresponde a una persona en un gasto."""

    gasto_id: UUID
    usuario_id: UUID
    monto: int
    lleva_residuo: bool
    porcentaje: int | None


@dataclass(frozen=True)
class GastoVista:
    """Gasto con el nombre de quien pagó y sus partes."""

    gasto: Gasto
    pagador_nombre_usuario: str
    partes: tuple[ParteGasto, ...]


class GastoInvalido(Exception):
    """El gasto no cumple las reglas de la división.

    Por ejemplo, partes que no suman el monto del gasto o un participante que
    no pertenece al grupo.
    """


class RepositorioGastos:
    """Consultas de las tablas `gastos` y `partes_gasto`."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def registrar(
        self,
        *,
        grupo_id: UUID,
        concepto: str,
        monto: int,
        fecha: date,
        pagador_id: UUID,
        regla: str,
        partes: Sequence[ParteGasto],
        registrado_por: UUID,
        ahora: datetime,
    ) -> GastoVista:
        """Guarda el gasto y sus partes en una sola transacción.

        Antes de confirmar se comprueba que la suma de las partes sea
        exactamente `monto`; si no, se lanza `GastoInvalido` y no se guarda
        nada (INV-07).
        """
        raise NotImplementedError

    def listar_del_grupo(self, id_grupo: UUID) -> list[GastoVista]:
        """Gastos del grupo, del más reciente al más antiguo."""
        raise NotImplementedError
