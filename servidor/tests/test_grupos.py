"""Pruebas para grupos, miembros, permisos y control de acceso (Entrega 6).

Cubre los requisitos RF-COM-01, RF-COM-02, INV-17, RNF-07 y ESC-08:
- Creación de grupo con creador como miembro y administrador.
- Invitación a grupos restringida a administradores y amigos.
- Respuesta 404 uniforme para no miembros e invitados no aceptados.
- Visibilidad de gastos y balances exclusiva para miembros confirmados.
"""

from typing import Any
from uuid import UUID, uuid4

from conftest import encabezado_auth
from fastapi.testclient import TestClient
from repositorio_en_memoria import RepositorioEnMemoria


class TestCreacionYListadoGrupos:
    """Pruebas para crear y listar grupos (sección 2.2, RF-COM-01)."""

    def test_crear_grupo_valido_asigna_creador_como_administrador(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """POST /grupos responde 201 y asigna a ana rol de administrador."""
        resp = cliente.post(
            "/grupos",
            json={
                "nombre": "Viaje a Mérida",
                "descripcion": "Gastos del viaje de marzo",
                "moneda": "USD",
                "actividad": "Viaje a Mérida",
            },
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201
        datos = resp.json()
        assert "id" in datos
        assert datos["nombre"] == "Viaje a Mérida"
        assert datos["moneda"] == "USD"
        assert datos["rol"] == "administrador"
        assert datos["cantidad_miembros"] == 1

    def test_crear_grupo_sin_nombre_responde_400(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Nombre es obligatorio; responde 400 con error en el campo 'nombre'."""
        resp = cliente.post(
            "/grupos",
            json={"nombre": "", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 400
        errores = resp.json()["errores"]
        assert any(e["campo"] == "nombre" for e in errores)

    def test_crear_grupo_moneda_invalida_responde_400(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """Moneda debe ser código ISO de 3 letras mayúsculas; si no, responde 400."""
        resp = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Prueba", "moneda": "dolares"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 400
        errores = resp.json()["errores"]
        assert any(e["campo"] == "moneda" for e in errores)

    def test_listar_mis_grupos_incluye_invitaciones_pendientes(
        self,
        cliente: TestClient,
        token_ana: str,
        token_beto: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """GET /grupos devuelve los grupos del usuario y sus invitaciones pendientes."""
        resp_crear = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Ana", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        assert resp_crear.status_code == 201
        grupo_id = resp_crear.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        beto_id = UUID(usuarios_flujo["beto"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, beto_id)

        resp_inv = cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp_inv.status_code == 201

        resp_ana = cliente.get("/grupos", headers=encabezado_auth(token_ana))
        assert resp_ana.status_code == 200
        datos_ana = resp_ana.json()
        assert any(g["id"] == grupo_id for g in datos_ana["grupos"])

        resp_beto = cliente.get("/grupos", headers=encabezado_auth(token_beto))
        assert resp_beto.status_code == 200
        datos_beto = resp_beto.json()
        assert any(i["grupo_id"] == grupo_id for i in datos_beto["invitaciones"])


class TestMiembrosEInvitacionesGrupo:
    """Pruebas para invitar y gestionar miembros (sección 2.2, RF-COM-02)."""

    def test_administrador_invita_a_un_amigo_responde_201(
        self,
        cliente: TestClient,
        token_ana: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """ana (admin) invita a beto (amigo) y responde 201 Created."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Amigos", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        beto_id = UUID(usuarios_flujo["beto"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, beto_id)

        resp = cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 201
        datos = resp.json()
        assert datos["usuario"] == "beto"
        assert datos["rol"] == "miembro"
        assert datos["estado"] == "invitado"

    def test_miembro_no_administrador_no_puede_invitar_responde_403(
        self,
        cliente: TestClient,
        token_ana: str,
        token_beto: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """Un miembro sin rol administrador no puede invitar (403 Forbidden)."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Test", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        beto_id = UUID(usuarios_flujo["beto"]["perfil"]["id"])
        carla_id = UUID(usuarios_flujo["carla"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, beto_id)
        repositorio_falso.registrar_amistad_aceptada(beto_id, carla_id)

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

        resp = cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "carla"},
            headers=encabezado_auth(token_beto),
        )
        assert resp.status_code == 403
        assert "administrador" in resp.json()["mensaje"].lower()

    def test_invitar_a_alguien_que_no_es_amigo_responde_403(
        self, cliente: TestClient, token_ana: str
    ) -> None:
        """No se puede invitar a un usuario con quien no se tiene amistad (403)."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Solo Amigos", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        resp = cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "dario"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 403
        assert "amigos" in resp.json()["mensaje"].lower()

    def test_invitar_a_usuario_ya_miembro_o_invitado_responde_409(
        self,
        cliente: TestClient,
        token_ana: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """Invitar a quien ya pertenece o tiene invitación pendiente responde 409."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Duplicado", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        beto_id = UUID(usuarios_flujo["beto"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, beto_id)

        cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        resp = cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )
        assert resp.status_code == 409
        errores = resp.json()["errores"]
        assert any(e["campo"] == "usuario" for e in errores)

    def test_responder_invitacion_de_grupo_solo_por_el_invitado(
        self,
        cliente: TestClient,
        token_ana: str,
        token_beto: str,
        token_carla: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """Solo el usuario invitado puede aceptar o rechazar la membresía."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Respuestas", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        beto_id = UUID(usuarios_flujo["beto"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, beto_id)

        cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "beto"},
            headers=encabezado_auth(token_ana),
        )

        resp_carla = cliente.post(
            f"/grupos/{grupo_id}/miembros/yo/respuesta",
            json={"respuesta": "aceptar"},
            headers=encabezado_auth(token_carla),
        )
        assert resp_carla.status_code == 404

        resp_beto = cliente.post(
            f"/grupos/{grupo_id}/miembros/yo/respuesta",
            json={"respuesta": "aceptar"},
            headers=encabezado_auth(token_beto),
        )
        assert resp_beto.status_code == 200
        assert "aceptada" in resp_beto.json()["mensaje"].lower()


class TestControlDeAccesoY404Uniforme:
    """Pruebas de aislamiento, permisos y 404 uniforme (ESC-08, INV-17, RNF-07)."""

    def test_no_miembro_recibe_404_identico_a_grupo_inexistente(
        self, cliente: TestClient, token_ana: str, token_dario: str
    ) -> None:
        """dario (no miembro) recibe el mismo 404 y cuerpo que ante id inventado."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Secreto", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_real_id = resp_g.json()["id"]
        id_inventado = str(uuid4())

        resp_real = cliente.get(
            f"/grupos/{grupo_real_id}", headers=encabezado_auth(token_dario)
        )
        resp_inventado = cliente.get(
            f"/grupos/{id_inventado}", headers=encabezado_auth(token_dario)
        )

        assert resp_real.status_code == 404
        assert resp_inventado.status_code == 404
        assert resp_real.json() == resp_inventado.json()
        assert resp_real.json() == {"mensaje": "Grupo no encontrado."}

    def test_invitado_sin_aceptar_no_ve_gastos_ni_balances(
        self,
        cliente: TestClient,
        token_ana: str,
        token_carla: str,
        usuarios_flujo: dict[str, dict[str, Any]],
        repositorio_falso: RepositorioEnMemoria,
    ) -> None:
        """carla no ve gastos ni balances hasta aceptar la invitación al grupo."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Protegido", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        ana_id = UUID(usuarios_flujo["ana"]["perfil"]["id"])
        carla_id = UUID(usuarios_flujo["carla"]["perfil"]["id"])
        repositorio_falso.registrar_amistad_aceptada(ana_id, carla_id)

        cliente.post(
            f"/grupos/{grupo_id}/miembros",
            json={"usuario": "carla"},
            headers=encabezado_auth(token_ana),
        )

        resp = cliente.get(f"/grupos/{grupo_id}", headers=encabezado_auth(token_carla))
        assert resp.status_code == 404
        assert resp.json() == {"mensaje": "Grupo no encontrado."}

    def test_no_miembro_no_puede_registrar_gasto_responde_404(
        self, cliente: TestClient, token_ana: str, token_dario: str
    ) -> None:
        """dario intentando registrar gasto en el grupo recibe 404 uniforme."""
        resp_g = cliente.post(
            "/grupos",
            json={"nombre": "Grupo Gastos", "moneda": "USD"},
            headers=encabezado_auth(token_ana),
        )
        grupo_id = resp_g.json()["id"]

        resp = cliente.post(
            f"/grupos/{grupo_id}/gastos",
            json={"concepto": "Cena", "monto": "50.00"},
            headers=encabezado_auth(token_dario),
        )
        assert resp.status_code == 404
        assert resp.json() == {"mensaje": "Grupo no encontrado."}

    def test_operaciones_sin_token_responden_401(self, cliente: TestClient) -> None:
        """Cualquier ruta de grupos sin encabezado Bearer responde 401."""
        id_grupo = str(uuid4())
        assert cliente.get("/grupos").status_code == 401
        assert (
            cliente.post(
                "/grupos", json={"nombre": "Grupo", "moneda": "USD"}
            ).status_code
            == 401
        )
        assert cliente.get(f"/grupos/{id_grupo}").status_code == 401
        assert (
            cliente.post(
                f"/grupos/{id_grupo}/miembros", json={"usuario": "ana"}
            ).status_code
            == 401
        )
        assert (
            cliente.post(
                f"/grupos/{id_grupo}/miembros/yo/respuesta",
                json={"respuesta": "aceptar"},
            ).status_code
            == 401
        )
        assert cliente.post(f"/grupos/{id_grupo}/gastos").status_code == 401
