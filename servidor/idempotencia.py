"""Reglas de la clave de idempotencia de ``POST /grupos/{id}/gastos`` (RNF-17, ESC-09).

Funciones puras: no tocan la base de datos, la red ni el reloj (ADR-015).

- La clave llega en el encabezado ``Idempotency-Key``. Es **opcional** para el
  servidor: sin ella el registro se comporta como en la Entrega 6 (siempre 201).
  La app la envía siempre: un UUID que genera una sola vez por operación.
- Una clave válida tiene de 8 a 128 caracteres ASCII visibles (sin espacios).
- Su alcance es **por usuario**: la misma clave de dos personas no choca.
- Junto con la clave se guarda la **huella** del cuerpo canónico (SHA-256). Una
  clave repetida con la misma huella devuelve el gasto ya registrado (200); con
  otra huella es otra operación y se rechaza (409).

La huella se calcula sobre los datos ya validados y la división ya calculada,
no sobre el texto recibido: «50» y «50.00» o un participante escrito con otra
mayúscula dan la misma huella, porque describen el mismo gasto.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections.abc import Sequence
from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from enum import StrEnum
from uuid import UUID

from reglas import ErrorCampo

MIN_CLAVE = 8
MAX_CLAVE = 128
CENTIMO = Decimal("0.01")

MENSAJE_CLAVE_INVALIDA = (
    f"La clave de idempotencia debe tener de {MIN_CLAVE} a {MAX_CLAVE} "
    "caracteres visibles."
)
MENSAJE_CLAVE_EN_CONFLICTO = (
    "Esa clave ya se usó con otros datos; usa una clave nueva para un gasto distinto."
)

# ASCII visible: del «!» (0x21) a la «~» (0x7E). Excluye espacios y controles.
_PATRON_CLAVE = re.compile(rf"[\x21-\x7E]{{{MIN_CLAVE},{MAX_CLAVE}}}")


class Decision(StrEnum):
    """Qué hacer con un registro según la clave guardada."""

    REGISTRAR = "registrar"  # clave nueva (o sin clave): 201
    DEVOLVER_EXISTENTE = "devolver_existente"  # misma clave y huella: 200
    CONFLICTO = "conflicto"  # misma clave, otra huella: 409


@dataclass(frozen=True)
class ParteParaHuella:
    usuario_id: UUID
    monto: Decimal
    porcentaje: int | None = None


@dataclass(frozen=True)
class ClaveGuardada:
    """Una clave ya consumida: qué gasto creó y con qué huella."""

    gasto_id: UUID
    huella: str


def validar_clave(valor: str | None) -> tuple[str | None, ErrorCampo | None]:
    """Devuelve la clave tal cual, ``None`` si no vino, o el error del campo.

    No se recorta ni se pasa a minúsculas: la clave es opaca y se compara
    exacta. Un encabezado vacío cuenta como clave inválida, no como ausente.
    """
    if valor is None:
        return None, None
    if not _PATRON_CLAVE.fullmatch(valor):
        return None, ErrorCampo("idempotency_key", MENSAJE_CLAVE_INVALIDA)
    return valor, None


def huella_gasto(
    *,
    grupo_id: UUID,
    concepto: str,
    monto: Decimal,
    fecha: date,
    pagador_id: UUID,
    regla: str,
    partes: Sequence[ParteParaHuella],
) -> str:
    """SHA-256 (hexadecimal) del cuerpo canónico de un gasto.

    Entran grupo, concepto, monto, fecha, pagador, regla y las partes ordenadas
    por usuario. Los montos van como texto con dos decimales y el JSON se
    serializa con claves ordenadas y sin espacios, así que dos cuerpos que
    describen el mismo gasto dan la misma huella en cualquier máquina.
    """
    canonico = {
        "grupo": str(grupo_id),
        "concepto": concepto,
        "monto": _dinero(monto),
        "fecha": fecha.isoformat(),
        "pagador": str(pagador_id),
        "regla": regla,
        "partes": [
            {
                "usuario": str(parte.usuario_id),
                "monto": _dinero(parte.monto),
                "porcentaje": parte.porcentaje,
            }
            for parte in sorted(partes, key=lambda p: str(p.usuario_id))
        ],
    }
    texto = json.dumps(
        canonico, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    )
    return hashlib.sha256(texto.encode("utf-8")).hexdigest()


def decidir(guardada: ClaveGuardada | None, huella: str) -> Decision:
    """Qué responder ante una clave según lo que ya se guardó con ella."""
    if guardada is None:
        return Decision.REGISTRAR
    if guardada.huella == huella:
        return Decision.DEVOLVER_EXISTENTE
    return Decision.CONFLICTO


def _dinero(monto: Decimal) -> str:
    exacto = monto.quantize(CENTIMO)
    if exacto != monto:
        raise ValueError("Un monto de la huella tiene más de dos decimales.")
    return f"{exacto:f}"
