import pytest

from configuracion import Configuracion, cargar_configuracion

URL = "postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras"
SECRETO = "x" * 32


def test_carga_configuracion_valida() -> None:
    configuracion = cargar_configuracion(
        {
            "CUENTAS_CLARAS_DB_URL": URL,
            "CUENTAS_CLARAS_SECRETO_JWT": SECRETO,
            "CUENTAS_CLARAS_MINUTOS_TOKEN": "30",
        }
    )

    assert configuracion == Configuracion(
        url_base_datos=URL, secreto_jwt=SECRETO, minutos_token=30
    )


def test_minutos_token_por_defecto() -> None:
    configuracion = cargar_configuracion(
        {
            "CUENTAS_CLARAS_DB_URL": URL,
            "CUENTAS_CLARAS_SECRETO_JWT": SECRETO,
        }
    )

    assert configuracion.minutos_token == 60


def test_falta_url_base_datos() -> None:
    with pytest.raises(RuntimeError, match="CUENTAS_CLARAS_DB_URL"):
        cargar_configuracion({"CUENTAS_CLARAS_SECRETO_JWT": SECRETO})


def test_secreto_demasiado_corto() -> None:
    with pytest.raises(RuntimeError, match="CUENTAS_CLARAS_SECRETO_JWT"):
        cargar_configuracion(
            {
                "CUENTAS_CLARAS_DB_URL": URL,
                "CUENTAS_CLARAS_SECRETO_JWT": "corto",
            }
        )


def test_minutos_invalidos() -> None:
    with pytest.raises(RuntimeError, match="CUENTAS_CLARAS_MINUTOS_TOKEN"):
        cargar_configuracion(
            {
                "CUENTAS_CLARAS_DB_URL": URL,
                "CUENTAS_CLARAS_SECRETO_JWT": SECRETO,
                "CUENTAS_CLARAS_MINUTOS_TOKEN": "cero",
            }
        )


def test_minutos_no_positivos() -> None:
    with pytest.raises(RuntimeError, match="mayor que cero"):
        cargar_configuracion(
            {
                "CUENTAS_CLARAS_DB_URL": URL,
                "CUENTAS_CLARAS_SECRETO_JWT": SECRETO,
                "CUENTAS_CLARAS_MINUTOS_TOKEN": "0",
            }
        )
