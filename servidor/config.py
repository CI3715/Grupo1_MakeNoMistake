"""Configuración del servidor mediante variables de entorno."""

import os
from pathlib import Path


def obtener_jwt_secret() -> str:
    """Obtiene el secreto para la firma de tokens JWT.

    En producción es obligatorio definir JWT_SECRET en las variables de entorno.
    """
    secreto = os.getenv("JWT_SECRET")
    if not secreto:
        raise RuntimeError(
            "La variable de entorno JWT_SECRET no está configurada. "
            "El servidor no puede operar sin un secreto de firma."
        )
    return secreto


def obtener_jwt_minutos() -> int:
    """Obtiene la vigencia en minutos de las claves de acceso temporal (60)."""
    valor = os.getenv("JWT_MINUTOS", "60")
    try:
        minutos = int(valor)
        if minutos <= 0:
            return 60
        return minutos
    except ValueError:
        return 60


def obtener_ruta_bd() -> Path:
    """Obtiene la ruta al archivo de base de datos SQLite de usuarios."""
    ruta = os.getenv("BD_USUARIOS_PATH")
    if not ruta:
        ruta = os.getenv("SQLITE_DB_PATH", "usuarios.db")
    return Path(ruta)
