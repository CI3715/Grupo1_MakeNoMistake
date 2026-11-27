"""Pruebas unitarias de las reglas de registro (reglas.py), sin HTTP ni base."""

import pytest

import reglas


@pytest.mark.parametrize(
    "valor", ["ana", "ana.perez", "ana_perez", "a1b2c3", "a" * 30, "...", "___"]
)
def test_nombre_usuario_valido(valor: str) -> None:
    assert reglas.validar_nombre_usuario(valor) is None


@pytest.mark.parametrize("valor", ["an", "a" * 31])
def test_nombre_usuario_longitud(valor: str) -> None:
    error = reglas.validar_nombre_usuario(valor)
    assert error == reglas.ErrorCampo(
        "nombre_usuario", reglas.MENSAJE_NOMBRE_USUARIO_LONGITUD
    )


@pytest.mark.parametrize(
    "valor", ["AnaPerez", "ana-perez", "ana perez", "ana@perez", "añoño", "ana$"]
)
def test_nombre_usuario_caracteres(valor: str) -> None:
    error = reglas.validar_nombre_usuario(valor)
    assert error == reglas.ErrorCampo(
        "nombre_usuario", reglas.MENSAJE_NOMBRE_USUARIO_CARACTERES
    )


@pytest.mark.parametrize("valor", [None, ""])
def test_nombre_usuario_obligatorio(valor: str | None) -> None:
    error = reglas.validar_nombre_usuario(valor)
    assert error == reglas.ErrorCampo("nombre_usuario", reglas.MENSAJE_OBLIGATORIO)


def test_nombre_visible_se_recorta_antes_de_medir() -> None:
    assert reglas.validar_nombre_visible("  " + "A" * 50 + "  ") is None
    assert reglas.validar_nombre_visible("A" * 51) == reglas.ErrorCampo(
        "nombre_visible", reglas.MENSAJE_NOMBRE_VISIBLE_LARGO
    )


@pytest.mark.parametrize("valor", ["", "   ", "\t\n"])
def test_nombre_visible_vacio(valor: str) -> None:
    assert reglas.validar_nombre_visible(valor) == reglas.ErrorCampo(
        "nombre_visible", reglas.MENSAJE_NOMBRE_VISIBLE_VACIO
    )


@pytest.mark.parametrize(
    "valor", ["ana@example.com", "ANA@Example.COM", " ana@ex.co ", "a.b+c@d.e.f"]
)
def test_correo_valido(valor: str) -> None:
    assert reglas.validar_correo(valor) is None


@pytest.mark.parametrize(
    "valor",
    ["ana-example.com", "ana@", "@example.com", "ana@example", "a b@c.d", "a@@b.c"],
)
def test_correo_invalido(valor: str) -> None:
    assert reglas.validar_correo(valor) == reglas.ErrorCampo(
        "correo", reglas.MENSAJE_CORREO_INVALIDO
    )


def test_correo_demasiado_largo() -> None:
    correo = "a" * 250 + "@b.co"
    assert reglas.validar_correo(correo) is not None


def test_normalizar_correo() -> None:
    assert reglas.normalizar_correo("  Ana@Example.COM ") == "ana@example.com"


def test_contrasena_limite_de_10() -> None:
    assert reglas.validar_contrasena("a" * 10) is None
    assert reglas.validar_contrasena("a" * 9) == reglas.ErrorCampo(
        "contrasena", reglas.MENSAJE_CONTRASENA_CORTA
    )


def test_contrasena_espacios_cuentan() -> None:
    assert reglas.validar_contrasena(" " * 10) is None


def test_validar_registro_devuelve_todos_los_errores() -> None:
    datos, errores = reglas.validar_registro("An", "", "malo", "corta")
    assert datos is None
    assert [e.campo for e in errores] == [
        "nombre_usuario",
        "nombre_visible",
        "correo",
        "contrasena",
    ]


def test_validar_registro_normaliza() -> None:
    datos, errores = reglas.validar_registro(
        "ana.perez", "  Ana Pérez ", " ANA@Example.com", "clave-segura-1234"
    )
    assert errores == []
    assert datos == reglas.DatosRegistro(
        nombre_usuario="ana.perez",
        nombre_visible="Ana Pérez",
        correo="ana@example.com",
        contrasena="clave-segura-1234",
    )


def test_mensajes_no_repiten_el_valor() -> None:
    secreto = "valor-secreto"
    _, errores = reglas.validar_registro(secreto + "$", "", secreto, "corto")
    for error in errores:
        assert secreto not in error.mensaje


def test_validar_credenciales() -> None:
    assert reglas.validar_credenciales("a@b.c", "x") == []
    campos = [e.campo for e in reglas.validar_credenciales(None, "")]
    assert campos == ["correo", "contrasena"]
