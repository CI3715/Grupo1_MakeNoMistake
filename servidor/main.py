from collections.abc import AsyncIterator, Sequence
from contextlib import asynccontextmanager
from datetime import date, datetime, timezone
from typing import Annotated, Any, Protocol
from uuid import UUID

from fastapi import Depends, FastAPI, Request, Response
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
from repositorio_amistades import (
    Amistad,
    AmistadVista,
    AmistadYaRespondida,
    EstadoAmistad,
    InvitacionPendiente,
    RepositorioAmistades,
    YaSonAmigos,
)
from repositorio_grupos import (
    EstadoMiembro,
    Grupo,
    GrupoVista,
    Miembro,
    MiembroVista,
    RepositorioGrupos,
    Rol,
)
from repositorio_usuarios import RepositorioUsuarios, Usuario, UsuarioDuplicado

VERSION = "0.1.0"
FORMATO_FECHA = "%Y-%m-%dT%H:%M:%SZ"

MENSAJE_CREDENCIALES = "Correo o contraseña incorrectos."
MENSAJE_SESION_INVALIDA = "Sesión no válida o vencida."
MENSAJE_CUERPO_INVALIDO = "El cuerpo de la petición debe ser un objeto JSON."
MENSAJE_TIPO_TEXTO = "Este campo debe ser texto."
MENSAJE_GRUPO_NO_ENCONTRADO = "Grupo no encontrado."
MENSAJE_NO_ADMIN = "Solo el administrador puede invitar miembros."
MENSAJE_NO_AMIGOS = "Solo puedes invitar a personas que sean tus amigos."
MENSAJE_YA_MIEMBRO = (
    "El usuario ya pertenece al grupo o tiene una invitación pendiente."
)
MENSAJE_RESPUESTA_INVALIDA = "La respuesta debe ser 'aceptar' o 'rechazar'."
MENSAJE_DUPLICADO = {
    "nombre_usuario": "Ese nombre de usuario ya está registrado.",
    "correo": "Ese correo ya está registrado.",
}


class AlmacenUsuarios(Protocol):
    """Lo que las rutas necesitan del almacén de usuarios."""

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


class AlmacenGrupos(Protocol):
    """Lo que las rutas necesitan del almacén de grupos y miembros."""

    def crear_grupo(
        self,
        *,
        creado_por: UUID,
        nombre: str,
        descripcion: str | None,
        moneda: str,
        actividad: str | None,
        fecha_actividad: date | None,
        ahora: datetime,
    ) -> Grupo: ...

    def buscar_grupo_por_id(self, id_grupo: UUID) -> Grupo | None: ...

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None: ...

    def listar_grupos_de(self, usuario_id: UUID) -> list[GrupoVista]: ...

    def listar_miembros_de_grupo(self, id_grupo: UUID) -> list[MiembroVista]: ...

    def invitar_a_grupo(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        invitado_por: UUID,
        ahora: datetime,
    ) -> Miembro: ...

    def responder_invitacion_grupo(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        aceptar: bool,
        ahora: datetime,
    ) -> Miembro | None: ...


class AlmacenAmistades(Protocol):
    """Lo que las rutas necesitan del almacén de amistades."""

    def crear(
        self, *, invitador_id: UUID, invitado_id: UUID, ahora: datetime
    ) -> Amistad: ...

    def buscar_por_id(self, id_amistad: UUID) -> Amistad | None: ...

    def buscar_activa_entre(self, uno: UUID, otro: UUID) -> Amistad | None: ...

    def listar_de(self, usuario_id: UUID) -> list[AmistadVista]: ...

    def responder(
        self, *, id_amistad: UUID, aceptar: bool, ahora: datetime
    ) -> Amistad: ...

    def eliminar(self, *, id_amistad: UUID, ahora: datetime) -> None: ...

    def son_amigos(self, uno: UUID, otro: UUID) -> bool: ...


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
        app.state.repositorio_grupos = RepositorioGrupos(base_datos)
        app.state.repositorio_amistades = RepositorioAmistades(base_datos)
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


class NuevoGrupo(BaseModel):
    nombre: str | None = None
    descripcion: str | None = None
    moneda: str | None = None
    actividad: str | None = None


class InvitarMiembro(BaseModel):
    usuario: str | None = None


class ResponderInvitacionGrupo(BaseModel):
    respuesta: str | None = None


