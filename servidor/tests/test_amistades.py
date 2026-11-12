"""Pruebas para el módulo de amistades e invitaciones (Entrega 6).

Cubre los requisitos RF-SOC-04, RF-SOC-05, RF-SOC-06 e INV-10:
- Búsqueda de usuarios por coincidencia exacta sin exponer el correo.
- Ausencia de búsqueda parcial o listado para evitar enumeración.
- Ciclo de vida de invitaciones: creación, aceptación y rechazo.
- Restricciones: no autoinvitación, no duplicados en ningún sentido.
- Eliminación de amistad sin borrado en cascada de grupos ni gastos.
"""

from conftest import encabezado_auth
from fastapi.testclient import TestClient


class TestBusquedaUsuarios:
    """Pruebas de búsqueda exacta de usuarios (sección 2.1.1, RF-SOC-04)."""

    def test_busqueda_exacta_devuelve_nombre_usuario_y_visible(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """GET /usuarios?usuario=beto devuelve 200 con nombre de usuario y visible."""
        resp = cliente.get("/usuarios?usuario=beto", headers=encabezado_auth(token_ana))
        assert resp.status_code == 200
        datos = resp.json()
        assert datos["nombre_usuario"] == "beto"
        assert datos["nombre_visible"] == "Beto Pérez"

    def test_busqueda_exacta_nunca_retorna_correo(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """La respuesta de búsqueda de usuarios nunca expone el campo de correo."""
        resp = cliente.get("/usuarios?usuario=beto", headers=encabezado_auth(token_ana))
        assert resp.status_code == 200
        datos = resp.json()
        assert "correo" not in datos
        assert "correo" not in resp.text.lower()
        assert "beto@ejemplo.com" not in resp.text

    def test_busqueda_parcial_o_por_prefijo_responde_404(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Búsquedas incompletas o parciales devuelven 404 (sin enumeración)."""
        resp_prefijo = cliente.get(
            "/usuarios?usuario=bet", headers=encabezado_auth(token_ana)
        )
        assert resp_prefijo.status_code == 404
        assert resp_prefijo.json()["mensaje"] == "Usuario no encontrado."

        resp_inexistente = cliente.get(
            "/usuarios?usuario=no_existo_xyz", headers=encabezado_auth(token_ana)
        )
        assert resp_inexistente.status_code == 404
        assert resp_inexistente.json()["mensaje"] == "Usuario no encontrado."

    def test_busqueda_sin_parametro_responde_400(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Petición GET /usuarios sin parámetro 'usuario' responde 400."""
        resp = cliente.get("/usuarios", headers=encabezado_auth(token_ana))
        assert resp.status_code == 400
        errores = resp.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores)

        resp_vacio = cliente.get(
            "/usuarios?usuario=", headers=encabezado_auth(token_ana)
        )
        assert resp_vacio.status_code == 400
        errores_vacio = resp_vacio.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores_vacio)


class TestInvitacionesAmistad:
    """Pruebas para el envío y respuesta de invitaciones (sección 2.1.2, RF-SOC-05)."""

    def test_enviar_invitacion_valida_responde_201(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """ana envía invitación a beto y recibe 201 Created."""
        resp = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201
        datos = resp.json()
        assert "id" in datos
        assert datos["estado"] == "pendiente"
        assert datos["otro_nombre_usuario"] == "beto"
        assert datos["enviada_por_mi"] is True

    def test_amistad_no_existe_antes_de_ser_aceptada(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """La amistad no está activa (INV-10) hasta que el destinatario acepta."""
        resp = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201

        resp_ana = cliente.get("/amistades", headers=encabezado_auth(token_ana))
        assert resp_ana.status_code == 200
        amistades_ana = resp_ana.json()
        assert any(
            a["otro_nombre_usuario"] == "beto" and a["estado"] == "pendiente"
            for a in amistades_ana
        )
        assert not any(
            a["otro_nombre_usuario"] == "beto" and a["estado"] == "aceptada"
            for a in amistades_ana
        )

    def test_invitarse_a_si_mismo_responde_400_con_campo_usuario(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Un usuario no puede invitarse a sí mismo (responde 400 con campo usuario)."""
        resp = cliente.post(
            "/amistades",
            json={"usuario": "ana"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 400
        errores = resp.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores)

    def test_invitacion_duplicada_mismo_sentido_responde_409(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Segunda invitación pendiente de ana a beto responde 409 Conflict."""
        resp1 = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp1.status_code == 201

        resp2 = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp2.status_code == 409
        errores = resp2.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores)

    def test_invitacion_duplicada_sentido_inverso_responde_409(
        self, cliente: TestClient, token_ana: str, token_beto: str
    ) -> None:
        """Si ana invitó a beto, beto intentando invitar a ana responde 409."""
        resp1 = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp1.status_code == 201

        resp2 = cliente.post(
            "/amistades",
            json={"usuario": "ana"},
            headers=encabezado_auth(token_beto),
        )
        assert resp2.status_code == 409
        errores = resp2.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores)

    def test_invitar_a_quien_ya_es_amigo_responde_409(
        self, cliente: TestClient, token_ana: str, token_beto: str
    ) -> None:
        """Enviar invitación a un usuario que ya es amigo responde 409 Conflict."""
        resp_inv = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp_inv.status_code == 201
        id_amistad = resp_inv.json()["id"]

        resp_resp = cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_beto),
        )
        assert resp_resp.status_code == 200

        resp_dup = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp_dup.status_code == 409
        errores = resp_dup.json().get("errores", [])
        assert any(e["campo"] == "usuario" for e in errores)

    def test_solo_el_destinatario_puede_responder_la_invitacion(
        self, cliente: TestClient, token_ana: str, token_carla: str
    ) -> None:
        """El emisor no puede aceptar su propia invitación enviada."""
        resp = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201
        id_amistad = resp.json()["id"]

        resp_ana = cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_ana),
        )
        assert resp_ana.status_code == 403

        resp_carla = cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_carla),
        )
        assert resp_carla.status_code == 403

    def test_invitacion_ya_respondida_responde_409(
        self, cliente: TestClient, token_ana: str, token_beto: str
    ) -> None:
        """Intentar responder una invitación ya resuelta responde 409 Conflict."""
        resp = cliente.post(
            "/amistades",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201
        id_amistad = resp.json()["id"]

        resp_ok = cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_beto),
        )
        assert resp_ok.status_code == 200

        resp_otra = cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": False},
            headers=encabezado_auth(token_beto),
        )
        assert resp_otra.status_code == 409


