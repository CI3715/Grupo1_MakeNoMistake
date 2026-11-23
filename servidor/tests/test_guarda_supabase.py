"""Pruebas de la guarda anti-borrado para Supabase (Angel - Tarea 3).

Garantiza que ninguna suite de pruebas con TRUNCATE pueda ejecutarse por error
contra la base de datos compartida en Supabase.
"""

import pytest
from apoyo_compartido import (
    MENSAJE_ERROR_SUPABASE,
    es_host_supabase,
    verificar_no_supabase,
)


def test_detecta_host_supabase_pooler() -> None:
    """Detecta URLs que usan el Session pooler de Supabase."""
    url = "postgresql://postgres.refid:password@aws-0-sa-east-1.pooler.supabase.com:5432/postgres?sslmode=require"
    assert es_host_supabase(url) is True


def test_detecta_host_supabase_directo() -> None:
    """Detecta URLs directas de Supabase."""
    url = "postgresql://postgres:password@db.refid.supabase.co:5432/postgres"
    assert es_host_supabase(url) is True


def test_no_bloquea_docker_compose_local() -> None:
    """Permite URLs que apuntan al PostgreSQL local de Docker Compose."""
    url = "postgresql://cuentas_claras:password_local_dev@localhost:55432/cuentas_claras_pruebas"
    assert es_host_supabase(url) is False


def test_no_bloquea_url_vacia_o_none() -> None:
    """Maneja valores None o vacíos sin fallar ni bloquear."""
    assert es_host_supabase(None) is False
    assert es_host_supabase("") is False


def test_verificar_no_supabase_falla_con_mensaje_claro_en_supabase() -> None:
    """verificar_no_supabase lanza pytest.fail con el mensaje oficial si es Supabase."""
    url = "postgresql://postgres:pass@db.refid.supabase.co:5432/postgres"
    with pytest.raises(pytest.fail.Exception) as exc_info:
        verificar_no_supabase(url)

    assert MENSAJE_ERROR_SUPABASE in str(exc_info.value)
    assert "Docker Compose" in str(exc_info.value)
    assert "55432" in str(exc_info.value)


def test_verificar_no_supabase_pasa_con_postgres_local() -> None:
    """verificar_no_supabase permite la ejecución si es localhost."""
    url = "postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras_pruebas"
    # No debe levantar ninguna excepción
    verificar_no_supabase(url)
    verificar_no_supabase(None)
