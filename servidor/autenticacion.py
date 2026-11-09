"""Dependencia de autenticación Bearer para rutas protegidas."""

from typing import Annotated

from fastapi import Header

from base_datos import buscar_usuario_por_id
from modelos import UsuarioPublico
from seguridad import (
    ErrorSeguridad,
    decodificar_token_sesion,
)


class ErrorAutenticacion(Exception):
    """Excepción para peticiones con credenciales de sesión ausentes o inválidas."""

    def __init__(self, mensaje: str = "Sesión no válida o vencida.") -> None:
        self.mensaje = mensaje
        super().__init__(mensaje)


def obtener_usuario_actual(
    authorization: Annotated[str | None, Header()] = None,
) -> UsuarioPublico:
    """Extrae y valida el token Bearer del encabezado Authorization.

    Retorna el modelo público del usuario si el token es válido y vigente.
    Lanza ErrorAutenticacion (401) si falta el token, si el esquema no es Bearer,
    si el token está vencido, corrupto, firmado con otro secreto o no existe.
    """
    if not authorization:
        raise ErrorAutenticacion("Sesión no válida o vencida.")

    partes = authorization.split(" ", 1)
    if len(partes) != 2 or partes[0] != "Bearer":
        raise ErrorAutenticacion("Sesión no válida o vencida.")

    token = partes[1].strip()
    if not token:
        raise ErrorAutenticacion("Sesión no válida o vencida.")

    try:
        usuario_id = decodificar_token_sesion(token)
    except ErrorSeguridad as e:
        raise ErrorAutenticacion("Sesión no válida o vencida.") from e

    datos_usuario = buscar_usuario_por_id(usuario_id)
    if not datos_usuario:
        raise ErrorAutenticacion("Sesión no válida o vencida.")

    return UsuarioPublico(
        id=str(datos_usuario["id"]),
        nombre_usuario=str(datos_usuario["nombre_usuario"]),
        nombre_visible=str(datos_usuario["nombre_visible"]),
        correo=str(datos_usuario["correo"]),
        fecha_alta=str(datos_usuario["fecha_alta"]),
    )
