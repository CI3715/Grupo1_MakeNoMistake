"""Modelos de datos y esquemas para las respuestas de la API.

Coinciden con los esquemas definidos en openapi.yaml.
"""

from pydantic import BaseModel, ConfigDict


class UsuarioPublico(BaseModel):
    """Datos públicos de un usuario. Nunca incluye contraseña ni hash."""

    model_config = ConfigDict(extra="forbid")

    id: str
    nombre_usuario: str
    nombre_visible: str
    correo: str
    fecha_alta: str


class SesionRespuesta(BaseModel):
    """Respuesta de sesión iniciada exitosamente."""

    model_config = ConfigDict(extra="forbid")

    token: str
    tipo: str = "Bearer"
    vence_en: str


class ErrorCampo(BaseModel):
    """Detalle del error en un campo específico."""

    model_config = ConfigDict(extra="forbid")

    campo: str
    mensaje: str


class ErroresRespuesta(BaseModel):
    """Estructura de error 400 y 409 con la lista de campos que fallaron."""

    model_config = ConfigDict(extra="forbid")

    errores: list[ErrorCampo]


class MensajeRespuesta(BaseModel):
    """Estructura para respuestas simples o errores 401."""

    model_config = ConfigDict(extra="forbid")

    mensaje: str
