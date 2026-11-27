"""La decisión de autorización de grupo, sin HTTP (INV-17, RNF-07)."""

import json
from datetime import datetime, timezone
from uuid import UUID, uuid4

import pytest
from repositorio_en_memoria import RepositorioEnMemoria, RepositorioGruposEnMemoria

from autorizacion import grupo_no_encontrado, pertenencia_aceptada
from repositorio_grupos import EstadoMiembro, Rol

AHORA = datetime(2026, 11, 12, tzinfo=timezone.utc)


@pytest.fixture
def almacen() -> RepositorioEnMemoria:
    return RepositorioEnMemoria()


@pytest.fixture
def grupos(almacen: RepositorioEnMemoria) -> RepositorioGruposEnMemoria:
    return RepositorioGruposEnMemoria(almacen)


def persona(almacen: RepositorioEnMemoria, nombre: str) -> UUID:
    return almacen.crear(
        nombre_usuario=nombre,
        nombre_visible=nombre.title(),
        correo=f"{nombre}@ejemplo.com",
        hash_contrasena="hash",
    ).id


def test_el_miembro_aceptado_obtiene_grupo_y_rol(
    almacen: RepositorioEnMemoria, grupos: RepositorioGruposEnMemoria
) -> None:
    ana = persona(almacen, "ana")
    grupo = grupos.crear(
        creado_por=ana,
        nombre="Viaje",
        descripcion=None,
        moneda="USD",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    pertenencia = pertenencia_aceptada(grupos, str(grupo.id), ana)
    assert pertenencia is not None
    assert pertenencia.grupo == grupo
    assert pertenencia.miembro.rol == Rol.ADMINISTRADOR
    assert pertenencia.miembro.estado == EstadoMiembro.ACEPTADO


def test_los_cuatro_casos_negados_son_indistinguibles(
    almacen: RepositorioEnMemoria, grupos: RepositorioGruposEnMemoria
) -> None:
    ana, beto, carla = (persona(almacen, n) for n in ("ana", "beto", "carla"))
    grupo = grupos.crear(
        creado_por=ana,
        nombre="Viaje",
        descripcion=None,
        moneda="USD",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    grupos.invitar(id_grupo=grupo.id, usuario_id=beto, invitado_por=ana, ahora=AHORA)

    assert pertenencia_aceptada(grupos, "no-es-uuid", ana) is None
    assert pertenencia_aceptada(grupos, str(uuid4()), ana) is None
    assert pertenencia_aceptada(grupos, str(grupo.id), carla) is None  # ajena
    assert pertenencia_aceptada(grupos, str(grupo.id), beto) is None  # invitado

    grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=beto, ahora=AHORA)
    assert pertenencia_aceptada(grupos, str(grupo.id), beto) is not None


def test_el_404_uniforme_tiene_siempre_el_mismo_cuerpo() -> None:
    respuesta = grupo_no_encontrado()
    assert respuesta.status_code == 404
    assert json.loads(bytes(respuesta.body)) == {"mensaje": "Grupo no encontrado."}
    assert grupo_no_encontrado().body == respuesta.body
