from collections.abc import AsyncIterator, Sequence
from contextlib import asynccontextmanager
from datetime import datetime, timezone
from typing import Annotated, Any, Protocol
from uuid import UUID

from fastapi import Depends, FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.openapi.utils import get_openapi
from fastapi.responses import JSONResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel, ConfigDict, Field

import contrasenas
import reglas
import token_sesion
from base_datos import BaseDatos
from configuracion import cargar_configuracion
from repositorio_usuarios import RepositorioUsuarios, Usuario, UsuarioDuplicado

VERSION = "0.1.0"
FORMATO_FECHA = "%Y-%m-%dT%H:%M:%SZ"

MENSAJE_CREDENCIALES = "Correo o contraseña incorrectos."
MENSAJE_SESION_INVALIDA = "Sesión no válida o vencida."
MENSAJE_CUERPO_INVALIDO = "El cuerpo de la petición debe ser un objeto JSON."
MENSAJE_TIPO_TEXTO = "Este campo debe ser texto."
MENSAJE_DUPLICADO = {
    "nombre_usuario": "Ese nombre de usuario ya está registrado.",
    "correo": "Ese correo ya está registrado.",
}


class AlmacenUsuarios(Protocol):
    """Lo que las rutas necesitan del almacén de usuarios.

    En producción es ``RepositorioUsuarios`` (PostgreSQL). Las pruebas de las
    rutas usan uno en memoria con el mismo comportamiento.
    """

    def crear(
        self,
        *,
        nombre_usuario: str,
        nombre_visible: str,
        correo: str,
        hash_contrasena: str,
    ) -> Usuario: ...

    def buscar_por_correo(self, correo: str) -> Usuario | None: ...

    def buscar_por_nombre_usuario(self, nombre_usuario: str) -> Usuario | None: ...

    def buscar_por_id(self, id_usuario: UUID) -> Usuario | None: ...


@asynccontextmanager
async def ciclo_de_vida(app: FastAPI) -> AsyncIterator[None]:
    """Conecta con PostgreSQL y comprueba la configuración al arrancar.

    Si falta ``CUENTAS_CLARAS_DB_URL`` o el secreto del token, el servidor no
    arranca. Las pruebas de las rutas dejan un almacén en memoria en
    ``app.state.repositorio_usuarios`` antes de arrancar; en ese caso no se
    abre ninguna conexión.
    """
    if getattr(app.state, "repositorio_usuarios", None) is None:
        configuracion = cargar_configuracion()  # también carga servidor/.env
        base_datos = BaseDatos(configuracion.url_base_datos)
        base_datos.crear_esquema()
        app.state.repositorio_usuarios = RepositorioUsuarios(base_datos)
    token_sesion.comprobar_configuracion()

    yield


app = FastAPI(title="Cuentas Claras API", version=VERSION, lifespan=ciclo_de_vida)

esquema_bearer = HTTPBearer(auto_error=False, scheme_name="tokenBearer")
# ---------------------------------------------------------------- Modelos


class PingRespuesta(BaseModel):
    estado: str
    mensaje: str
    version: str
    timestamp: str


class RegistroUsuario(BaseModel):
    # Los campos son opcionales para el modelo porque las reglas propias
    # (reglas.py) informan todos los campos que faltan o fallan a la vez.
    model_config = ConfigDict(
        json_schema_extra={
            "required": ["nombre_usuario", "nombre_visible", "correo", "contrasena"]
        }
    )

    nombre_usuario: str | None = Field(default=None, examples=["ana.perez"])
    nombre_visible: str | None = Field(default=None, examples=["Ana Pérez"])
    correo: str | None = Field(default=None, examples=["ana@example.com"])
    contrasena: str | None = Field(
        default=None, json_schema_extra={"format": "password", "writeOnly": True}
    )


class Credenciales(BaseModel):
    model_config = ConfigDict(json_schema_extra={"required": ["correo", "contrasena"]})

    correo: str | None = Field(default=None, examples=["ana@example.com"])
    contrasena: str | None = Field(
        default=None, json_schema_extra={"format": "password", "writeOnly": True}
    )


class UsuarioRespuesta(BaseModel):
    """Usuario sin contraseña ni hash."""

    id: str
    nombre_usuario: str
    nombre_visible: str
    correo: str
    fecha_alta: str


