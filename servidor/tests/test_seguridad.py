"""Pruebas unitarias de seguridad, criptografía y aislamiento INV-16.

Verifica RNF-05 (argon2id con sal única, contraseñas no recuperables),
RF-SOC-02/RES-07 (tokens JWT seguros) e INV-16 (aislamiento de datos financieros).
"""

import sqlite3
from pathlib import Path

import pytest
from fastapi.routing import APIRoute

from base_datos import (
    buscar_usuario_por_correo,
    buscar_usuario_por_id,
    buscar_usuario_por_nombre,
    conectar,
    inicializar_base_datos,
    insertar_usuario,
)
from main import app
from seguridad import (
    TokenInvalidoError,
    TokenVencidoError,
    crear_token_sesion,
    decodificar_token_sesion,
    hashear_contrasena,
    verificar_contrasena,
)


def test_hash_usa_argon2id_con_sal_unica() -> None:
    """Verifica que el hash usa argon2id y sal única por ejecución."""
    clave = "clave-segura-1234"
    hash1 = hashear_contrasena(clave)
    hash2 = hashear_contrasena(clave)

    assert hash1.startswith("$argon2id$")
    assert hash2.startswith("$argon2id$")
    # Sal única: dos llamadas con la misma clave generan hashes distintos
    assert hash1 != hash2


def test_contrasena_guardada_no_es_recuperable() -> None:
    """Comprueba que el hash es unidireccional y no contiene la clave (RNF-05)."""
    clave = "mi-clave-super-secreta-4321"
    hash_generado = hashear_contrasena(clave)

    assert clave not in hash_generado
    assert verificar_contrasena(clave, hash_generado) is True
    assert verificar_contrasena("otra-clave-diferente", hash_generado) is False
    assert verificar_contrasena(clave.upper(), hash_generado) is False


def test_creacion_y_validacion_token_jwt() -> None:
    """Crea un token JWT válido y comprueba la extracción del usuario_id."""
    usuario_id = "usr-uuid-5678"
    token, vence_en = crear_token_sesion(usuario_id, minutos=30)

    assert isinstance(token, str) and len(token) > 0
    assert isinstance(vence_en, str) and "T" in vence_en and vence_en.endswith("Z")

    usuario_recuperado = decodificar_token_sesion(token)
    assert usuario_recuperado == usuario_id


def test_token_vencido_lanza_excepcion() -> None:
    """Un token con fecha de expiración en el pasado produce TokenVencidoError."""
    token_vencido, _ = crear_token_sesion("usr-expirado", minutos=-10)
    with pytest.raises(TokenVencidoError):
        decodificar_token_sesion(token_vencido)


def test_token_alterado_o_invalido_lanza_excepcion() -> None:
    """Un token manipulado produce TokenInvalidoError."""
    token_valido, _ = crear_token_sesion("usr-valido", minutos=15)
    token_corrupto = token_valido[:-4] + "ZZZZ"
    with pytest.raises(TokenInvalidoError):
        decodificar_token_sesion(token_corrupto)


def test_token_firmado_con_otro_secreto_lanza_excepcion() -> None:
    """Un token firmado con una clave secreta ajena produce TokenInvalidoError."""
    token_ajeno, _ = crear_token_sesion(
        "usr-ajeno",
        secreto="otro-secreto-totalmente-distinto-y-muy-largo-para-pruebas",
    )
    with pytest.raises(TokenInvalidoError):
        decodificar_token_sesion(token_ajeno)


def test_base_datos_insercion_y_consultas(bd_prueba: Path) -> None:
    """Prueba la creación de un usuario y su búsqueda por ID, nombre y correo."""
    usuario = insertar_usuario(
        id_usuario="usr-1",
        nombre_usuario="juan.perez",
        nombre_visible="Juan Pérez",
        correo="juan@example.com",
        hash_contrasena=hashear_contrasena("clave-juan-123"),
        fecha_alta="2026-11-09T12:00:00Z",
        ruta=bd_prueba,
    )

    assert usuario["id"] == "usr-1"
    assert usuario["correo"] == "juan@example.com"

    por_id = buscar_usuario_por_id("usr-1", ruta=bd_prueba)
    assert por_id is not None
    assert por_id["nombre_usuario"] == "juan.perez"

    por_nombre = buscar_usuario_por_nombre("juan.perez", ruta=bd_prueba)
    assert por_nombre is not None
    assert por_nombre["id"] == "usr-1"

    por_correo_mayus = buscar_usuario_por_correo("JUAN@EXAMPLE.COM", ruta=bd_prueba)
    assert por_correo_mayus is not None
    assert por_correo_mayus["id"] == "usr-1"


def test_base_datos_unicidad_nombre_usuario_y_correo(bd_prueba: Path) -> None:
    """Verifica que la base rechaza nombres de usuario o correos duplicados."""
    insertar_usuario(
        id_usuario="usr-1",
        nombre_usuario="pedro",
        nombre_visible="Pedro",
        correo="pedro@example.com",
        hash_contrasena=hashear_contrasena("clave-123456"),
        fecha_alta="2026-11-09T12:00:00Z",
        ruta=bd_prueba,
    )

    # Nombre de usuario repetido
    with pytest.raises(sqlite3.IntegrityError):
        insertar_usuario(
            id_usuario="usr-2",
            nombre_usuario="pedro",
            nombre_visible="Otro Pedro",
            correo="pedro2@example.com",
            hash_contrasena=hashear_contrasena("clave-123456"),
            fecha_alta="2026-11-09T12:00:00Z",
            ruta=bd_prueba,
        )

    # Correo repetido (incluso con diferente combinación de mayúsculas)
    with pytest.raises(sqlite3.IntegrityError):
        insertar_usuario(
            id_usuario="usr-3",
            nombre_usuario="pedro3",
            nombre_visible="Tercer Pedro",
            correo="PEDRO@EXAMPLE.COM",
            hash_contrasena=hashear_contrasena("clave-123456"),
            fecha_alta="2026-11-09T12:00:00Z",
            ruta=bd_prueba,
        )


def test_servidor_respeta_inv16_sin_tablas_financieras(bd_prueba: Path) -> None:
    """Verifica que la base del servidor no contenga tablas financieras (INV-16)."""
    inicializar_base_datos(bd_prueba)
    with conectar(bd_prueba) as conn:
        cursor = conn.execute(
            "SELECT name FROM sqlite_master "
            "WHERE type = 'table' AND name NOT LIKE 'sqlite_%';"
        )
        tablas = [fila["name"] for fila in cursor.fetchall()]

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
