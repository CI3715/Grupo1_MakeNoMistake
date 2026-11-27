"""Configuración del servidor leída de variables de entorno.

El servidor no arranca si falta la conexión a PostgreSQL o si el secreto del
token es demasiado corto. Así el error aparece al iniciar y no en la primera
petición de una persona usuaria.
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv

LONGITUD_MINIMA_SECRETO = 32
MINUTOS_TOKEN_POR_DEFECTO = 60
RUTA_ENV = Path(__file__).resolve().with_name(".env")


@dataclass(frozen=True)
class Configuracion:
    """Valores que el resto del servidor necesita para funcionar."""

    url_base_datos: str
    secreto_jwt: str
    minutos_token: int


def cargar_configuracion(entorno: Mapping[str, str] | None = None) -> Configuracion:
    """Lee y valida la configuración.

    Si no se pasa un entorno explícito, carga `servidor/.env` (sin sobrescribir
    las variables que ya existan) y usa `os.environ`. Las pruebas pasan un
    diccionario para no depender del entorno de la máquina.
    """

    if entorno is None:
        load_dotenv(RUTA_ENV)
        entorno = os.environ

    url_base_datos = entorno.get("CUENTAS_CLARAS_DB_URL", "").strip()
    if not url_base_datos:
        raise RuntimeError(
            "Falta CUENTAS_CLARAS_DB_URL. Copia servidor/.env.example a "
            "servidor/.env y ajusta la conexión a PostgreSQL."
        )

    secreto_jwt = entorno.get("CUENTAS_CLARAS_SECRETO_JWT", "").strip()
    if len(secreto_jwt) < LONGITUD_MINIMA_SECRETO:
        raise RuntimeError(
            "CUENTAS_CLARAS_SECRETO_JWT debe tener al menos "
            f"{LONGITUD_MINIMA_SECRETO} caracteres. Genera uno con: "
            'python -c "import secrets; print(secrets.token_urlsafe(48))"'
        )

    minutos_crudos = entorno.get(
        "CUENTAS_CLARAS_MINUTOS_TOKEN", str(MINUTOS_TOKEN_POR_DEFECTO)
    ).strip()
    try:
        minutos_token = int(minutos_crudos)
    except ValueError as error:
        raise RuntimeError(
            "CUENTAS_CLARAS_MINUTOS_TOKEN debe ser un número entero."
        ) from error

    if minutos_token <= 0:
        raise RuntimeError("CUENTAS_CLARAS_MINUTOS_TOKEN debe ser mayor que cero.")

    return Configuracion(
        url_base_datos=url_base_datos,
        secreto_jwt=secreto_jwt,
        minutos_token=minutos_token,
    )