class SesionRespuesta(BaseModel):
    token: str
    tipo: str = "Bearer"
    vence_en: str


class ErrorCampoRespuesta(BaseModel):
    campo: str
    mensaje: str


class ErroresPorCampo(BaseModel):
    errores: list[ErrorCampoRespuesta]


class ErrorMensaje(BaseModel):
    mensaje: str


# ---------------------------------------------------------------- Errores


def respuesta_errores(
    codigo: int, errores: Sequence[reglas.ErrorCampo]
) -> JSONResponse:
    return JSONResponse(
        status_code=codigo,
        content={
            "errores": [{"campo": e.campo, "mensaje": e.mensaje} for e in errores]
        },
    )


@app.exception_handler(RequestValidationError)
async def manejar_validacion(_: Request, error: RequestValidationError) -> JSONResponse:
    """Responde 400 con el campo que falla en lugar del 422 de FastAPI.

    El 422 por defecto incluye el valor recibido (``input``), contraseña
    incluida. Aquí solo se devuelven el campo y un mensaje propio (RNF-05).
    """
    errores: list[reglas.ErrorCampo] = []
    vistos: set[str] = set()
    for detalle in error.errors():
        ubicacion: tuple[Any, ...] = tuple(detalle.get("loc", ()))
        # Un JSON malformado llega con la posición del error (un número) en
        # lugar del nombre de un campo.
        if (
            len(ubicacion) >= 2
            and ubicacion[0] == "body"
            and isinstance(ubicacion[1], str)
        ):
            campo, mensaje = str(ubicacion[1]), MENSAJE_TIPO_TEXTO
        else:
            campo, mensaje = "cuerpo", MENSAJE_CUERPO_INVALIDO
        if campo not in vistos:
            vistos.add(campo)
            errores.append(reglas.ErrorCampo(campo, mensaje))
    return respuesta_errores(400, errores)


def no_autorizado(mensaje: str, con_desafio: bool) -> JSONResponse:
    encabezados = {"WWW-Authenticate": "Bearer"} if con_desafio else None
    return JSONResponse(
        status_code=401, content={"mensaje": mensaje}, headers=encabezados
    )


def a_respuesta(usuario: Usuario) -> UsuarioRespuesta:
    return UsuarioRespuesta(
        id=str(usuario.id),
        nombre_usuario=usuario.nombre_usuario,
        nombre_visible=usuario.nombre_visible,
        correo=usuario.correo,
        # PostgreSQL devuelve la fecha en la zona horaria de la sesión.
        fecha_alta=usuario.fecha_alta.astimezone(timezone.utc).strftime(FORMATO_FECHA),
    )


def obtener_repositorio(request: Request) -> AlmacenUsuarios:
    repositorio: AlmacenUsuarios = request.app.state.repositorio_usuarios
    return repositorio


Repositorio = Annotated[AlmacenUsuarios, Depends(obtener_repositorio)]


def campos_ocupados(
    repositorio: AlmacenUsuarios, nombre_usuario: str, correo: str
) -> list[str]:
    """Campos del registro que ya usa otra persona, en el orden del formulario."""
    campos = []
    if repositorio.buscar_por_nombre_usuario(nombre_usuario) is not None:
        campos.append("nombre_usuario")
    if repositorio.buscar_por_correo(correo) is not None:
        campos.append("correo")
    return campos


def respuesta_duplicado(campos: list[str]) -> JSONResponse:
    return respuesta_errores(
        409, [reglas.ErrorCampo(c, MENSAJE_DUPLICADO[c]) for c in campos]
    )


# ---------------------------------------------------------------- Rutas


@app.get("/ping", response_model=PingRespuesta, tags=["Estado"], operation_id="ping")
def ping() -> PingRespuesta:
    return PingRespuesta(
        estado="ok",
        mensaje="Servidor Cuentas Claras activo",
        version=VERSION,
        timestamp=datetime.now(timezone.utc).strftime(FORMATO_FECHA),
    )


