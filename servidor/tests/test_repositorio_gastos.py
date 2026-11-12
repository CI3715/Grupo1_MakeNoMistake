"""Pruebas del repositorio de gastos contra PostgreSQL real.

Se comprueba lo que pide la Entrega 6: montos exactos en ``NUMERIC``, gasto y
partes en una sola transacción, errores que dicen qué campo o regla falla y
que una amistad eliminada no borra los gastos ya compartidos.
"""

from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
from decimal import Decimal
from uuid import UUID, uuid4

import pytest
from apoyo_compartido import crear_usuario

import repositorio_gastos as modulo_gastos
from base_datos import BaseDatos
from repositorio_gastos import (
    GastoInvalido,
    ParteCalculada,
    ParteGasto,
    RepositorioGastos,
)
from repositorio_grupos import RepositorioGrupos
from repositorio_usuarios import RepositorioUsuarios, Usuario

AHORA = datetime(2026, 11, 11, 12, 0, tzinfo=timezone.utc)


@dataclass
class Escenario:
    base_datos: BaseDatos
    usuarios: RepositorioUsuarios
    grupos: RepositorioGrupos
    gastos: RepositorioGastos
    ana: Usuario
    beto: Usuario
    carla: Usuario
    grupo_id: UUID


@pytest.fixture()
def escenario(base_datos_pruebas: BaseDatos) -> Escenario:
    usuarios = RepositorioUsuarios(base_datos_pruebas)
    grupos = RepositorioGrupos(base_datos_pruebas)
    gastos = RepositorioGastos(base_datos_pruebas)

    ana = crear_usuario(usuarios, "ana")
    beto = crear_usuario(usuarios, "beto")
    carla = crear_usuario(usuarios, "carla")

    grupo = grupos.crear(
        creado_por=ana.id,
        nombre="Viaje",
        descripcion=None,
        moneda="EUR",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    for persona in (beto, carla):
        grupos.invitar(
            id_grupo=grupo.id,
            usuario_id=persona.id,
            invitado_por=ana.id,
            ahora=AHORA,
        )
        grupos.aceptar_invitacion(id_grupo=grupo.id, usuario_id=persona.id, ahora=AHORA)

    return Escenario(
        base_datos=base_datos_pruebas,
        usuarios=usuarios,
        grupos=grupos,
        gastos=gastos,
        ana=ana,
        beto=beto,
        carla=carla,
        grupo_id=grupo.id,
    )


def _partes_iguales(escenario: Escenario) -> list[ParteCalculada]:
    return [
        ParteCalculada(
            usuario_id=escenario.ana.id,
            monto=Decimal("33.34"),
            lleva_residuo=True,
        ),
        ParteCalculada(
            usuario_id=escenario.beto.id,
            monto=Decimal("33.33"),
        ),
        ParteCalculada(
            usuario_id=escenario.carla.id,
            monto=Decimal("33.33"),
        ),
    ]


def test_registrar_guarda_gasto_y_partes_exactas(
    escenario: Escenario,
) -> None:
    vista = escenario.gastos.registrar(
        grupo_id=escenario.grupo_id,
        concepto="Cena",
        monto=Decimal("100.00"),
        fecha=date(2026, 11, 11),
        pagador_id=escenario.ana.id,
        regla="iguales",
        partes=_partes_iguales(escenario),
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )

    assert vista.gasto.monto == Decimal("100.00")
    assert vista.pagador_nombre_usuario == "ana"
    assert vista.gasto.fecha_registro == AHORA
    assert [parte.monto for parte in vista.partes] == [
        Decimal("33.34"),
        Decimal("33.33"),
        Decimal("33.33"),
    ]
    assert [parte.lleva_residuo for parte in vista.partes] == [True, False, False]

    listado = escenario.gastos.listar_del_grupo(escenario.grupo_id)
    assert len(listado) == 1
    assert listado[0] == vista

    with escenario.base_datos.conexion() as conexion:
        gasto = conexion.execute(
            "SELECT monto FROM gastos WHERE id = %s", (vista.gasto.id,)
        ).fetchone()
        partes = conexion.execute(
            "SELECT usuario_id, monto, lleva_residuo FROM partes_gasto "
            "WHERE gasto_id = %s",
            (vista.gasto.id,),
        ).fetchall()
        suma = conexion.execute(
            "SELECT sum(monto) AS total FROM partes_gasto WHERE gasto_id = %s",
            (vista.gasto.id,),
        ).fetchone()

    assert gasto is not None and gasto["monto"] == Decimal("100.00")
    assert len(partes) == 3
    assert all(isinstance(fila["monto"], Decimal) for fila in partes)
    assert suma is not None and suma["total"] == Decimal("100.00")


def test_las_columnas_de_dinero_son_numeric_y_no_float(
    base_datos_pruebas: BaseDatos,
) -> None:
    with base_datos_pruebas.conexion() as conexion:
        filas = conexion.execute(
            "SELECT table_name, data_type, numeric_precision, numeric_scale "
            "FROM information_schema.columns "
            "WHERE table_schema = current_schema() "
            "AND column_name = 'monto' "
            "AND table_name IN ('gastos', 'partes_gasto') "
            "ORDER BY table_name"
        ).fetchall()

    assert [fila["table_name"] for fila in filas] == ["gastos", "partes_gasto"]
    for fila in filas:
        assert fila["data_type"] == "numeric"
        assert fila["numeric_precision"] == 14
        assert fila["numeric_scale"] == 2


def test_partes_que_no_suman_no_guardan_nada(escenario: Escenario) -> None:
    partes = [
        ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("33.33")),
        ParteCalculada(usuario_id=escenario.beto.id, monto=Decimal("33.33")),
        ParteCalculada(usuario_id=escenario.carla.id, monto=Decimal("33.33")),
    ]

    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("100.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=partes,
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "partes"
    assert error.value.regla == "suma"
    assert "99.99" in str(error.value)
    assert escenario.gastos.listar_del_grupo(escenario.grupo_id) == []


def test_regla_desconocida_dice_la_regla(escenario: Escenario) -> None:
    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="mitades",
            partes=[
                ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("10.00"))
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "regla"
    assert error.value.regla == "regla"
    assert "mitades" in str(error.value)


def test_monto_con_mas_de_dos_decimales_se_rechaza(
    escenario: Escenario,
) -> None:
    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("10.001"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=[
                ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("10.00"))
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "monto"
    assert "dos decimales" in str(error.value)


def test_float_se_rechaza_de_forma_explicita(escenario: Escenario) -> None:
    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=10.0,  # type: ignore[arg-type]
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=[
                ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("10.00"))
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "monto"
    assert "coma flotante" in str(error.value)


def test_participante_ajeno_al_grupo_se_rechaza(escenario: Escenario) -> None:
    david = crear_usuario(escenario.usuarios, "david")

    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=[
                ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("5.00")),
                ParteCalculada(usuario_id=david.id, monto=Decimal("5.00")),
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "participantes"
    assert error.value.regla == "miembro"
    assert str(david.id) in str(error.value)
    assert escenario.gastos.listar_del_grupo(escenario.grupo_id) == []


def test_pagador_y_registrador_deben_estar_aceptados(
    escenario: Escenario,
) -> None:
    grupo = escenario.grupos.crear(
        creado_por=escenario.ana.id,
        nombre="Pendiente",
        descripcion=None,
        moneda="EUR",
        actividad=None,
        fecha_actividad=None,
        ahora=AHORA,
    )
    escenario.grupos.invitar(
        id_grupo=grupo.id,
        usuario_id=escenario.beto.id,
        invitado_por=escenario.ana.id,
        ahora=AHORA,
    )
    partes = [
        ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("5.00")),
        ParteCalculada(usuario_id=escenario.beto.id, monto=Decimal("5.00")),
    ]

    with pytest.raises(GastoInvalido) as error_pagador:
        escenario.gastos.registrar(
            grupo_id=grupo.id,
            concepto="Cena",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.beto.id,
            regla="iguales",
            partes=partes,
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )
    assert error_pagador.value.campo == "pagador"

    with pytest.raises(GastoInvalido) as error_registrador:
        escenario.gastos.registrar(
            grupo_id=grupo.id,
            concepto="Cena",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=partes,
            registrado_por=escenario.beto.id,
            ahora=AHORA,
        )
    assert error_registrador.value.campo == "registrado_por"


def test_porcentajes_exactos_se_guardan(escenario: Escenario) -> None:
    vista = escenario.gastos.registrar(
        grupo_id=escenario.grupo_id,
        concepto="Hotel",
        monto=Decimal("10.00"),
        fecha=date(2026, 11, 11),
        pagador_id=escenario.ana.id,
        regla="porcentajes",
        partes=[
            ParteCalculada(
                usuario_id=escenario.ana.id,
                monto=Decimal("3.34"),
                lleva_residuo=True,
                porcentaje=3334,
            ),
            ParteCalculada(
                usuario_id=escenario.beto.id,
                monto=Decimal("3.33"),
                porcentaje=3333,
            ),
            ParteCalculada(
                usuario_id=escenario.carla.id,
                monto=Decimal("3.33"),
                porcentaje=3333,
            ),
        ],
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )

    assert [parte.porcentaje for parte in vista.partes] == [3334, 3333, 3333]


def test_porcentajes_que_no_suman_cien_se_rechazan(
    escenario: Escenario,
) -> None:
    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Hotel",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="porcentajes",
            partes=[
                ParteCalculada(
                    usuario_id=escenario.ana.id,
                    monto=Decimal("5.00"),
                    porcentaje=5000,
                ),
                ParteCalculada(
                    usuario_id=escenario.beto.id,
                    monto=Decimal("5.00"),
                    porcentaje=4000,
                ),
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "porcentajes"
    assert error.value.regla == "porcentajes"


def test_porcentaje_solo_vale_con_su_regla(escenario: Escenario) -> None:
    with pytest.raises(GastoInvalido) as error:
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("10.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=[
                ParteCalculada(
                    usuario_id=escenario.ana.id,
                    monto=Decimal("10.00"),
                    porcentaje=10000,
                )
            ],
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    assert error.value.campo == "porcentajes"
    assert error.value.regla == "iguales"


def test_montos_fijos_y_alias_de_la_interfaz(escenario: Escenario) -> None:
    fijos = escenario.gastos.registrar(
        grupo_id=escenario.grupo_id,
        concepto="Alquiler",
        monto=Decimal("120.00"),
        fecha=date(2026, 11, 10),
        pagador_id=escenario.ana.id,
        regla="montos_fijos",
        partes=[
            ParteCalculada(usuario_id=escenario.ana.id, monto=Decimal("50.00")),
            ParteCalculada(usuario_id=escenario.beto.id, monto=Decimal("40.00")),
            ParteCalculada(usuario_id=escenario.carla.id, monto=Decimal("30.00")),
        ],
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )
    assert fijos.gasto.regla == "montos"

    iguales = escenario.gastos.registrar(
        grupo_id=escenario.grupo_id,
        concepto="Taxi",
        monto=Decimal("100.00"),
        fecha=date(2026, 11, 11),
        pagador_id=escenario.ana.id,
        regla="partes_iguales",
        partes=_partes_iguales(escenario),
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )
    assert iguales.gasto.regla == "iguales"


def test_si_falla_una_parte_no_queda_gasto(
    escenario: Escenario, monkeypatch: pytest.MonkeyPatch
) -> None:
    def explotar(*_: object) -> None:
        raise RuntimeError("fallo simulado al insertar las partes")

    monkeypatch.setattr(modulo_gastos, "_insertar_partes", explotar)

    with pytest.raises(RuntimeError):
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto="Cena",
            monto=Decimal("100.00"),
            fecha=date(2026, 11, 11),
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=_partes_iguales(escenario),
            registrado_por=escenario.ana.id,
            ahora=AHORA,
        )

    # La transacción revirtió el INSERT del gasto.
    assert escenario.gastos.listar_del_grupo(escenario.grupo_id) == []


def test_listar_del_grupo_ordena_del_mas_reciente_al_mas_antiguo(
    escenario: Escenario,
) -> None:
    for concepto, fecha in (
        ("Antiguo", date(2026, 11, 9)),
        ("Nuevo", date(2026, 11, 11)),
    ):
        escenario.gastos.registrar(
            grupo_id=escenario.grupo_id,
            concepto=concepto,
            monto=Decimal("100.00"),
            fecha=fecha,
            pagador_id=escenario.ana.id,
            regla="iguales",
            partes=_partes_iguales(escenario),
            registrado_por=escenario.ana.id,
            ahora=AHORA + timedelta(hours=len(concepto)),
        )

    listado = escenario.gastos.listar_del_grupo(escenario.grupo_id)

    assert [vista.gasto.concepto for vista in listado] == ["Nuevo", "Antiguo"]


def test_parte_gasto_con_gasto_id_se_acepta_y_se_reescribe(
    escenario: Escenario,
) -> None:
    vista = escenario.gastos.registrar(
        grupo_id=escenario.grupo_id,
        concepto="Cena",
        monto=Decimal("100.00"),
        fecha=date(2026, 11, 11),
        pagador_id=escenario.ana.id,
        regla="iguales",
        partes=[
            ParteGasto(
                gasto_id=uuid4(),
                usuario_id=parte.usuario_id,
                monto=parte.monto,
                lleva_residuo=parte.lleva_residuo,
                porcentaje=parte.porcentaje,
            )
            for parte in _partes_iguales(escenario)
        ],
        registrado_por=escenario.ana.id,
        ahora=AHORA,
    )

    assert all(parte.gasto_id == vista.gasto.id for parte in vista.partes)
