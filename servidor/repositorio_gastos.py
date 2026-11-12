"""Repositorio de gastos compartidos sobre PostgreSQL (Entrega 6).

El dinero viaja y se guarda como ``Decimal`` con dos decimales exactos y la
columna es ``NUMERIC(14, 2)``. Nunca se usa ``float``: sumar, comparar y
repartir se hace con aritmética decimal exacta y enteros de céntimos.

El repositorio recibe las partes **ya calculadas** por el motor de división.
Su trabajo es comprobar que la regla se cumple (la suma, los porcentajes o el
reparto), que las personas pertenecen al grupo y guardar el gasto junto con
todas sus partes en una sola transacción: o queda todo, o no queda nada.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from datetime import date, datetime
from decimal import Decimal, InvalidOperation
from typing import Any
from uuid import UUID, uuid4

from psycopg import Connection, errors

from base_datos import BaseDatos

# Escala y límites del dinero. ``NUMERIC(14, 2)`` admite doce dígitos enteros.
CENTIMOS = Decimal("0.01")
MAXIMO = Decimal("999999999999.99")

REGLAS_VALIDAS = frozenset({"iguales", "porcentajes", "montos"})
# La interfaz usa nombres más largos; la base guarda los canónicos.
ALIAS_REGLAS = {"partes_iguales": "iguales", "montos_fijos": "montos"}

SQL_INSERTAR_GASTO = """
    INSERT INTO gastos (
        id, grupo_id, concepto, monto, fecha, pagador_id,
        regla, registrado_por, fecha_registro
    )
    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
"""

SQL_INSERTAR_PARTE = """
    INSERT INTO partes_gasto (
        gasto_id, usuario_id, monto, lleva_residuo, porcentaje
    )
    VALUES (%s, %s, %s, %s, %s)
"""

SQL_LISTAR_GASTOS = """
    SELECT g.id, g.grupo_id, g.concepto, g.monto, g.fecha, g.pagador_id,
           g.regla, g.registrado_por, g.fecha_registro,
           u.nombre_usuario AS pagador_nombre_usuario
    FROM gastos g
    JOIN usuarios u ON u.id = g.pagador_id
    WHERE g.grupo_id = %s
    ORDER BY g.fecha DESC, g.fecha_registro DESC, g.id DESC
"""

SQL_LISTAR_PARTES = """
    SELECT p.gasto_id, p.usuario_id, p.monto, p.lleva_residuo, p.porcentaje
    FROM partes_gasto p
    JOIN gastos g ON g.id = p.gasto_id
    JOIN usuarios u ON u.id = p.usuario_id
    WHERE g.grupo_id = %s
    ORDER BY p.gasto_id, u.nombre_usuario
"""

SQL_EXISTE_GRUPO = "SELECT 1 FROM grupos WHERE id = %s"
SQL_EXISTE_MIEMBRO_ACEPTADO = """
    SELECT 1 FROM miembros
    WHERE grupo_id = %s AND usuario_id = %s AND estado = 'aceptado'
"""
SQL_PARTICIPANTES_ACEPTADOS = """
    SELECT usuario_id FROM miembros
    WHERE grupo_id = %s AND estado = 'aceptado' AND usuario_id = ANY(%s)