class NuevaAmistad(BaseModel):
    usuario: str | None = None


class ResponderAmistad(BaseModel):
    aceptar: bool | None = None


class UsuarioPublicoRespuesta(BaseModel):
    nombre_usuario: str
    nombre_visible: str


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


def obtener_repositorio_grupos(request: Request) -> AlmacenGrupos:
    repo: AlmacenGrupos = request.app.state.repositorio_grupos
    return repo


RepositorioGruposDep = Annotated[AlmacenGrupos, Depends(obtener_repositorio_grupos)]


def obtener_repositorio_amistades(request: Request) -> AlmacenAmistades:
    repo: AlmacenAmistades = request.app.state.repositorio_amistades
    return repo


RepositorioAmistadesDep = Annotated[
    AlmacenAmistades, Depends(obtener_repositorio_amistades)
]


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


@app.get(
    "/usuarios",
    response_model=UsuarioPublicoRespuesta,
    tags=["Usuarios"],
    summary="Buscar usuario por coincidencia exacta",
    responses={
        400: {"model": ErroresPorCampo},
        401: {"model": ErrorMensaje},
        404: {"model": ErrorMensaje},
    },
)
def buscar_usuario(
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repositorio: Repositorio,
    usuario: str | None = None,
) -> UsuarioPublicoRespuesta | JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_solicitante = id_desde_token(credenciales.credentials)
    if id_solicitante is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    if usuario is None or not usuario.strip():
        return respuesta_errores(
            400, [reglas.ErrorCampo("usuario", "El nombre de usuario es obligatorio.")]
        )

    encontrado = repositorio.buscar_por_nombre_usuario(usuario.strip())
    if encontrado is None or encontrado.nombre_usuario != usuario.strip():
        return JSONResponse(
            status_code=404,
            content={"mensaje": "Usuario no encontrado."},
        )

    return UsuarioPublicoRespuesta(
        nombre_usuario=encontrado.nombre_usuario,
        nombre_visible=encontrado.nombre_visible,
    )


