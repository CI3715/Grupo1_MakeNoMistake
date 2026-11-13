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

import re
from collections.abc import Sequence
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from enum import StrEnum

MAX_CENTIMOS: int = 999_999_999_999  # 9.999.999.999,99
CIEN_POR_CIENTO: int = 10_000  # 100.00 %
NUMERO_DECIMAL = re.compile(r"[+-]?(\d+(\.\d*)?|\.\d+)")


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
    # Como en el núcleo de Rust: sin notación científica («1e2»), NaN ni infinito.
    if not NUMERO_DECIMAL.fullmatch(texto_limpio):
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


@dataclass(frozen=True)
class GastoParaBalance:
    """Lo que el balance necesita de un gasto: quién pagó, cuánto y las partes."""

    pagador: str
    monto: str
    partes: Sequence[tuple[str, str]]


@dataclass(frozen=True)
class Balance:
    """Balance de un miembro (sección 2.5). Montos como texto con punto.

    ``balance = pago - le_corresponde``: positivo, le deben; negativo, debe.
    """

    usuario: str
    pago: str
    le_corresponde: str
    balance: str


def centimos_exactos(texto: str) -> int:
    """Céntimos de un monto guardado («33.34» -> 3334). Admite 0 y negativos.

    A diferencia de ``a_centesimas`` (que valida lo que escribe una persona),
    esta función lee montos que ya pasaron por la división, así que un valor
    con más de dos decimales es un error de programación y se rechaza en lugar
    de truncarse en silencio.
    """
    try:
        numero = Decimal(texto.strip())
    except InvalidOperation as error:
        raise ValueError(f"Monto no válido: {texto!r}") from error
    if not numero.is_finite():
        raise ValueError(f"Monto no válido: {texto!r}")
    centimos = numero * 100
    if centimos != centimos.to_integral_value():
        raise ValueError(f"El monto {texto!r} tiene más de dos decimales.")
    return int(centimos)


def calcular_balances(
    miembros: Sequence[str], gastos: Sequence[GastoParaBalance]
) -> list[Balance]:
    """Balance de cada persona del grupo, en orden alfabético (INV-08).

    Aparece cada miembro, aunque no tenga gastos (con 0.00), y también quien
    figure en un gasto sin estar en ``miembros``, para que la suma no pierda
    ningún céntimo. Toda la aritmética es entera sobre céntimos, así que la suma
    de los balances es exactamente 0.00 siempre que cada gasto cuadre (INV-07).
    """
    pago: dict[str, int] = {m.strip().lower(): 0 for m in miembros}
    corresponde: dict[str, int] = dict.fromkeys(pago, 0)

    for gasto in gastos:
        pagador = gasto.pagador.strip().lower()
        pago[pagador] = pago.get(pagador, 0) + centimos_exactos(gasto.monto)
        corresponde.setdefault(pagador, 0)
        for usuario, monto in gasto.partes:
            nombre = usuario.strip().lower()
            corresponde[nombre] = corresponde.get(nombre, 0) + centimos_exactos(monto)
            pago.setdefault(nombre, 0)

    return [
        Balance(
            usuario=usuario,
            pago=con_punto(pago[usuario]),
            le_corresponde=con_punto(corresponde[usuario]),
            balance=con_punto(pago[usuario] - corresponde[usuario]),
        )
        for usuario in sorted(pago)
    ]


def calcular_balances_grupo(
    gastos: Sequence[tuple[str, list[tuple[str, str]]]],
) -> dict[str, str]:
    """Balance neto por usuario (versión compacta de ``calcular_balances``).

    Cada gasto es ``(pagador, [(participante, monto_parte)])`` y el monto del
    gasto es la suma de sus partes.
    """
    convertidos = [
        GastoParaBalance(
            pagador=pagador,
            monto=con_punto(sum(centimos_exactos(m) for _, m in partes)),
            partes=partes,
        )
        for pagador, partes in gastos
    ]
    return {b.usuario: b.balance for b in calcular_balances([], convertidos)}
