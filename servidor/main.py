"""Punto de entrada de la API del servidor de Cuentas Claras."""

from datetime import datetime, timezone
from typing import Annotated

from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from autenticacion import ErrorAutenticacion, obtener_usuario_actual
from modelos import MensajeRespuesta, UsuarioPublico

VERSION = "0.1.0"

app = FastAPI(title="Cuentas Claras API", version=VERSION)


class PingRespuesta(BaseModel):
    estado: str
    mensaje: str
    version: str
    timestamp: str


@app.exception_handler(ErrorAutenticacion)
def manejador_error_autenticacion(
    _request: Request, exc: ErrorAutenticacion
) -> JSONResponse:
    """Maneja fallos de autenticación 401 según el contrato OpenAPI."""
    return JSONResponse(
        status_code=401,
        content={"mensaje": exc.mensaje},
        headers={"WWW-Authenticate": "Bearer"},
    )


@app.exception_handler(HTTPException)
def manejador_http_exception(_request: Request, exc: HTTPException) -> JSONResponse:
    """Asegura que los errores HTTP tengan la estructura esperada por el cliente."""
    cabeceras = dict(exc.headers or {})
    if exc.status_code == 401:
        if "WWW-Authenticate" not in cabeceras:
            cabeceras["WWW-Authenticate"] = "Bearer"
        return JSONResponse(
            status_code=401,
            content={"mensaje": str(exc.detail)},
            headers=cabeceras,
        )
    return JSONResponse(
        status_code=exc.status_code,
        content={"mensaje": str(exc.detail)},
        headers=cabeceras,
    )


@app.get("/ping", response_model=PingRespuesta, tags=["Estado"])
def ping() -> PingRespuesta:
    """Comprueba que el servidor está activo."""
    return PingRespuesta(
        estado="ok",
        mensaje="Servidor Cuentas Claras activo",
        version=VERSION,
        timestamp=datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    )


@app.get(
    "/usuarios/yo",
    response_model=UsuarioPublico,
    responses={401: {"model": MensajeRespuesta}},
    tags=["Usuarios"],
)
def obtener_perfil(
    usuario_actual: Annotated[UsuarioPublico, Depends(obtener_usuario_actual)],
) -> UsuarioPublico:
    """Retorna los datos del usuario autenticado en la sesión actual."""
    return usuario_actual
