#!/usr/bin/env python3
"""Reconstruye los saldos de una exportación sin cifrar de Cuentas Claras (RNF-04).

Es independiente de la aplicación: solo usa la biblioteca estándar de Python (3.9 o
posterior) y sigue el formato documentado en ``docs/formato-exportacion.md``.

Uso::

    python3 herramientas/reconstruir_saldos.py mi-copia.txt
    python3 herramientas/reconstruir_saldos.py mi-copia.txt --json

Imprime el saldo de cada cuenta y el total por moneda. Con ``--json`` imprime lo mismo
en JSON, que es lo que usa la prueba automatizada que compara estos saldos con los de la
aplicación. Si el archivo no es válido, no imprime saldos: explica el problema y
termina con código 2.

La regla es la de la aplicación (INV-05)::

    saldo = saldo_inicial + ingresos - gastos + efectos de las correcciones

Los movimientos nunca se editan: una corrección (``MovimientoCorregido``) suma al
saldo su ``efecto``, la diferencia entre el monto anterior y el correcto. Las partes
de gastos de grupo (``GastoDeGrupoAplicado``) son gastos. Todo se calcula con
``decimal.Decimal``, sin coma flotante.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

MARCA = "# cuentas-claras-respaldo"
VERSION_SOPORTADA = 1
ENCABEZADO = re.compile(
    r"^# cuentas-claras-respaldo version=(0|[1-9][0-9]*)"
    r" exportado=([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?Z)"
    r" eventos=(0|[1-9][0-9]*)$"
)
CLAVES_EVENTO = {"id", "tipo", "momento_registro", "dispositivo_origen", "datos"}

# Monedas de fábrica y sus decimales; las propias llegan en eventos MonedaCreada.
DECIMALES_DE_FABRICA = {"VES": 2, "USD": 2, "EUR": 2, "USDT": 2}


class ArchivoInvalido(Exception):
    """El archivo no cumple el formato: no se reconstruye nada."""


@dataclass
class Cuenta:
    id: str
    nombre: str
    moneda: str
    saldo_inicial: Decimal


@dataclass
class Movimiento:
    id: str
    cuenta_id: str
    tipo: str  # "ingreso" o "egreso"
    monto: Decimal


@dataclass
class Correccion:
    id_movimiento_original: str
    efecto: Decimal


def _decimal(texto: Any, que: str) -> Decimal:
    if not isinstance(texto, str):
        raise ArchivoInvalido(f"{que} debe ser un texto con un número decimal")
    try:
        valor = Decimal(texto)
    except InvalidOperation:
        raise ArchivoInvalido(f"{que} no es un número decimal: {texto!r}") from None
    if not valor.is_finite():
        raise ArchivoInvalido(f"{que} no es un número decimal: {texto!r}")
    return valor


def _texto(datos: dict[str, Any], *claves: str) -> str:
    """El primer campo de texto presente entre ``claves`` (hay nombres alternativos)."""
    for clave in claves:
        valor = datos.get(clave)
        if isinstance(valor, str):
            return valor
    raise ArchivoInvalido(f"falta el campo {claves[0]}")


def _tipo_movimiento(texto: str) -> str:
    if texto == "ingreso":
        return "ingreso"
    if texto in ("egreso", "gasto"):
        return "egreso"
    raise ArchivoInvalido(f"tipo de movimiento desconocido: {texto!r}")


def leer_exportacion(texto: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Valida el archivo completo y devuelve (encabezado, eventos). Todo o nada."""
    if not texto.endswith("\n"):
        raise ArchivoInvalido("el archivo debe terminar con un salto de línea")
    lineas = texto[:-1].split("\n")
    primera = lineas[0]
    if not primera.startswith(MARCA):
        raise ArchivoInvalido("no es una exportación de Cuentas Claras")
    coincide = ENCABEZADO.match(primera)
    if coincide is None:
        otra = re.match(r"^# cuentas-claras-respaldo version=([0-9]+) ", primera)
        if otra and int(otra.group(1)) != VERSION_SOPORTADA:
            raise ArchivoInvalido(
                f"el archivo es de la versión {otra.group(1)} del formato y "
                f"esta herramienta soporta la {VERSION_SOPORTADA}"
            )
        raise ArchivoInvalido("el encabezado no tiene el formato esperado")
    version, exportado, cantidad = coincide.groups()
    if int(version) != VERSION_SOPORTADA:
        raise ArchivoInvalido(
            f"el archivo es de la versión {version} del formato y "
            f"esta herramienta soporta la {VERSION_SOPORTADA}"
        )

    eventos: list[dict[str, Any]] = []
    vistos: set[str] = set()
    for numero, linea in enumerate(lineas[1:], start=2):
        try:
            evento = json.loads(linea)
        except json.JSONDecodeError:
            raise ArchivoInvalido(f"la línea {numero} no es JSON válido") from None
        if not isinstance(evento, dict) or set(evento) != CLAVES_EVENTO:
            raise ArchivoInvalido(f"la línea {numero} no tiene las claves de un evento")
        for clave in ("id", "tipo", "momento_registro", "dispositivo_origen"):
            if not isinstance(evento[clave], str):
                raise ArchivoInvalido(f"la línea {numero}: {clave} debe ser texto")
        if not evento["id"] or not evento["tipo"] or not evento["momento_registro"]:
            raise ArchivoInvalido(f"la línea {numero} tiene un campo obligatorio vacío")
        if evento["id"] in vistos:
            raise ArchivoInvalido(f"la línea {numero} repite el id {evento['id']}")
        vistos.add(evento["id"])
        datos = evento["datos"]
        if isinstance(datos, str):
            # Una carga que no era objeto JSON viaja como cadena (no debería pasar).
            try:
                datos = json.loads(datos)
            except json.JSONDecodeError:
                datos = None
        elif not isinstance(datos, dict):
            raise ArchivoInvalido(f"la línea {numero}: datos debe ser un objeto")
        evento["datos"] = datos
        eventos.append(evento)

    if len(eventos) != int(cantidad):
        raise ArchivoInvalido(
            f"el encabezado anuncia {cantidad} eventos "
            f"y el archivo tiene {len(eventos)}"
        )
    encabezado = {
        "version": int(version),
        "exportado": exportado,
        "eventos": int(cantidad),
    }
    return encabezado, eventos


