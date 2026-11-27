"""Pruebas del repositorio de grupos y miembros contra PostgreSQL real."""

from datetime import date, datetime, timezone
from uuid import uuid4

import pytest
from apoyo_compartido import crear_usuario

from base_datos import BaseDatos
from repositorio_grupos import (
    EstadoMiembro,
    Grupo,
    GrupoInvalido,
    GrupoNoEncontrado,
    InvitacionDeGrupoNoPendiente,
    RepositorioGrupos,
    Rol,
    UsuarioNoEncontrado,
    YaEsMiembro,
)
from repositorio_usuarios import RepositorioUsuarios, Usuario

AHORA = datetime(2026, 11, 11, 12, 0, tzinfo=timezone.utc)


@pytest.fixture()
def usuarios(base_datos_pruebas: BaseDatos) -> RepositorioUsuarios:
    return RepositorioUsuarios(base_datos_pruebas)


@pytest.fixture()
def grupos(base_datos_pruebas: BaseDatos) -> RepositorioGrupos:
    return RepositorioGrupos(base_datos_pruebas)


@pytest.fixture()
def personas(usuarios: RepositorioUsuarios) -> tuple[Usuario, Usuario, Usuario]:
    return (
        crear_usuario(usuarios, "ana"),
        crear_usuario(usuarios, "beto"),
        crear_usuario(usuarios, "carla"),
    )


def _grupo(repositorio: RepositorioGrupos, creado_por: Usuario) -> Grupo:
    return repositorio.crear(
        creado_por=creado_por.id,
        nombre="Piso compartido",
        descripcion="Gastos comunes",
        moneda="EUR",
        actividad="Alquiler",
        fecha_actividad=date(2026, 11, 1),
        ahora=AHORA,
    )


def test_crear_agrega_al_creador_como_administrador(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, _, _ = personas

    grupo = _grupo(grupos, ana)

    assert grupos.buscar_por_id(grupo.id) == grupo
    assert grupo.nombre == "Piso compartido"
    assert grupo.moneda == "EUR"
    assert grupo.creado_por == ana.id

    miembro = grupos.buscar_miembro(id_grupo=grupo.id, usuario_id=ana.id)
    assert miembro is not None
    assert miembro.rol is Rol.ADMINISTRADOR
    assert miembro.estado is EstadoMiembro.ACEPTADO
    assert miembro.invitado_por is None
    assert miembro.fecha_respuesta == AHORA

    vistas = grupos.listar_de(ana.id)
    assert len(vistas) == 1
    assert vistas[0].grupo == grupo
    assert vistas[0].mi_rol is Rol.ADMINISTRADOR
    assert vistas[0].mi_estado is EstadoMiembro.ACEPTADO
    assert vistas[0].nombre_creador == "ana"


def test_invitar_aceptar_y_listar_miembros(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, beto, _ = personas
    grupo = _grupo(grupos, ana)

    invitado = grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )

    assert invitado.rol is Rol.MIEMBRO
    assert invitado.estado is EstadoMiembro.INVITADO
    assert invitado.invitado_por == ana.id
    assert invitado.fecha_respuesta is None

    pendientes = grupos.listar_de(beto.id)
    assert len(pendientes) == 1
    assert pendientes[0].mi_estado is EstadoMiembro.INVITADO

    aceptado = grupos.aceptar_invitacion(
        id_grupo=grupo.id,
        usuario_id=beto.id,
        ahora=AHORA,
    )
    assert aceptado.estado is EstadoMiembro.ACEPTADO
    assert aceptado.fecha_respuesta == AHORA

    miembros = grupos.listar_miembros(grupo.id)
    assert [m.nombre_usuario for m in miembros] == ["ana", "beto"]
    assert all(not hasattr(m, "correo") for m in miembros)


def test_no_se_puede_invitar_dos_veces(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, beto, _ = personas
    grupo = _grupo(grupos, ana)
    grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )

    with pytest.raises(YaEsMiembro):
        grupos.invitar(
            id_grupo=grupo.id,
            usuario_id=beto.id,
            invitado_por=ana.id,
            ahora=AHORA,
        )


def test_rechazar_borra_la_invitacion_y_permite_reinvitar(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, beto, _ = personas
    grupo = _grupo(grupos, ana)
    grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )

    grupos.rechazar_invitacion(id_grupo=grupo.id, usuario_id=beto.id)

    assert grupos.buscar_miembro(id_grupo=grupo.id, usuario_id=beto.id) is None
    assert grupos.listar_de(beto.id) == []

    nueva = grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )
    assert nueva.estado is EstadoMiembro.INVITADO


def test_no_se_puede_rechazar_ni_aceptar_sin_invitacion(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, beto, _ = personas
    grupo = _grupo(grupos, ana)
    grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )
    grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=beto.id, ahora=AHORA)

    with pytest.raises(InvitacionDeGrupoNoPendiente):
        grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=beto.id, ahora=AHORA)
    # Rechazar no borra a un miembro ya aceptado.
    with pytest.raises(InvitacionDeGrupoNoPendiente):
        grupos.rechazar_invitacion(id_grupo=grupo.id, usuario_id=beto.id)
    assert grupos.buscar_miembro(id_grupo=grupo.id, usuario_id=beto.id) is not None


def test_invitar_valida_grupo_y_persona(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, beto, _ = personas
    grupo = _grupo(grupos, ana)

    with pytest.raises(GrupoNoEncontrado):
        grupos.invitar(
            id_grupo=uuid4(),
            usuario_id=beto.id,
            invitado_por=ana.id,
            ahora=AHORA,
        )

    with pytest.raises(UsuarioNoEncontrado):
        grupos.invitar(
            id_grupo=grupo.id,
            usuario_id=uuid4(),
            invitado_por=ana.id,
            ahora=AHORA,
        )


def test_crear_valida_nombre_y_moneda(
    grupos: RepositorioGrupos, personas: tuple[Usuario, Usuario, Usuario]
) -> None:
    ana, _, _ = personas

    with pytest.raises(GrupoInvalido) as error_nombre:
        grupos.crear(
            creado_por=ana.id,
            nombre="   ",
            descripcion=None,
            moneda="EUR",
            actividad=None,
            fecha_actividad=None,
            ahora=AHORA,
        )
    assert error_nombre.value.campo == "nombre"

    with pytest.raises(GrupoInvalido) as error_moneda:
        grupos.crear(
            creado_por=ana.id,
            nombre="Viaje",
            descripcion=None,
            moneda="EURO",
            actividad=None,
            fecha_actividad=None,
            ahora=AHORA,
        )
    assert error_moneda.value.campo == "moneda"
