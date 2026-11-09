"""Módulo de seguridad: derivación lenta de contraseñas y gestión de JWT.

Cumple con RNF-05 (argon2id con sal única) y RF-SOC-02/RES-07 (JWT firmado con HS256).
"""

from datetime import datetime, timedelta, timezone
from typing import Any

import jwt
from argon2 import PasswordHasher
from argon2.exceptions import VerificationError, VerifyMismatchError

from config import obtener_jwt_minutos, obtener_jwt_secret

# PasswordHasher usa argon2id por defecto con sal aleatoria criptográfica única
_hasheador = PasswordHasher()

ALGORITMO_JWT = "HS256"
FORMATO_FECHA_UTC = "%Y-%m-%dT%H:%M:%SZ"


class ErrorSeguridad(Exception):
    """Excepción base para fallos de seguridad y autenticación."""


class TokenInvalidoError(ErrorSeguridad):
    """El token tiene un formato inválido, fue alterado o tiene firma errónea."""


class TokenVencidoError(ErrorSeguridad):
    """El token ha expirado según su fecha exp."""


def hashear_contrasena(contrasena: str) -> str:
    """Genera un hash seguro usando argon2id con sal única."""
    return _hasheador.hash(contrasena)


def verificar_contrasena(contrasena: str, hash_guardado: str) -> bool:
    """Verifica si la contraseña coincide con el hash guardado."""
    try:
        return _hasheador.verify(hash_guardado, contrasena)
    except (VerifyMismatchError, VerificationError):
        return False


def crear_token_sesion(
    usuario_id: str,
    minutos: int | None = None,
    secreto: str | None = None,
) -> tuple[str, str]:
    """Genera un token JWT firmado y su fecha de vencimiento en UTC."""
    minutos_vigencia = minutos if minutos is not None else obtener_jwt_minutos()
    clave_secreta = secreto if secreto is not None else obtener_jwt_secret()

    ahora = datetime.now(timezone.utc)
    vencimiento = ahora + timedelta(minutes=minutos_vigencia)
    vence_en_iso = vencimiento.strftime(FORMATO_FECHA_UTC)

    payload: dict[str, Any] = {
        "sub": usuario_id,
        "iat": int(ahora.timestamp()),
        "exp": int(vencimiento.timestamp()),
    }

    token = jwt.encode(payload, clave_secreta, algorithm=ALGORITMO_JWT)
    return token, vence_en_iso


def decodificar_token_sesion(
    token: str,
    secreto: str | None = None,
) -> str:
    """Valida y decodifica un token JWT. Retorna el ID del usuario (subject).

    Lanza TokenVencidoError si expiró o TokenInvalidoError si es inválido.
    """
    clave_secreta = secreto if secreto is not None else obtener_jwt_secret()
    try:
        payload = jwt.decode(
            token,
            clave_secreta,
            algorithms=[ALGORITMO_JWT],
            options={"require": ["sub", "exp"]},
        )
        usuario_id = payload.get("sub")
        if not usuario_id or not isinstance(usuario_id, str):
            raise TokenInvalidoError(
                "El token no contiene un identificador de usuario válido."
            )
        return usuario_id
    except jwt.ExpiredSignatureError as e:
        raise TokenVencidoError("El token ha expirado.") from e
    except (jwt.InvalidTokenError, jwt.DecodeError) as e:
        raise TokenInvalidoError("El token no es válido o está alterado.") from e