@app.post(
    "/amistades",
    status_code=201,
    tags=["Amistades"],
    summary="Enviar solicitud de amistad",
    responses={
        400: {"model": ErroresPorCampo},
        401: {"model": ErrorMensaje},
        404: {"model": ErrorMensaje},
        409: {"model": ErroresPorCampo},
    },
)
def enviar_solicitud_amistad(
    cuerpo: NuevaAmistad,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_usuarios: Repositorio,
    repo_amistades: RepositorioAmistadesDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    usuario_destino = cuerpo.usuario.strip() if cuerpo.usuario else ""
    if not usuario_destino:
        return respuesta_errores(
            400, [reglas.ErrorCampo("usuario", "El nombre de usuario es obligatorio.")]
        )

    destino = repo_usuarios.buscar_por_nombre_usuario(usuario_destino)
    if destino is None:
        return JSONResponse(
            status_code=404,
            content={"mensaje": "Usuario no encontrado."},
        )

    if destino.id == id_usuario:
        return respuesta_errores(
            400,
            [
                reglas.ErrorCampo(
                    "usuario", "No puedes enviar una solicitud de amistad a ti mismo."
                )
            ],
        )

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    try:
        amistad = repo_amistades.crear(
            invitador_id=id_usuario, invitado_id=destino.id, ahora=ahora
        )
    except YaSonAmigos:
        return respuesta_errores(
            409,
            [reglas.ErrorCampo("usuario", "Ya existe una amistad con esa persona.")],
        )
    except InvitacionPendiente:
        return respuesta_errores(
            409,
            [
                reglas.ErrorCampo(
                    "usuario",
                    "Ya existe una invitación pendiente entre ambas personas.",
                )
            ],
        )

    return JSONResponse(
        status_code=201,
        content={
            "id": str(amistad.id),
            "estado": amistad.estado.value,
            "otro_id": str(destino.id),
            "otro_nombre_usuario": destino.nombre_usuario,
            "otro_nombre_visible": destino.nombre_visible,
            "enviada_por_mi": True,
            "fecha_invitacion": amistad.fecha_invitacion.strftime(FORMATO_FECHA),
            "fecha_respuesta": None,
        },
    )


@app.get(
    "/amistades",
    tags=["Amistades"],
    summary="Listar amistades e invitaciones",
    responses={401: {"model": ErrorMensaje}},
)
def listar_amistades(
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_amistades: RepositorioAmistadesDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    vistas = repo_amistades.listar_de(id_usuario)
    return JSONResponse(
        status_code=200,
        content=[
            {
                "id": str(v.id),
                "estado": v.estado.value,
                "otro_id": str(v.otro_id),
                "otro_nombre_usuario": v.otro_nombre_usuario,
                "otro_nombre_visible": v.otro_nombre_visible,
                "enviada_por_mi": v.enviada_por_mi,
                "fecha_invitacion": v.fecha_invitacion.strftime(FORMATO_FECHA),
                "fecha_respuesta": (
                    v.fecha_respuesta.strftime(FORMATO_FECHA)
                    if v.fecha_respuesta
                    else None
                ),
            }
            for v in vistas
        ],
    )


@app.post(
    "/amistades/{id_amistad}/respuesta",
    tags=["Amistades"],
    summary="Responder a solicitud de amistad",
    responses={
        400: {"model": ErroresPorCampo},
        401: {"model": ErrorMensaje},
        403: {"model": ErrorMensaje},
        404: {"model": ErrorMensaje},
        409: {"model": ErrorMensaje},
    },
)
def responder_solicitud_amistad(
    id_amistad: str,
    cuerpo: ResponderAmistad,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_amistades: RepositorioAmistadesDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_amistad = UUID(id_amistad)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": "Amistad no encontrada."}
        )

    amistad = repo_amistades.buscar_por_id(uuid_amistad)
    if amistad is None:
        return JSONResponse(
            status_code=404, content={"mensaje": "Amistad no encontrada."}
        )

    if amistad.invitado_id != id_usuario:
        return JSONResponse(
            status_code=403,
            content={
                "mensaje": "Solo el destinatario puede responder a la invitación."
            },
        )

    if amistad.estado != EstadoAmistad.PENDIENTE:
        return JSONResponse(
            status_code=409,
            content={"mensaje": "La invitación ya fue respondida o cancelada."},
        )

    if cuerpo.aceptar is None:
        return respuesta_errores(
            400, [reglas.ErrorCampo("aceptar", "El campo aceptar es obligatorio.")]
        )

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    actualizada = repo_amistades.responder(
        id_amistad=uuid_amistad, aceptar=cuerpo.aceptar, ahora=ahora
    )
    return JSONResponse(
        status_code=200,
        content={
            "id": str(actualizada.id),
            "estado": actualizada.estado.value,
            "fecha_respuesta": (
                actualizada.fecha_respuesta.strftime(FORMATO_FECHA)
                if actualizada.fecha_respuesta
                else None
            ),
        },
    )


@app.delete(
    "/amistades/{id_amistad}",
    status_code=204,
    response_class=Response,
    tags=["Amistades"],
    summary="Eliminar amistad",
    responses={
        401: {"model": ErrorMensaje},
        404: {"model": ErrorMensaje},
    },
)
def eliminar_amistad(
    id_amistad: str,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_amistades: RepositorioAmistadesDep,
) -> Response:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_amistad = UUID(id_amistad)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": "Amistad no encontrada."}
        )

    amistad = repo_amistades.buscar_por_id(uuid_amistad)
    if amistad is None or id_usuario not in (amistad.invitador_id, amistad.invitado_id):
        return JSONResponse(
            status_code=404, content={"mensaje": "Amistad no encontrada."}
        )

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    try:
        repo_amistades.eliminar(id_amistad=uuid_amistad, ahora=ahora)
    except AmistadYaRespondida:
        pass

    return Response(status_code=204)


@app.post(
    "/grupos",
    status_code=201,
    tags=["Grupos"],
    summary="Crear grupo",
)
def crear_grupo(
    cuerpo: NuevoGrupo,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    errores: list[reglas.ErrorCampo] = []
    nombre = cuerpo.nombre.strip() if cuerpo.nombre else ""
    if not nombre:
        errores.append(
            reglas.ErrorCampo("nombre", "El nombre del grupo es obligatorio.")
        )

    moneda = cuerpo.moneda.strip().upper() if cuerpo.moneda else ""
    if len(moneda) != 3 or not moneda.isalpha():
        errores.append(
            reglas.ErrorCampo(
                "moneda",
                "La moneda debe ser un código ISO de tres letras mayúsculas.",
            )
        )

    if errores:
        return respuesta_errores(400, errores)

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    grupo = repo_grupos.crear_grupo(
        creado_por=id_usuario,
        nombre=nombre,
        descripcion=cuerpo.descripcion,
        moneda=moneda,
        actividad=cuerpo.actividad,
        fecha_actividad=None,
        ahora=ahora,
    )
    return JSONResponse(
        status_code=201,
        content={
            "id": str(grupo.id),
            "nombre": grupo.nombre,
            "descripcion": grupo.descripcion or "",
            "moneda": grupo.moneda,
            "actividad": grupo.actividad or "",
            "rol": "administrador",
            "cantidad_miembros": 1,
        },
    )


@app.get(
    "/grupos",
    tags=["Grupos"],
    summary="Listar mis grupos",
)
def listar_mis_grupos(
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
    repo_usuarios: Repositorio,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    vistas = repo_grupos.listar_grupos_de(id_usuario)
    grupos_resp: list[dict[str, Any]] = []
    invitaciones_resp: list[dict[str, Any]] = []

    for vista in vistas:
        if vista.mi_estado == EstadoMiembro.ACEPTADO:
            miembros = repo_grupos.listar_miembros_de_grupo(vista.grupo.id)
            cant = sum(1 for m in miembros if m.estado == EstadoMiembro.ACEPTADO)
            grupos_resp.append(
                {
                    "id": str(vista.grupo.id),
                    "nombre": vista.grupo.nombre,
                    "descripcion": vista.grupo.descripcion or "",
                    "moneda": vista.grupo.moneda,
                    "actividad": vista.grupo.actividad or "",
                    "rol": vista.mi_rol.value,
                    "cantidad_miembros": cant,
                }
            )
        elif vista.mi_estado == EstadoMiembro.INVITADO:
            creador = repo_usuarios.buscar_por_id(vista.grupo.creado_por)
            invitador_datos = {
                "nombre_usuario": creador.nombre_usuario if creador else "",
                "nombre_visible": creador.nombre_visible if creador else "",
            }
            invitaciones_resp.append(
                {
                    "grupo_id": str(vista.grupo.id),
                    "nombre": vista.grupo.nombre,
                    "moneda": vista.grupo.moneda,
                    "actividad": vista.grupo.actividad or "",
                    "invitado_por": invitador_datos,
                }
            )

    return JSONResponse(
        status_code=200,
        content={"grupos": grupos_resp, "invitaciones": invitaciones_resp},
    )


@app.get(
    "/grupos/{id_grupo}",
    tags=["Grupos"],
    summary="Detalle de grupo",
)
def obtener_grupo(
    id_grupo: str,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_grupo = UUID(id_grupo)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    grupo = repo_grupos.buscar_grupo_por_id(uuid_grupo)
    if grupo is None:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    miembro = repo_grupos.buscar_miembro(id_grupo=uuid_grupo, usuario_id=id_usuario)
    if miembro is None or miembro.estado != EstadoMiembro.ACEPTADO:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    miembros = repo_grupos.listar_miembros_de_grupo(uuid_grupo)
    miembros_resp = [
        {
            "usuario": m.nombre_usuario,
            "nombre_visible": m.nombre_visible,
            "rol": m.rol.value,
            "estado": m.estado.value,
        }
        for m in miembros
    ]

    return JSONResponse(
        status_code=200,
        content={
            "id": str(grupo.id),
            "nombre": grupo.nombre,
            "descripcion": grupo.descripcion or "",
            "moneda": grupo.moneda,
            "actividad": grupo.actividad or "",
            "rol": miembro.rol.value,
            "miembros": miembros_resp,
            "gastos": [],
            "balances": {},
        },
    )


@app.post(
    "/grupos/{id_grupo}/miembros",
    status_code=201,
    tags=["Grupos"],
    summary="Invitar miembro a grupo",
)
def invitar_miembro(
    id_grupo: str,
    cuerpo: InvitarMiembro,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
    repo_usuarios: Repositorio,
    repo_amistades: RepositorioAmistadesDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_grupo = UUID(id_grupo)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    grupo = repo_grupos.buscar_grupo_por_id(uuid_grupo)
    if grupo is None:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    miembro_admin = repo_grupos.buscar_miembro(
        id_grupo=uuid_grupo, usuario_id=id_usuario
    )
    if miembro_admin is None or miembro_admin.estado != EstadoMiembro.ACEPTADO:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    if miembro_admin.rol != Rol.ADMINISTRADOR:
        return JSONResponse(status_code=403, content={"mensaje": MENSAJE_NO_ADMIN})

    if not cuerpo.usuario or not cuerpo.usuario.strip():
        return respuesta_errores(
            400,
            [reglas.ErrorCampo("usuario", "El nombre de usuario es obligatorio.")],
        )

    invitado = repo_usuarios.buscar_por_nombre_usuario(cuerpo.usuario.strip())
    if invitado is None:
        return JSONResponse(
            status_code=404, content={"mensaje": "Usuario no encontrado."}
        )

    if not repo_amistades.son_amigos(id_usuario, invitado.id):
        return JSONResponse(status_code=403, content={"mensaje": MENSAJE_NO_AMIGOS})

    existente = repo_grupos.buscar_miembro(id_grupo=uuid_grupo, usuario_id=invitado.id)
    if existente is not None:
        return respuesta_errores(
            409, [reglas.ErrorCampo("usuario", MENSAJE_YA_MIEMBRO)]
        )

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    repo_grupos.invitar_a_grupo(
        id_grupo=uuid_grupo,
        usuario_id=invitado.id,
        invitado_por=id_usuario,
        ahora=ahora,
    )
    return JSONResponse(
        status_code=201,
        content={
            "usuario": invitado.nombre_usuario,
            "nombre_visible": invitado.nombre_visible,
            "rol": "miembro",
            "estado": "invitado",
        },
    )


@app.post(
    "/grupos/{id_grupo}/miembros/yo/respuesta",
    tags=["Grupos"],
    summary="Responder invitación a grupo",
)
def responder_invitacion_grupo(
    id_grupo: str,
    cuerpo: ResponderInvitacionGrupo,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_grupo = UUID(id_grupo)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    grupo = repo_grupos.buscar_grupo_por_id(uuid_grupo)
    if grupo is None:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    mi_invitacion = repo_grupos.buscar_miembro(
        id_grupo=uuid_grupo, usuario_id=id_usuario
    )
    if mi_invitacion is None or mi_invitacion.estado != EstadoMiembro.INVITADO:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    resp_str = cuerpo.respuesta.strip().lower() if cuerpo.respuesta else ""
    if resp_str not in ("aceptar", "rechazar"):
        return respuesta_errores(
            400, [reglas.ErrorCampo("respuesta", MENSAJE_RESPUESTA_INVALIDA)]
        )

    ahora = datetime.now(timezone.utc).replace(microsecond=0)
    repo_grupos.responder_invitacion_grupo(
        id_grupo=uuid_grupo,
        usuario_id=id_usuario,
        aceptar=(resp_str == "aceptar"),
        ahora=ahora,
    )
    msg = "Invitación aceptada." if resp_str == "aceptar" else "Invitación rechazada."
    return JSONResponse(status_code=200, content={"mensaje": msg})


@app.post(
    "/grupos/{id_grupo}/gastos",
    tags=["Gastos"],
    summary="Registrar gasto de grupo",
)
def registrar_gasto_grupo(
    id_grupo: str,
    credenciales: Annotated[
        HTTPAuthorizationCredentials | None, Depends(esquema_bearer)
    ],
    repo_grupos: RepositorioGruposDep,
) -> JSONResponse:
    if credenciales is None or credenciales.scheme.lower() != "bearer":
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)
    id_usuario = id_desde_token(credenciales.credentials)
    if id_usuario is None:
        return no_autorizado(MENSAJE_SESION_INVALIDA, con_desafio=True)

    try:
        uuid_grupo = UUID(id_grupo)
    except ValueError:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    grupo = repo_grupos.buscar_grupo_por_id(uuid_grupo)
    if grupo is None:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    miembro = repo_grupos.buscar_miembro(id_grupo=uuid_grupo, usuario_id=id_usuario)
    if miembro is None or miembro.estado != EstadoMiembro.ACEPTADO:
        return JSONResponse(
            status_code=404, content={"mensaje": MENSAJE_GRUPO_NO_ENCONTRADO}
        )

    return JSONResponse(
        status_code=501, content={"mensaje": "Endpoint no implementado aún."}
    )


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
