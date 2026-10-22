from datetime import datetime, timezone

from fastapi.testclient import TestClient

from main import app

client = TestClient(app)

CAMPOS_ESPERADOS = {"estado", "mensaje", "version", "timestamp"}
FORMATO_TIMESTAMP = "%Y-%m-%dT%H:%M:%SZ"


def test_ping() -> None:
    response = client.get("/ping")

    assert response.status_code == 200


def test_respuesta_es_json() -> None:
    response = client.get("/ping")

    assert response.headers["content-type"].startswith("application/json")


def test_campos() -> None:
    response = client.get("/ping")

    assert set(response.json()) == CAMPOS_ESPERADOS


def test_estado() -> None:
    response = client.get("/ping")

    assert response.json()["estado"] == "ok"


def test_mensaje() -> None:
    response = client.get("/ping")

    assert response.json()["mensaje"] == "Servidor Cuentas Claras activo"


def test_version() -> None:
    response = client.get("/ping")

    assert response.json()["version"] == "0.1.0"


def test_timestamp_formato_utc() -> None:
    timestamp = client.get("/ping").json()["timestamp"]

    datetime.strptime(timestamp, FORMATO_TIMESTAMP)


def test_timestamp_es_actual() -> None:
    timestamp = client.get("/ping").json()["timestamp"]

    recibido = datetime.strptime(timestamp, FORMATO_TIMESTAMP).replace(
        tzinfo=timezone.utc
    )
    ahora = datetime.now(timezone.utc)

    assert abs((ahora - recibido).total_seconds()) < 5


def test_ping_solo_lectura() -> None:
    response = client.post("/ping")

    assert response.status_code == 405


def test_documentacion_disponible() -> None:
    assert client.get("/docs").status_code == 200

    rutas = client.get("/openapi.json").json()["paths"]

    assert "/ping" in rutas