"""
SQL_NOMBRE_USUARIO = "SELECT nombre_usuario FROM usuarios WHERE id = %s"


@dataclass(frozen=True)
class Gasto:
    """Fila de la tabla `gastos`, con el monto exacto en euros."""

    id: UUID
    grupo_id: UUID
    concepto: str
    monto: Decimal
    fecha: date
    pagador_id: UUID
    regla: str
    registrado_por: UUID
    fecha_registro: datetime


@dataclass(frozen=True)
class ParteCalculada:
    """Parte que entrega el motor de división, antes de existir el gasto.

    No lleva ``gasto_id`` porque el identificador lo genera el repositorio en
    la misma transacción. Así el llamador no tiene que inventar un UUID que
    después podría no coincidir con el guardado.
    """

    usuario_id: UUID
    monto: Decimal
    lleva_residuo: bool = False
    porcentaje: int | None = None


@dataclass(frozen=True)
class ParteGasto:
    """Importe exacto que le corresponde a una persona en un gasto guardado."""

    gasto_id: UUID
    usuario_id: UUID
    monto: Decimal
    lleva_residuo: bool
    porcentaje: int | None


# Una parte puede llegar ya con forma de fila (por compatibilidad con la
# firma anterior) o como parte calculada sin `gasto_id`.
ParteEntrada = ParteCalculada | ParteGasto


@dataclass(frozen=True)
class GastoVista:
    """Gasto con el nombre de quien pagó y sus partes."""

    gasto: Gasto
    pagador_nombre_usuario: str
    partes: tuple[ParteGasto, ...]


class GastoInvalido(Exception):
    """El gasto no cumple las reglas de la división.

    ``campo`` dice qué dato falla (``monto``, ``regla``, ``partes``,
    ``porcentajes``, ``participantes``, ``pagador``...) y ``regla`` identifica
    la comprobación concreta (``suma``, ``porcentajes``, ``iguales``,
    ``miembro``...). La capa HTTP puede traducirlo a un 400 sin mirar el driver.
    """

    def __init__(self, campo: str, mensaje: str, *, regla: str | None = None) -> None:
        self.campo = campo
        self.regla = regla
        self.mensaje = mensaje
        super().__init__(mensaje)


class RepositorioGastos:
    """Consultas de las tablas `gastos` y `partes_gasto`."""

    def __init__(self, base_datos: BaseDatos) -> None:
        self._base_datos = base_datos

    def registrar(
        self,
        *,
        grupo_id: UUID,
        concepto: str,
        monto: Decimal,
        fecha: date,
        pagador_id: UUID,
        regla: str,
        partes: Sequence[ParteEntrada],
        registrado_por: UUID,
        ahora: datetime,
    ) -> GastoVista:
        """Guarda el gasto y sus partes en una sola transacción.

        Antes de tocar la base se comprueba que la suma de las partes sea
        exactamente ``monto`` y que la regla se cumpla; después, que pagador,
        registrador y participantes sean miembros aceptados del grupo. Si
        algo falla se lanza ``GastoInvalido`` con el campo y la regla, y no se
        guarda nada (INV-07).
        """
        regla_normalizada = _normalizar_regla(regla)
        monto_exacto = _monto_valido(monto, "monto", permitir_cero=False)
        concepto_limpio = concepto.strip()
        if not concepto_limpio:
            raise GastoInvalido(
                "concepto", "El concepto del gasto no puede estar vacío."
            )

        calculadas = tuple(_normalizar_parte(parte) for parte in partes)
        _validar_partes(calculadas, monto_exacto, regla_normalizada)

        gasto = Gasto(
            id=uuid4(),
            grupo_id=grupo_id,
            concepto=concepto_limpio,
            monto=monto_exacto,
            fecha=fecha,
            pagador_id=pagador_id,
            regla=regla_normalizada,
            registrado_por=registrado_por,
            fecha_registro=_sin_microsegundos(ahora),
        )
        partes_guardadas = tuple(
            ParteGasto(
                gasto_id=gasto.id,
                usuario_id=parte.usuario_id,
                monto=parte.monto,
                lleva_residuo=parte.lleva_residuo,
                porcentaje=parte.porcentaje,
            )
            for parte in calculadas
        )

        with self._base_datos.conexion() as conexion:
            _exigir_grupo(conexion, grupo_id)
            _exigir_miembro_aceptado(
                conexion,
                grupo_id,
                pagador_id,
                campo="pagador",
                mensaje="El pagador no es miembro aceptado del grupo.",
            )
            _exigir_miembro_aceptado(
                conexion,
                grupo_id,
                registrado_por,
                campo="registrado_por",
                mensaje="Quien registra el gasto no es miembro aceptado del grupo.",
            )
            _exigir_participantes(conexion, grupo_id, calculadas)
            nombre_pagador = _nombre_usuario(conexion, pagador_id)

            try:
                conexion.execute(
                    SQL_INSERTAR_GASTO,
                    (
                        gasto.id,
                        gasto.grupo_id,
                        gasto.concepto,
                        gasto.monto,
                        gasto.fecha,
                        gasto.pagador_id,
                        gasto.regla,
                        gasto.registrado_por,
                        gasto.fecha_registro,
                    ),
                )
                _insertar_partes(conexion, partes_guardadas)
            except errors.ForeignKeyViolation as error:
                raise GastoInvalido(
                    "participantes",
                    "Alguna persona del gasto ya no pertenece al grupo.",
                    regla="miembro",
                ) from error
            except errors.UniqueViolation as error:
                raise GastoInvalido(
                    "participantes",
                    "Hay una persona repetida entre las partes.",
                    regla="participantes",
                ) from error
            except errors.CheckViolation as error:
                raise GastoInvalido(
                    "datos",
                    "El gasto incumple una restricción de la base de datos.",
                ) from error
            except errors.NumericValueOutOfRange as error:
                raise GastoInvalido(
                    "monto", "El monto supera el máximo que admite la base."
                ) from error

        return GastoVista(
            gasto=gasto,
            pagador_nombre_usuario=nombre_pagador,
            partes=partes_guardadas,
        )

    def listar_del_grupo(self, id_grupo: UUID) -> list[GastoVista]:
        """Gastos del grupo, del más reciente al más antiguo."""

        with self._base_datos.conexion() as conexion:
            filas_gastos = conexion.execute(SQL_LISTAR_GASTOS, (id_grupo,)).fetchall()
            filas_partes = conexion.execute(SQL_LISTAR_PARTES, (id_grupo,)).fetchall()

        partes_por_gasto: dict[UUID, list[ParteGasto]] = {}
        for fila in filas_partes:
            parte = _parte_desde_fila(fila)
            partes_por_gasto.setdefault(parte.gasto_id, []).append(parte)

        return [
            GastoVista(
                gasto=_gasto_desde_fila(fila),
                pagador_nombre_usuario=fila["pagador_nombre_usuario"],
                partes=tuple(partes_por_gasto.get(fila["id"], [])),
            )
            for fila in filas_gastos
        ]


def _insertar_partes(
    conexion: Connection[dict[str, Any]], partes: Sequence[ParteGasto]
) -> None:
    with conexion.cursor() as cursor:
        cursor.executemany(
            SQL_INSERTAR_PARTE,
            [
                (
                    parte.gasto_id,
                    parte.usuario_id,
                    parte.monto,
                    parte.lleva_residuo,
                    parte.porcentaje,
                )
                for parte in partes
            ],
        )


def _normalizar_regla(regla: str) -> str:
    """Traduce los nombres de la interfaz al valor canónico de la base."""

    nombre = regla.strip().lower()
    nombre = ALIAS_REGLAS.get(nombre, nombre)
    if nombre not in REGLAS_VALIDAS:
        raise GastoInvalido(
            "regla",
            f"La regla «{regla}» no es válida; usa iguales, porcentajes o montos.",
            regla="regla",
        )
    return nombre


def _normalizar_parte(parte: ParteEntrada) -> ParteCalculada:
    return ParteCalculada(
        usuario_id=parte.usuario_id,
        monto=_monto_valido(parte.monto, "partes", permitir_cero=True),
        lleva_residuo=parte.lleva_residuo,
        porcentaje=parte.porcentaje,
    )


def _monto_valido(valor: object, campo: str, *, permitir_cero: bool) -> Decimal:
    """Convierte un monto a Decimal con dos decimales exactos o falla.

    Acepta ``Decimal``, ``int`` y texto; rechaza ``float`` de forma explícita,
    valores con más de dos decimales y cantidades fuera del rango de la
    columna. No redondea nunca en silencio.
    """

    if isinstance(valor, bool):
        raise GastoInvalido(campo, "El monto debe ser un número decimal exacto.")
    if isinstance(valor, float):
        raise GastoInvalido(
            campo,
            "El monto no puede ser coma flotante; usa Decimal o texto.",
        )
    if isinstance(valor, Decimal):
        numero = valor
    elif isinstance(valor, int):
        numero = Decimal(valor)
    elif isinstance(valor, str):
        try:
            numero = Decimal(valor.strip())
        except InvalidOperation as error:
            raise GastoInvalido(
                campo, "El monto no es un número decimal válido."
            ) from error
    else:
        raise GastoInvalido(campo, "El monto debe ser un número decimal exacto.")

    if not numero.is_finite():
        raise GastoInvalido(campo, "El monto debe ser un número finito.")
    try:
        exacto = numero.quantize(CENTIMOS)
    except InvalidOperation as error:
        raise GastoInvalido(campo, "El monto es demasiado grande.") from error
    if numero != exacto:
        raise GastoInvalido(campo, "El monto no puede tener más de dos decimales.")
    if exacto > MAXIMO or exacto < -MAXIMO:
        raise GastoInvalido(campo, "El monto supera el máximo permitido.")
    if exacto < 0:
        raise GastoInvalido(campo, "El monto de una parte no puede ser negativo.")
    if not permitir_cero and exacto <= 0:
        raise GastoInvalido(campo, "El monto del gasto debe ser mayor que cero.")
    return exacto


def _validar_partes(
    partes: Sequence[ParteCalculada], monto: Decimal, regla: str
) -> None:
    if not partes:
        raise GastoInvalido(
            "partes", "El gasto necesita al menos una parte.", regla="partes"
        )

    vistos: set[UUID] = set()
    for parte in partes:
        if parte.usuario_id in vistos:
            raise GastoInvalido(
                "participantes",
                "Hay una persona repetida entre las partes.",
                regla="participantes",
            )
        vistos.add(parte.usuario_id)

    suma = sum((parte.monto for parte in partes), Decimal("0.00"))
    if suma != monto:
        raise GastoInvalido(
            "partes",
            f"Las partes suman {suma} y el gasto es {monto}.",
            regla="suma",
        )

    if regla == "porcentajes":
        _validar_porcentajes(partes, monto)
        return

    for parte in partes:
        if parte.porcentaje is not None:
            raise GastoInvalido(
                "porcentajes",
                "El porcentaje solo se usa con la regla porcentajes.",
                regla=regla,
            )
    if regla == "iguales":
        _validar_partes_iguales(partes, monto)


def _validar_porcentajes(partes: Sequence[ParteCalculada], monto: Decimal) -> None:
    for parte in partes:
        if parte.porcentaje is None:
            raise GastoInvalido(
                "porcentajes",
                "Cada parte de un gasto por porcentajes necesita su porcentaje.",
                regla="porcentajes",
            )
        if not 0 <= parte.porcentaje <= 10000:
            raise GastoInvalido(
                "porcentajes",
                "Cada porcentaje debe estar entre 0 y 10000 centésimas.",
                regla="porcentajes",
            )

    total_porcentajes = sum(
        parte.porcentaje for parte in partes if parte.porcentaje is not None
    )
    if total_porcentajes != 10000:
        raise GastoInvalido(
            "porcentajes",
            "Los porcentajes suman "
            f"{total_porcentajes} centésimas; deben sumar 10000 (100,00 %).",
            regla="porcentajes",
        )

    total_centimos = _centimos(monto)
    truncadas = 0
    extras = 0
    for parte in partes:
        assert parte.porcentaje is not None
        base = total_centimos * parte.porcentaje // 10000
        parte_centimos = _centimos(parte.monto)
        if parte_centimos not in (base, base + 1):
            raise GastoInvalido(
                "partes",
                "El importe de una parte no corresponde a su porcentaje.",
                regla="porcentajes",
            )
        truncadas += base
        extras += parte_centimos - base
    if extras != total_centimos - truncadas:
        raise GastoInvalido(
            "partes",
            "El residuo de la regla porcentajes no está repartido completo.",
            regla="porcentajes",
        )


def _validar_partes_iguales(partes: Sequence[ParteCalculada], monto: Decimal) -> None:
    total_centimos = _centimos(monto)
    base, residuo = divmod(total_centimos, len(partes))
    for parte in partes:
        parte_centimos = _centimos(parte.monto)
        if parte_centimos not in (base, base + 1):
            raise GastoInvalido(
                "partes",
                "Con la regla iguales cada parte debe ser "
                f"{base} o {base + 1} céntimos.",
                regla="iguales",
            )
    extras = sum(1 for parte in partes if _centimos(parte.monto) == base + 1)
    if extras != residuo:
        raise GastoInvalido(
            "partes",
            "El residuo de la regla iguales no está repartido completo.",
            regla="iguales",
        )


def _centimos(monto: Decimal) -> int:
    """Céntimos enteros de un Decimal ya escalado a dos decimales."""

    return int(monto * 100)


def _exigir_grupo(conexion: Connection[dict[str, Any]], grupo_id: UUID) -> None:
    if conexion.execute(SQL_EXISTE_GRUPO, (grupo_id,)).fetchone() is None:
        raise GastoInvalido("grupo_id", "El grupo no existe.")


def _exigir_miembro_aceptado(
    conexion: Connection[dict[str, Any]],
    grupo_id: UUID,
    usuario_id: UUID,
    *,
    campo: str,
    mensaje: str,
) -> None:
    fila = conexion.execute(
        SQL_EXISTE_MIEMBRO_ACEPTADO, (grupo_id, usuario_id)
    ).fetchone()
    if fila is None:
        raise GastoInvalido(campo, mensaje, regla="miembro")


def _exigir_participantes(
    conexion: Connection[dict[str, Any]],
    grupo_id: UUID,
    partes: Sequence[ParteCalculada],
) -> None:
    ids = [parte.usuario_id for parte in partes]
    filas = conexion.execute(SQL_PARTICIPANTES_ACEPTADOS, (grupo_id, ids)).fetchall()
    aceptados = {fila["usuario_id"] for fila in filas}
    faltan = [str(usuario_id) for usuario_id in ids if usuario_id not in aceptados]
    if faltan:
        raise GastoInvalido(
            "participantes",
            "No son miembros aceptados del grupo: " + ", ".join(faltan) + ".",
            regla="miembro",
        )


def _nombre_usuario(conexion: Connection[dict[str, Any]], usuario_id: UUID) -> str:
    fila = conexion.execute(SQL_NOMBRE_USUARIO, (usuario_id,)).fetchone()
    assert fila is not None
    return str(fila["nombre_usuario"])


def _sin_microsegundos(ahora: datetime) -> datetime:
    return ahora.replace(microsecond=0)


def _gasto_desde_fila(fila: dict[str, Any]) -> Gasto:
    return Gasto(
        id=fila["id"],
        grupo_id=fila["grupo_id"],
        concepto=fila["concepto"],
        monto=fila["monto"],
        fecha=fila["fecha"],
        pagador_id=fila["pagador_id"],
        regla=fila["regla"],
        registrado_por=fila["registrado_por"],
        fecha_registro=fila["fecha_registro"],
    )


def _parte_desde_fila(fila: dict[str, Any]) -> ParteGasto:
    return ParteGasto(
        gasto_id=fila["gasto_id"],
        usuario_id=fila["usuario_id"],
        monto=fila["monto"],
        lleva_residuo=fila["lleva_residuo"],
        porcentaje=fila["porcentaje"],
    )
