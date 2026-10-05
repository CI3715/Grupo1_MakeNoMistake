from datetime import datetime, timezone

from fastapi import FastAPI
from pydantic import BaseModel

VERSION = "0.1.0"

app = FastAPI(title="Cuentas Claras API", version=VERSION)


class PingRespuesta(BaseModel):
    estado: str
    mensaje: str
    version: str
    timestamp: str


@app.get("/ping", response_model=PingRespuesta)
def ping():
    return PingRespuesta(
        estado="ok",
        mensaje="Servidor Cuentas Claras activo",
        version=VERSION,
        timestamp=datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    )