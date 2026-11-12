"""Hash de contraseñas con argon2id (RNF-05).

Cada hash lleva su propia sal aleatoria, así que la misma contraseña produce
hashes distintos. El hash guarda también sus parámetros, por lo que se puede
verificar aunque los parámetros por defecto cambien más adelante.
"""

import secrets

from argon2 import PasswordHasher, Type
from argon2.exceptions import InvalidHashError, VerificationError

_hasher = PasswordHasher(type=Type.ID)


def hashear(contrasena: str) -> str:
    return _hasher.hash(contrasena)


def verificar(hash_guardado: str, contrasena: str) -> bool:
    """True si la contraseña coincide. Nunca lanza por un hash malformado."""
    try:
        return _hasher.verify(hash_guardado, contrasena)
    except (VerificationError, InvalidHashError):
        return False


# Hash de una contraseña aleatoria que nadie conoce. POST /sesiones verifica
# contra él cuando el correo no existe, para que el tiempo de respuesta sea el
# mismo que con una contraseña incorrecta y no delate qué correos existen.
HASH_FALSO = hashear(secrets.token_urlsafe(32))
