"""Pruebas del repositorio de amistades contra PostgreSQL real."""

from datetime import datetime, timedelta, timezone
from decimal import Decimal
from uuid import uuid4

import pytest
from apoyo_compartido import crear_usuario
from psycopg import errors

from base_datos import BaseDatos
from repositorio_amistades import (
    AmistadNoEncontrada,
    AmistadYaRespondida,
    EstadoAmistad,
    InvitacionPendiente,
    NoEsDestinatario,
    RepositorioAmistades,
    YaSonAmigos,
)
from repositorio_gastos import ParteCalculada, RepositorioGastos
from repositorio_grupos import RepositorioGrupos
from repositorio_usuarios import RepositorioUsuarios, Usuario

AHORA = datetime(2026, 11, 11, 12, 0, tzinfo=timezone.utc)


@pytest.fixture()
def usuarios(base_datos_pruebas: BaseDatos) -> RepositorioUsuarios:
    return RepositorioUsuarios(base_datos_pruebas)


@pytest.fixture()
def amistades(base_datos_pruebas: BaseDatos) -> RepositorioAmistades:
    return RepositorioAmistades(base_datos_pruebas)


@pytest.fixture()
def pareja(
    usuarios: RepositorioUsuarios,
) -> tuple[Usuario, Usuario]:
    return crear_usuario(usuarios, "ana"), crear_usuario(usuarios, "beto")


