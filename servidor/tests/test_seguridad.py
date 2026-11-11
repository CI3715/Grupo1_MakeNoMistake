"""Pruebas unitarias del hash de contraseñas, del token firmado y de INV-16."""

import os
import re
from datetime import datetime, timedelta, timezone
from pathlib import Path

import jwt
import pytest
from fastapi.routing import APIRoute
from fastapi.testclient import TestClient

import contrasenas
import token_sesion
from base_datos import SENTENCIAS_ESQUEMA
from main import app

# ---------------------------------------------------------------- Hash


def test_hash_es_argon2id_y_verifica() -> None:
    hash_ = contrasenas.hashear("una-clave-larga-123")
    assert hash_.startswith("$argon2id$")
    assert "una-clave-larga-123" not in hash_
    assert contrasenas.verificar(hash_, "una-clave-larga-123")
    assert not contrasenas.verificar(hash_, "otra-clave-larga-123")


def test_hash_tiene_sal_unica() -> None:
    assert contrasenas.hashear("misma-clave-123") != contrasenas.hashear(
        "misma-clave-123"
    )


def test_verificar_hash_malformado_devuelve_false() -> None:
    assert not contrasenas.verificar("no-es-un-hash", "lo-que-sea-123")


def test_hash_falso_no_coincide_con_nada_comun() -> None:
    assert contrasenas.HASH_FALSO.startswith("$argon2id$")
    assert not contrasenas.verificar(contrasenas.HASH_FALSO, "")
    assert not contrasenas.verificar(contrasenas.HASH_FALSO, "clave-segura-1234")


# ---------------------------------------------------------------- Token


def test_token_ida_y_vuelta() -> None:
    emitido = token_sesion.crear_token("usuario-1")
    assert token_sesion.verificar_token(emitido.token) == "usuario-1"


def test_token_vence_segun_jwt_minutos(monkeypatch: pytest.MonkeyPatch) -> None:
    ahora = datetime(2026, 11, 9, 12, 0, 0, 123456, tzinfo=timezone.utc)
    monkeypatch.setenv("JWT_MINUTOS", "1")
    emitido = token_sesion.crear_token("u", ahora=ahora)
    assert emitido.vence_en == datetime(2026, 11, 9, 12, 1, 0, tzinfo=timezone.utc)


def test_token_60_minutos_por_defecto(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("JWT_MINUTOS")
    ahora = datetime(2026, 11, 9, 12, 0, 0, tzinfo=timezone.utc)
    emitido = token_sesion.crear_token("u", ahora=ahora)
    assert emitido.vence_en - ahora == timedelta(minutes=60)


def test_token_alterado_se_rechaza() -> None:
    token = token_sesion.crear_token("usuario-1").token
    cabecera, carga, firma = token.split(".")
    otra_carga = jwt.utils.base64url_encode(b'{"sub":"usuario-2","exp":9999999999}')
    alterado = f"{cabecera}.{otra_carga.decode()}.{firma}"
    assert token_sesion.verificar_token(alterado) is None


def test_token_vencido_se_rechaza() -> None:
    hace_dos_horas = datetime.now(timezone.utc) - timedelta(hours=2)
    token = token_sesion.crear_token("u", ahora=hace_dos_horas).token
    assert token_sesion.verificar_token(token) is None


def test_token_con_otro_secreto_se_rechaza(monkeypatch: pytest.MonkeyPatch) -> None:
    token = token_sesion.crear_token("u").token
    monkeypatch.setenv("JWT_SECRET", "otro-secreto-igual-de-largo-que-el-de-prueba")
    assert token_sesion.verificar_token(token) is None


def test_token_algoritmo_none_se_rechaza() -> None:
    vence = datetime.now(timezone.utc) + timedelta(minutes=5)
    token = jwt.encode({"sub": "u", "exp": vence}, key=None, algorithm="none")
    assert token_sesion.verificar_token(token) is None


def test_token_sin_sub_se_rechaza() -> None:
    vence = datetime.now(timezone.utc) + timedelta(minutes=5)
    token = jwt.encode({"exp": vence}, os.environ["JWT_SECRET"], algorithm="HS256")
    assert token_sesion.verificar_token(token) is None


@pytest.mark.parametrize("secreto", [None, "", "corto"])
def test_secreto_obligatorio_y_sin_valor_por_defecto(
    monkeypatch: pytest.MonkeyPatch, secreto: str | None
) -> None:
    if secreto is None:
        monkeypatch.delenv("JWT_SECRET")
    else:
        monkeypatch.setenv("JWT_SECRET", secreto)
    with pytest.raises(token_sesion.ConfiguracionInvalida):
        token_sesion.crear_token("u")


@pytest.mark.parametrize("minutos", ["0", "-5", "uno"])
def test_minutos_invalidos(monkeypatch: pytest.MonkeyPatch, minutos: str) -> None:
    monkeypatch.setenv("JWT_MINUTOS", minutos)
    with pytest.raises(token_sesion.ConfiguracionInvalida):
        token_sesion.comprobar_configuracion()


def test_servidor_no_arranca_sin_secreto(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("JWT_SECRET")
    with pytest.raises(token_sesion.ConfiguracionInvalida), TestClient(app):
        pass


def test_servidor_no_arranca_sin_base_de_datos(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """Sin CUENTAS_CLARAS_DB_URL no hay a dónde guardar usuarios: no arranca."""
    monkeypatch.setattr("configuracion.RUTA_ENV", tmp_path / "no-existe.env")
    monkeypatch.setenv("CUENTAS_CLARAS_SECRETO_JWT", "s" * 40)
    monkeypatch.delattr(app.state, "repositorio_usuarios")
    with pytest.raises(RuntimeError, match="CUENTAS_CLARAS_DB_URL"), TestClient(app):
        pass


# ---------------------------------------------------------------- INV-16


def test_servidor_respeta_inv16_sin_tablas_financieras() -> None:
    """Verifica que el esquema del servidor no cree tablas financieras (INV-16).

    ``test_integracion_postgres.py`` comprueba lo mismo contra PostgreSQL real.
    """
    tablas = [
        tabla
        for sentencia in SENTENCIAS_ESQUEMA
        for tabla in re.findall(
            r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)", sentencia, re.I
        )
    ]

    assert tablas == ["usuarios"]
    prohibidas = {
        "cuentas",
        "movimientos",
        "saldos",
        "tasas",
        "categorias",
        "patrimonio",
    }
    assert set(tablas).isdisjoint(prohibidas)


def test_servidor_respeta_inv16_en_rutas_expuestas() -> None:
    """Verifica que las rutas de FastAPI no expongan entidades financieras (INV-16)."""
    rutas = [ruta.path for ruta in app.routes if isinstance(ruta, APIRoute)]
    terminos_prohibidos = [
        "cuenta",
        "movimiento",
        "saldo",
        "tasa",
        "categoria",
        "patrimonio",
    ]
    for ruta in rutas:
        for termino in terminos_prohibidos:
            assert termino not in ruta.lower(), (
                f"Ruta {ruta} viola INV-16 con término {termino}"
            )
