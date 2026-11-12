"""División exacta de un gasto compartido (Entrega 6, sección 2.4).

Funciones puras, sin red ni base de datos. Toda la aritmética es entera sobre
céntimos utilizando Decimal (nunca coma flotante, RNF-08) y se convierte a céntimos
exactos. El procedimiento es determinista y coincide con el núcleo de Rust:

1. Se calcula la parte exacta de cada participante y se trunca a céntimos.
2. Residuo = total menos la suma de las partes truncadas.
3. El residuo se reparte de a un céntimo, en orden alfabético de nombre de
   usuario, hasta agotarlo.
4. Cada parte lleva una marca ``incluye_redondeo``.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from enum import StrEnum

MAX_CENTIMOS: int = 999_999_999_999  # 9.999.999.999,99
CIEN_POR_CIENTO: int = 10_000  # 100.00 %


class ReglaDivision(StrEnum):
    PARTES_IGUALES = "partes_iguales"
    PORCENTAJES = "porcentajes"
    MONTOS_FIJOS = "montos_fijos"


@dataclass(frozen=True)
class Participante:
    usuario: str
    valor: str = ""


@dataclass(frozen=True)
class ParteDividida:
    usuario: str
    monto: str
    incluye_redondeo: bool


@dataclass(frozen=True)
class Division:
    partes: list[ParteDividida]
    residuo: str


@dataclass(frozen=True)
class ErrorDivision:
    campo: str
    mensaje: str
    no_cuadra: bool = False


class ErrorValidacionDivision(Exception):
    """Excepción cuando los datos de división no son válidos."""

    def __init__(self, errores: list[ErrorDivision]) -> None:
        super().__init__("; ".join(e.mensaje for e in errores))
        self.errores = errores


def a_centesimas(texto: str) -> int | None:
    """Lee un número positivo con hasta dos decimales y lo devuelve ×100."""
    texto_limpio = texto.strip()
    if not texto_limpio:
        return None
    try:
        d = Decimal(texto_limpio).normalize()
    except InvalidOperation:
        return None

    exp = d.as_tuple().exponent
    if not isinstance(exp, int) or exp < -2 or d <= Decimal(0):
        return None

    return int(d * Decimal(100))


def con_punto(c: int) -> str:
    """Formatea céntimos como texto con punto (100 -> '1.00', -333 -> '-3.33')."""
    signo = "-" if c < 0 else ""
    abs_c = abs(c)
    return f"{signo}{abs_c // 100}.{abs_c % 100:02d}"


def con_coma(c: int) -> str:
    """Formatea céntimos como texto con coma (100 -> '1,00', -333 -> '-3,33')."""
    signo = "-" if c < 0 else ""
    abs_c = abs(c)
    return f"{signo}{abs_c // 100},{abs_c % 100:02d}"


def porcentaje_con_coma(c: int) -> str:
    """Porcentaje en centésimas como texto con coma, sin ceros innecesarios.

    Ejemplos: 9500 -> '95', 3350 -> '33,5', 3333 -> '33,33'.
    """
    entero = c // 100
    frac = c % 100
    if frac == 0:
        return str(entero)
    if frac % 10 == 0:
        return f"{entero},{frac // 10}"
    return f"{entero},{frac:02d}"


def faltan_o_sobran(esperado: int, suma: int) -> str:
    return "faltan" if suma < esperado else "sobran"


def dividir(
    monto: str,
    regla: str | ReglaDivision,
    participantes: Sequence[Participante | tuple[str, str] | str],
) -> Division:
    """Divide un monto entre los participantes según la regla indicada."""
    errores: list[ErrorDivision] = []

    total = a_centesimas(monto)
    if total is None:
        errores.append(
            ErrorDivision(
                campo="monto",
                mensaje="El monto debe ser mayor que 0 y tener hasta dos decimales.",
            )
        )
    elif total > MAX_CENTIMOS:
        errores.append(
            ErrorDivision(
                campo="monto",
                mensaje="El monto es demasiado grande.",
            )
        )

    regla_normalizada: ReglaDivision | None = None
    if isinstance(regla, ReglaDivision):
        texto_regla = regla.value
    else:
        texto_regla = str(regla).strip()
    try:
        regla_normalizada = ReglaDivision(texto_regla)
    except ValueError:
        errores.append(
            ErrorDivision(
                campo="regla",
                mensaje="Elige una regla de división válida.",
            )
        )

    if not participantes:
        errores.append(
            ErrorDivision(
                campo="participantes",
                mensaje="Elige al menos un participante.",
            )
        )

    nombres: list[tuple[str, str]] = []
    vistos: set[str] = set()

    for p in participantes:
        if isinstance(p, str):
            u, v = p, ""
        elif isinstance(p, tuple):
            u, v = p[0], p[1]
        else:
            u, v = p.usuario, p.valor

        usuario_limpio = u.strip().lower()
        if not usuario_limpio:
            errores.append(
                ErrorDivision(
                    campo="participantes",
                    mensaje="Falta el nombre de usuario de un participante.",
                )
            )
        elif usuario_limpio in vistos:
            errores.append(
                ErrorDivision(
                    campo="participantes",
                    mensaje=f"{usuario_limpio} está repetido entre los participantes.",
                )
            )
        else:
            vistos.add(usuario_limpio)
            nombres.append((usuario_limpio, v))

    if errores or total is None or regla_normalizada is None:
        raise ErrorValidacionDivision(errores)

    # Orden alfabético estricto de nombre de usuario
    nombres.sort(key=lambda x: x[0])
    cantidad = len(nombres)

    truncadas: list[int] = []

    if regla_normalizada == ReglaDivision.PARTES_IGUALES:
        base = total // cantidad
        truncadas = [base] * cantidad

    elif regla_normalizada == ReglaDivision.PORCENTAJES:
        valores_porcentaje: list[int] = []
        for usuario, texto_val in nombres:
            cent = a_centesimas(texto_val)
            if cent is None:
                errores.append(
                    ErrorDivision(
                        campo="porcentajes",
                        mensaje=(
                            f"El porcentaje de {usuario} debe ser mayor que 0 y tener "
                            "hasta dos decimales."
                        ),
                    )
                )
            else:
                valores_porcentaje.append(cent)

        if errores:
            raise ErrorValidacionDivision(errores)

        suma_porcentajes = sum(valores_porcentaje)
        if suma_porcentajes != CIEN_POR_CIENTO:
            diff = abs(CIEN_POR_CIENTO - suma_porcentajes)
            msg = (
                f"Los porcentajes suman {porcentaje_con_coma(suma_porcentajes)}; "
                f"{faltan_o_sobran(CIEN_POR_CIENTO, suma_porcentajes)} "
                f"{porcentaje_con_coma(diff)}."
            )
            raise ErrorValidacionDivision(
                [ErrorDivision(campo="porcentajes", mensaje=msg, no_cuadra=True)]
            )

        truncadas = [(total * c) // CIEN_POR_CIENTO for c in valores_porcentaje]

    elif regla_normalizada == ReglaDivision.MONTOS_FIJOS:
        valores_montos: list[int] = []
        for usuario, texto_val in nombres:
            cent = a_centesimas(texto_val)
            if cent is None:
                errores.append(
                    ErrorDivision(
                        campo="montos",
                        mensaje=(
                            f"El monto de {usuario} debe ser mayor que 0 y tener "
                            "hasta dos decimales."
                        ),
                    )
                )
            else:
                valores_montos.append(cent)

        if errores:
            raise ErrorValidacionDivision(errores)

        suma_montos = sum(valores_montos)
        if suma_montos != total:
            diff = abs(total - suma_montos)
            msg = (
                f"Los montos suman {con_coma(suma_montos)}; "
                f"{faltan_o_sobran(total, suma_montos)} {con_coma(diff)}."
            )
            raise ErrorValidacionDivision(
                [ErrorDivision(campo="montos", mensaje=msg, no_cuadra=True)]
            )

        truncadas = valores_montos

    # Residuo: total menos la suma de las partes truncadas
    residuo = total - sum(truncadas)
    por_repartir = residuo

    partes: list[ParteDividida] = []
    for (usuario, _), base in zip(nombres, truncadas, strict=True):
        recibe = por_repartir > 0
        if recibe:
            por_repartir -= 1
        partes.append(
            ParteDividida(
                usuario=usuario,
                monto=con_punto(base + (1 if recibe else 0)),
                incluye_redondeo=recibe,
            )
        )

    return Division(partes=partes, residuo=con_punto(residuo))


def calcular_balances_grupo(
    gastos: Sequence[tuple[str, list[tuple[str, str]]]],
) -> dict[str, str]:
    """Calcula el balance neto de cada miembro dado un historial de gastos.

    Cada gasto es una tupla: (pagador, [(participante, monto_parte)]).
    El balance de un usuario es: total_pagado - total_debido.
    La suma de todos los balances de todos los participantes es exactamente 0.00.
    """
    balances_centimos: dict[str, int] = {}

    for pagador, partes in gastos:
        pagador_limpio = pagador.strip().lower()
        monto_total_gasto = 0
        for usuario_parte, monto_texto in partes:
            usuario_limpio = usuario_parte.strip().lower()
            monto_cent = a_centesimas(monto_texto) or 0
            monto_total_gasto += monto_cent
            balances_centimos[usuario_limpio] = (
                balances_centimos.get(usuario_limpio, 0) - monto_cent
            )

        balances_centimos[pagador_limpio] = (
            balances_centimos.get(pagador_limpio, 0) + monto_total_gasto
        )

    return {u: con_punto(c) for u, c in sorted(balances_centimos.items())}