def test_crear_deja_la_invitacion_pendiente(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja

    amistad = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    assert amistad.estado is EstadoAmistad.PENDIENTE
    assert amistad.fecha_invitacion == AHORA
    assert amistad.fecha_respuesta is None
    assert amistad.fecha_baja is None
    assert amistades.buscar_activa_entre(ana.id, beto.id) == amistad
    # La pareja se consulta en los dos sentidos.
    assert amistades.buscar_activa_entre(beto.id, ana.id) == amistad


def test_la_pareja_es_unica_en_los_dos_sentidos(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    with pytest.raises(InvitacionPendiente):
        amistades.crear(invitador_id=beto.id, invitado_id=ana.id, ahora=AHORA)


def test_el_indice_unico_parcial_tambien_lo_impide_en_la_base(
    base_datos_pruebas: BaseDatos,
    amistades: RepositorioAmistades,
    pareja: tuple[Usuario, Usuario],
) -> None:
    ana, beto = pareja
    amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    with base_datos_pruebas.conexion() as conexion:
        with pytest.raises(errors.UniqueViolation):
            conexion.execute(
                "INSERT INTO amistades (id, invitador_id, invitado_id, estado, "
                "fecha_invitacion, fecha_respuesta, fecha_baja) "
                "VALUES (%s, %s, %s, 'pendiente', %s, NULL, NULL)",
                (uuid4(), beto.id, ana.id, AHORA),
            )
        conexion.rollback()


def test_aceptar_crea_la_amistad_y_la_lista_en_los_dos_lados(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    aceptada = amistades.responder(
        id_amistad=pendiente.id,
        aceptar=True,
        ahora=AHORA + timedelta(minutes=5),
        usuario_id=beto.id,
    )

    assert aceptada.estado is EstadoAmistad.ACEPTADA
    assert aceptada.fecha_respuesta == AHORA + timedelta(minutes=5)
    assert amistades.son_amigos(ana.id, beto.id)
    assert amistades.son_amigos(beto.id, ana.id)

    vistas_ana = amistades.listar_de(ana.id)
    vistas_beto = amistades.listar_de(beto.id)
    assert len(vistas_ana) == len(vistas_beto) == 1
    assert vistas_ana[0].enviada_por_mi is True
    assert vistas_ana[0].otro_nombre_usuario == "beto"
    assert vistas_beto[0].enviada_por_mi is False
    assert vistas_beto[0].otro_nombre_usuario == "ana"


def test_solo_el_destinatario_puede_responder(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    with pytest.raises(NoEsDestinatario):
        amistades.responder(
            id_amistad=pendiente.id,
            aceptar=True,
            ahora=AHORA,
            usuario_id=ana.id,
        )

    assert amistades.buscar_por_id(pendiente.id) == pendiente


def test_responder_dos_veces_falla(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)
    amistades.responder(
        id_amistad=pendiente.id, aceptar=True, ahora=AHORA, usuario_id=beto.id
    )

    with pytest.raises(AmistadYaRespondida):
        amistades.responder(
            id_amistad=pendiente.id,
            aceptar=False,
            ahora=AHORA,
            usuario_id=beto.id,
        )


def test_rechazar_permite_volver_a_invitar(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    rechazada = amistades.responder(
        id_amistad=pendiente.id, aceptar=False, ahora=AHORA, usuario_id=beto.id
    )

    assert rechazada.estado is EstadoAmistad.RECHAZADA
    assert not amistades.son_amigos(ana.id, beto.id)
    assert amistades.listar_de(ana.id) == []

    nueva = amistades.crear(invitador_id=beto.id, invitado_id=ana.id, ahora=AHORA)
    assert nueva.id != pendiente.id
    assert nueva.estado is EstadoAmistad.PENDIENTE


def test_ya_ser_amigos_impide_otra_invitacion(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)
    amistades.responder(
        id_amistad=pendiente.id, aceptar=True, ahora=AHORA, usuario_id=beto.id
    )

    with pytest.raises(YaSonAmigos):
        amistades.crear(invitador_id=beto.id, invitado_id=ana.id, ahora=AHORA)


def test_eliminar_marca_la_baja_y_no_toca_grupos_ni_gastos(
    base_datos_pruebas: BaseDatos,
    usuarios: RepositorioUsuarios,
    amistades: RepositorioAmistades,
    pareja: tuple[Usuario, Usuario],
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)
    amistades.responder(
        id_amistad=pendiente.id, aceptar=True, ahora=AHORA, usuario_id=beto.id
    )

    grupos = RepositorioGrupos(base_datos_pruebas)
    gastos = RepositorioGastos(base_datos_pruebas)
    grupo = grupos.crear(
        creado_por=ana.id,
        nombre="Piso compartido",
        descripcion=None,
        moneda="EUR",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    grupos.invitar(
        id_grupo=grupo.id, usuario_id=beto.id, invitado_por=ana.id, ahora=AHORA
    )
    grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=beto.id, ahora=AHORA)
    gastos.registrar(
        grupo_id=grupo.id,
        concepto="Cena",
        monto=Decimal("10.00"),
        fecha=AHORA.date(),
        pagador_id=ana.id,
        regla="iguales",
        partes=[
            ParteCalculada(usuario_id=ana.id, monto=Decimal("5.00")),
            ParteCalculada(usuario_id=beto.id, monto=Decimal("5.00")),
        ],
        registrado_por=ana.id,
        ahora=AHORA,
    )

    amistades.eliminar(id_amistad=pendiente.id, ahora=AHORA + timedelta(hours=1))

    eliminada = amistades.buscar_por_id(pendiente.id)
    assert eliminada is not None
    assert eliminada.estado is EstadoAmistad.ELIMINADA
    assert eliminada.fecha_baja == AHORA + timedelta(hours=1)
    assert not amistades.son_amigos(ana.id, beto.id)

    # RF-SOC-06: la amistad se cierra, pero el grupo, sus miembros y su gasto
    # siguen intactos.
    assert grupos.buscar_por_id(grupo.id) is not None
    miembro = grupos.buscar_miembro(id_grupo=grupo.id, usuario_id=beto.id)
    assert miembro is not None
    assert len(gastos.listar_del_grupo(grupo.id)) == 1


def test_eliminar_una_invitacion_pendiente_la_cierra(
    amistades: RepositorioAmistades, pareja: tuple[Usuario, Usuario]
) -> None:
    ana, beto = pareja
    pendiente = amistades.crear(invitador_id=ana.id, invitado_id=beto.id, ahora=AHORA)

    amistades.eliminar(id_amistad=pendiente.id, ahora=AHORA)

    cerrada = amistades.buscar_por_id(pendiente.id)
    assert cerrada is not None
    assert cerrada.estado is EstadoAmistad.ELIMINADA
    assert cerrada.fecha_respuesta == AHORA
    assert cerrada.fecha_baja == AHORA


def test_buscar_por_id_inexistente_devuelve_none(
    amistades: RepositorioAmistades,
) -> None:
    assert amistades.buscar_por_id(uuid4()) is None

    with pytest.raises(AmistadNoEncontrada):
        amistades.eliminar(id_amistad=uuid4(), ahora=AHORA)
