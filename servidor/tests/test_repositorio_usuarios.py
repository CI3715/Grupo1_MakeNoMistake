import os

import pytest
from dotenv import load_dotenv

from base_datos import BaseDatos
from configuracion import RUTA_ENV
from repositorio_usuarios import (
    CorreoYaRegistrado,
    NombreUsuarioYaRegistrado,
    RepositorioUsuarios,
)

# En local, `servidor/.env` ya trae la base de pruebas. En un clon limpio no
# existe y las pruebas se omiten; una variable de entorno explícita tiene
# prioridad sobre el archivo.
load_dotenv(RUTA_ENV)
URL_PRUEBAS = os.environ.get("CUENTAS_CLARAS_DB_URL_PRUEBAS") or None

pytestmark = pytest.mark.skipif(
    URL_PRUEBAS is None,
    reason=(
        "Define CUENTAS_CLARAS_DB_URL_PRUEBAS para probar el repositorio "
        "contra PostgreSQL."
    ),
)


@pytest.fixture()
def repositorio() -> RepositorioUsuarios:
    assert URL_PRUEBAS is not None

    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()

    with base_datos.conexion() as conexion:
        # CASCADE limpia también las tablas compartidas que referencian a
        # `usuarios` (amistades, miembros, partes_gasto).
        conexion.execute("TRUNCATE TABLE usuarios CASCADE")

    return RepositorioUsuarios(base_datos)


def test_crear_guarda_el_correo_en_minusculas(
    repositorio: RepositorioUsuarios,
) -> None:
    usuario = repositorio.crear(
        nombre_usuario="ana",
        nombre_visible="Ana Pérez",
        correo="Ana@Example.com",
        hash_contrasena="hash",
    )

    assert usuario.correo == "ana@example.com"
    assert usuario.fecha_alta.tzinfo is not None


def test_buscar_por_correo_no_distingue_mayusculas(
    repositorio: RepositorioUsuarios,
) -> None:
    creado = repositorio.crear(
        nombre_usuario="ana",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        hash_contrasena="hash",
    )

    encontrado = repositorio.buscar_por_correo("ANA@EXAMPLE.COM")

    assert encontrado is not None
    assert encontrado.id == creado.id


def test_buscar_por_id(repositorio: RepositorioUsuarios) -> None:
    creado = repositorio.crear(
        nombre_usuario="ana",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        hash_contrasena="hash",
    )

    encontrado = repositorio.buscar_por_id(creado.id)

    assert encontrado == creado


def test_correo_duplicado_indica_el_campo(
    repositorio: RepositorioUsuarios,
) -> None:
    repositorio.crear(
        nombre_usuario="ana",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        hash_contrasena="hash",
    )

    with pytest.raises(CorreoYaRegistrado) as error:
        repositorio.crear(
            nombre_usuario="otra",
            nombre_visible="Otra Persona",
            correo="ANA@EXAMPLE.COM",
            hash_contrasena="hash",
        )

    assert error.value.campo == "correo"


def test_nombre_usuario_duplicado_indica_el_campo(
    repositorio: RepositorioUsuarios,
) -> None:
    repositorio.crear(
        nombre_usuario="ana",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        hash_contrasena="hash",
    )

    with pytest.raises(NombreUsuarioYaRegistrado) as error:
        repositorio.crear(
            nombre_usuario="ana",
            nombre_visible="Otra Ana",
            correo="otra@example.com",
            hash_contrasena="hash",
        )

    assert error.value.campo == "nombre_usuario"


def test_crear_esquema_es_idempotente() -> None:
    assert URL_PRUEBAS is not None

    base_datos = BaseDatos(URL_PRUEBAS)
    base_datos.crear_esquema()
    base_datos.crear_esquema()