@app.post(
    "/usuarios",
    status_code=201,
    response_model=UsuarioRespuesta,
    tags=["Usuarios"],
    summary="Registrarse",
    operation_id="registrarUsuario",
    responses={400: {"model": ErroresPorCampo}, 409: {"model": ErroresPorCampo}},
)
def registrar_usuario(
    cuerpo: RegistroUsuario, repositorio: Repositorio
) -> UsuarioRespuesta | JSONResponse:
    datos, errores = reglas.validar_registro(
        cuerpo.nombre_usuario, cuerpo.nombre_visible, cuerpo.correo, cuerpo.contrasena
    )
    if datos is None:
        return respuesta_errores(400, errores)
    # Se consulta antes de insertar para informar todos los campos repetidos a
    # la vez; PostgreSQL solo reporta la primera restricción que falla.
    ocupados = campos_ocupados(repositorio, datos.nombre_usuario, datos.correo)
    if ocupados:
        return respuesta_duplicado(ocupados)
    try:
        usuario = repositorio.crear(
            nombre_usuario=datos.nombre_usuario,
            nombre_visible=datos.nombre_visible,
            correo=datos.correo,
            hash_contrasena=contrasenas.hashear(datos.contrasena),
        )
    except UsuarioDuplicado as duplicado:
        # Otra petición registró los mismos datos entre la consulta y el
        # INSERT. La restricción UNIQUE lo detecta.
        ocupados = campos_ocupados(repositorio, datos.nombre_usuario, datos.correo)
        return respuesta_duplicado(ocupados or [duplicado.campo])
    return a_respuesta(usuario)


@app.post(
    "/sesiones",
    response_model=SesionRespuesta,
    tags=["Sesiones"],
    summary="Iniciar sesión",
    operation_id="iniciarSesion",
    responses={400: {"model": ErroresPorCampo}, 401: {"model": ErrorMensaje}},
)
def iniciar_sesion(
    cuerpo: Credenciales, repositorio: Repositorio
) -> SesionRespuesta | JSONResponse:
    errores = reglas.validar_credenciales(cuerpo.correo, cuerpo.contrasena)
    if errores:
        return respuesta_errores(400, errores)
    assert cuerpo.correo is not None and cuerpo.contrasena is not None

    usuario = repositorio.buscar_por_correo(reglas.normalizar_correo(cuerpo.correo))
    # Siempre se calcula un argon2, exista o no el usuario, para que el
    # tiempo de respuesta no revele qué correos están registrados.
    hash_a_comparar = usuario.hash_contrasena if usuario else contrasenas.HASH_FALSO
    coincide = contrasenas.verificar(hash_a_comparar, cuerpo.contrasena)
    if usuario is None or not coincide:
        return no_autorizado(MENSAJE_CREDENCIALES, con_desafio=False)

    emitido = token_sesion.crear_token(str(usuario.id))
    return SesionRespuesta(
        token=emitido.token,
        tipo="Bearer",
        vence_en=emitido.vence_en.strftime(FORMATO_FECHA),
    )


@app.get(
    "/usuarios/yo",
    response_model=UsuarioRespuesta,
    tags=["Usuarios"],
    summary="Perfil propio",
    operation_id="obtenerPerfil",
    responses={401: {"model": ErrorMensaje}},
)
def obtener_perfil(
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repositorio: Repositorio,
) -> UsuarioRespuesta | JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    usuario = repositorio.buscar_por_id(id_usuario) if id_usuario else None
    if usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    return a_respuesta(usuario)


def id_desde_token(token: str) -> UUID | None:
    """El id de usuario del token, o None si el token no sirve.

    Un ``sub`` que no es un UUID se trata como token inválido (401) en lugar de
    llegar a PostgreSQL, que lo rechazaría con un error.
    """
    sub = token_sesion.verificar_token(token)
    if sub is None:
        return None
    try:
        return UUID(sub)
    except ValueError:
        return None


def esquema_openapi() -> dict[str, Any]:
    """OpenAPI sin el 422 que FastAPI añade solo: aquí nunca se responde 422."""
    if app.openapi_schema is None:
        esquema = get_openapi(title=app.title, version=app.version, routes=app.routes)
        for ruta in esquema["paths"].values():
            for operacion in ruta.values():
                operacion["responses"].pop("422", None)
        esquemas = esquema.get("components", {}).get("schemas", {})
        esquemas.pop("HTTPValidationError", None)
        esquemas.pop("ValidationError", None)
        app.openapi_schema = esquema
    return app.openapi_schema


app.openapi = esquema_openapi  # type: ignore[method-assign]