def reconstruir(eventos: list[dict[str, Any]]) -> dict[str, Any]:
    """Aplica los eventos en el orden del archivo y calcula saldos y totales."""
    decimales = dict(DECIMALES_DE_FABRICA)
    cuentas: dict[str, Cuenta] = {}
    movimientos: list[Movimiento] = []
    correcciones: list[Correccion] = []

    for evento in eventos:
        tipo, datos = evento["tipo"], evento["datos"]
        if not isinstance(datos, dict):
            continue
        if tipo == "MonedaCreada":
            decimales[_texto(datos, "codigo")] = int(datos["decimales"])
        elif tipo == "CuentaCreada":
            cuenta = Cuenta(
                id=_texto(datos, "id"),
                nombre=_texto(datos, "nombre"),
                moneda=_texto(datos, "moneda"),
                saldo_inicial=_decimal(datos.get("saldo_inicial"), "saldo_inicial"),
            )
            cuentas[cuenta.id] = cuenta
        elif tipo == "MovimientoRegistrado":
            movimientos.append(
                Movimiento(
                    id=_texto(datos, "id", "id_movimiento"),
                    cuenta_id=_texto(datos, "cuenta_id", "id_cuenta"),
                    tipo=_tipo_movimiento(_texto(datos, "tipo")),
                    monto=_decimal(datos.get("monto"), "monto"),
                )
            )
        elif tipo == "GastoDeGrupoAplicado":
            movimientos.append(
                Movimiento(
                    id=_texto(datos, "id_evento_servidor"),
                    cuenta_id=_texto(datos, "id_cuenta"),
                    tipo="egreso",
                    monto=_decimal(datos.get("monto"), "monto"),
                )
            )
        elif tipo == "MovimientoCorregido":
            # El movimiento original se conserva tal cual (INV-04); la corrección suma
            # su efecto al saldo. Sin `efecto` (bases anteriores) no cambia el saldo.
            correcciones.append(
                Correccion(
                    id_movimiento_original=_texto(
                        datos, "id_movimiento_original", "movimiento_original_id"
                    ),
                    efecto=_decimal(datos.get("efecto", "0"), "efecto"),
                )
            )
        # CategoriaCreada, TasaRegistrada, TasaEliminada y MonedaBaseCambiada no
        # cambian ningún saldo.

    saldos: dict[str, Decimal] = {c.id: c.saldo_inicial for c in cuentas.values()}
    cuenta_de_movimiento: dict[str, str] = {}
    for m in movimientos:
        if m.cuenta_id not in saldos:
            raise ArchivoInvalido(f"el movimiento {m.id} es de una cuenta desconocida")
        saldos[m.cuenta_id] += m.monto if m.tipo == "ingreso" else -m.monto
        cuenta_de_movimiento.setdefault(m.id, m.cuenta_id)
    for c in correcciones:
        # Una corrección de un movimiento que no está en el libro no cuenta.
        cuenta_id = cuenta_de_movimiento.get(c.id_movimiento_original)
        if cuenta_id is not None:
            saldos[cuenta_id] += c.efecto

    def formato(valor: Decimal, moneda: str) -> str:
        if moneda not in decimales:
            raise ArchivoInvalido(f"moneda desconocida: {moneda}")
        return f"{valor:.{decimales[moneda]}f}"

    totales: dict[str, Decimal] = {}
    resultado_cuentas = []
    for cuenta in cuentas.values():
        saldo = saldos[cuenta.id]
        totales[cuenta.moneda] = totales.get(cuenta.moneda, Decimal(0)) + saldo
        resultado_cuentas.append(
            {
                "id": cuenta.id,
                "nombre": cuenta.nombre,
                "moneda": cuenta.moneda,
                "saldo": formato(saldo, cuenta.moneda),
            }
        )
    return {
        "cuentas": resultado_cuentas,
        "totales": [
            {"moneda": moneda, "total": formato(total, moneda)}
            for moneda, total in sorted(totales.items())
        ],
    }


