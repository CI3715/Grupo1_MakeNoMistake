from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from datetime import datetime, timezone

from fastapi import FastAPI
from pydantic import BaseModel

from base_datos import BaseDatos
from configuracion import cargar_configuracion
from repositorio_usuarios import RepositorioUsuarios

VERSION = "0.1.0"


@asynccontextmanager
async def ciclo_de_vida(app: FastAPI) -> AsyncIterator[None]:
    """Prepara la base de datos al arrancar y la deja disponible en `app.state`.

    Si falta configuración o PostgreSQL no responde, el servidor no arranca.
    El esquema es idempotente: se puede ejecutar en cada inicio.
    """

    configuracion = cargar_configuracion()
    base_datos = BaseDatos(configuracion.url_base_datos)
    base_datos.crear_esquema()

    app.state.configuracion = configuracion
    app.state.base_datos = base_datos
    app.state.repositorio_usuarios = RepositorioUsuarios(base_datos)

    yield


app = FastAPI(title="Cuentas Claras API", version=VERSION, lifespan=ciclo_de_vida)


class PingRespuesta(BaseModel):
    estado: str
    mensaje: str
    version: str
    timestamp: str


@app.get("/ping", response_model=PingRespuesta)
def ping() -> PingRespuesta:
    return PingRespuesta(
        estado="ok",
        mensaje="Servidor Cuentas Claras activo",
        version=VERSION,
        timestamp=datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    )
