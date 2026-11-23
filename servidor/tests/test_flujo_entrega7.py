"""Flujo completo de la Entrega 7 como usuarios nuevos, con el almacén en memoria.

El recorrido vive en ``flujo_entrega7.py``; ``test_integracion_postgres.py``
repite el mismo contra PostgreSQL real.
"""

from fastapi.testclient import TestClient
from flujo_entrega7 import recorrer_flujo_entrega_7


def test_flujo_de_cambios_e_idempotencia_como_usuario_nuevo(
    cliente: TestClient,
) -> None:
    recibidos = recorrer_flujo_entrega_7(cliente)
    assert recibidos["dario"] == []
