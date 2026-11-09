"""Token de sesión firmado (JWT con HS256) para RF-SOC-02.

Configuración por variables de entorno:

- ``JWT_SECRET``: secreto de firma. Obligatorio y sin valor por defecto; debe
  tener al menos 32 caracteres.
- ``JWT_MINUTOS``: vigencia en minutos. 60 por defecto; 1 para la demostración
  del vencimiento.

El módulo se llama ``token_sesion`` y no ``token`` para no tapar el módulo
``token`` de la biblioteca estándar.
"""

import os
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone

import jwt

ALGORITMO = "HS256"
MINUTOS_POR_DEFECTO = 60
LONGITUD_MINIMA_SECRETO = 32


class ConfiguracionInvalida(RuntimeError):
    """Falta el secreto o la vigencia no es un entero positivo."""


@dataclass(frozen=True)
class TokenEmitido:
    token: str
    vence_en: datetime


def leer_secreto() -> str:
    secreto = os.environ.get("JWT_SECRET", "")
    if len(secreto) < LONGITUD_MINIMA_SECRETO:
        raise ConfiguracionInvalida(
            "La variable de entorno JWT_SECRET es obligatoria y debe tener "
            f"al menos {LONGITUD_MINIMA_SECRETO} caracteres."
        )
    return secreto


def leer_minutos() -> int:
    texto = os.environ.get("JWT_MINUTOS", "").strip()
    if texto == "":
        return MINUTOS_POR_DEFECTO
    try:
        minutos = int(texto)
    except ValueError:
        minutos = 0
    if minutos <= 0:
        raise ConfiguracionInvalida(
            "La variable de entorno JWT_MINUTOS debe ser un entero positivo."
        )
    return minutos


def comprobar_configuracion() -> None:
    """Falla al arrancar si la configuración no sirve, no en la primera sesión."""
    leer_secreto()
    leer_minutos()


def crear_token(id_usuario: str, ahora: datetime | None = None) -> TokenEmitido:
    """Firma un token con el id del usuario (``sub``) y su vencimiento (``exp``)."""
    emitido = (ahora or datetime.now(timezone.utc)).replace(microsecond=0)
    vence_en = emitido + timedelta(minutes=leer_minutos())
    carga = {"sub": id_usuario, "iat": emitido, "exp": vence_en}
    token = jwt.encode(carga, leer_secreto(), algorithm=ALGORITMO)
    return TokenEmitido(token=token, vence_en=vence_en)


def verificar_token(token: str) -> str | None:
    """Devuelve el id del usuario si el token es válido; si no, None.

    Rechaza tokens alterados, firmados con otro secreto o algoritmo, vencidos,
    o sin ``sub`` o ``exp``.
    """
    try:
        carga = jwt.decode(
            token,
            leer_secreto(),
            algorithms=[ALGORITMO],
            options={"require": ["sub", "exp"]},
        )
    except jwt.InvalidTokenError:
        return None
    id_usuario = carga.get("sub")
    if not isinstance(id_usuario, str) or id_usuario == "":
        return None
    return id_usuario