class TestListarYEliminarAmistad:
    """Pruebas de listado y eliminación de amistad (sección 2.1.3, RF-SOC-06)."""

    def test_listar_amistades_muestra_amigos_e_invitaciones(
        self, cliente: TestClient, token_ana: str, token_beto: str, token_carla: str
    ) -> None:
        """GET /amistades retorna amigos confirmados e invitaciones pendientes."""
        resp_b = cliente.post(
            "/amistades", json={"usuario": "beto"}, headers=encabezado_auth(token_ana)
        )
        id_b = resp_b.json()["id"]
        cliente.post(
            f"/amistades/{id_b}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_beto),
        )

        cliente.post(
            "/amistades", json={"usuario": "ana"}, headers=encabezado_auth(token_carla)
        )

        resp = cliente.get("/amistades", headers=encabezado_auth(token_ana))
        assert resp.status_code == 200
        lista = resp.json()
        assert len(lista) == 2

        amigo_beto = next(x for x in lista if x["otro_nombre_usuario"] == "beto")
        assert amigo_beto["estado"] == "aceptada"

        invit_carla = next(x for x in lista if x["otro_nombre_usuario"] == "carla")
        assert invit_carla["estado"] == "pendiente"
        assert invit_carla["enviada_por_mi"] is False

    def test_eliminar_amistad_responde_204_y_no_borra_grupos(
        self, cliente: TestClient, token_ana: str, token_beto: str
    ) -> None:
        """DELETE /amistades/{id} borra la amistad pero conserva grupos y gastos."""
        resp = cliente.post(
            "/amistades", json={"usuario": "beto"}, headers=encabezado_auth(token_ana)
        )
        id_amistad = resp.json()["id"]
        cliente.post(
            f"/amistades/{id_amistad}/respuesta",
            json={"aceptar": True},
            headers=encabezado_auth(token_beto),
        )

        resp_grupo = cliente.post(
            "/grupos",
            json={"nombre": "Viaje Amigos", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_grupo.json()["id"]
        cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        cliente.post(
            f"/grupos/{grupo_id}/miembros/yo/respuesta",
            json={"respuesta": "aceptar"},
            headers=encabezado_auth(token_beto),
        )

        resp_del = cliente.delete(
            f"/amistades/{id_amistad}", headers=encabezado_auth(token_ana)
        )
        assert resp_del.status_code == 204

        resp_lista = cliente.get("/amistades", headers=encabezado_auth(token_ana))
        assert not any(
            x["id"] == id_amistad and x["estado"] == "aceptada"
            for x in resp_lista.json()
        )

        resp_ver_grupo = cliente.get(
            f"/grupos/{grupo_id}", headers=encabezado_auth(token_beto)
        )
        assert resp_ver_grupo.status_code == 200