def imprimir_tabla(encabezado: dict[str, Any], resultado: dict[str, Any]) -> None:
    print(
        f"Exportación del {encabezado['exportado']} ({encabezado['eventos']} eventos)"
    )
    print()
    filas = [(c["nombre"], c["moneda"], c["saldo"]) for c in resultado["cuentas"]]
    ancho = max([len("Cuenta")] + [len(f[0]) for f in filas])
    print(f"{'Cuenta':<{ancho}}  Moneda  {'Saldo':>18}")
    for nombre, moneda, saldo in filas:
        print(f"{nombre:<{ancho}}  {moneda:<6}  {saldo:>18}")
    print()
    print("Total por moneda")
    for t in resultado["totales"]:
        print(f"  {t['moneda']:<6}  {t['total']:>18}")


def main(argumentos: list[str] | None = None) -> int:
    lector = argparse.ArgumentParser(
        description=(
            "Reconstruye los saldos de una exportación sin cifrar de Cuentas Claras."
        )
    )
    lector.add_argument(
        "archivo", help="exportación sin cifrar (.txt); «-» lee la entrada"
    )
    lector.add_argument(
        "--json", action="store_true", help="imprime el resultado en JSON"
    )
    opciones = lector.parse_args(argumentos)
    # En Windows la salida redirigida no siempre es UTF-8 y los nombres llevan tildes.
    for flujo in (sys.stdout, sys.stderr):
        reconfigurar = getattr(flujo, "reconfigure", None)
        if reconfigurar is not None:
            reconfigurar(encoding="utf-8")

    try:
        if opciones.archivo == "-":
            crudo = sys.stdin.buffer.read()
        else:
            crudo = Path(opciones.archivo).read_bytes()
    except OSError as error:
        print(f"No se pudo leer el archivo: {error.strerror}", file=sys.stderr)
        return 2

    try:
        try:
            texto = crudo.decode("utf-8")
        except UnicodeDecodeError:
            raise ArchivoInvalido(
                "el archivo no es texto UTF-8 (¿es una copia cifrada?)"
            ) from None
        encabezado, eventos = leer_exportacion(texto)
        resultado = reconstruir(eventos)
    except (ArchivoInvalido, KeyError, ValueError, TypeError) as error:
        print(f"Archivo inválido: {error}", file=sys.stderr)
        return 2

    if opciones.json:
        salida = {"encabezado": encabezado, **resultado}
        print(json.dumps(salida, ensure_ascii=False, indent=2))
    else:
        imprimir_tabla(encabezado, resultado)
    return 0


if __name__ == "__main__":
    sys.exit(main())
