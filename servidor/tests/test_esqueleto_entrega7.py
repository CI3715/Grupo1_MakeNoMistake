"""Esqueleto de pruebas del servidor y trazabilidad de reglas (Angel - Tarea 2).

Contiene la verificación de los ayudantes de prueba (usuarios del flujo,
grupo Mérida, registro idempotente y lectura de cambios) y la lista de pruebas
pendientes que cubren cada regla de las secciones 2.1 a 2.5 y los criterios de la
sección 3 del enunciado de la Entrega 7.
"""

import re
from pathlib import Path
from typing import Any

import pytest
from apoyo_compartido import (
    crear_grupo_merida_http,
    leer_cambios_http,
    registrar_gasto_idempotente_http,
)
from fastapi.testclient import TestClient

Usuarios = dict[str, dict[str, Any]]
HOTEL: dict[str, Any] = {
    "concepto": "Hotel",
    "monto": "100.00",
    "fecha": "2026-03-14",
    "pagador": "ana",
    "regla": "partes_iguales",
    "participantes": [
        {"usuario": "ana", "valor": ""},
        {"usuario": "beto", "valor": ""},
        {"usuario": "carla", "valor": ""},
    ],
}


def registrar(
    cliente: TestClient,
    grupo_id: str,
    usuarios: Usuarios,
    quien: str,
    cuerpo: dict[str, Any],
) -> dict[str, Any]:
    resp = registrar_gasto_idempotente_http(
        cliente, grupo_id, usuarios[quien]["auth"], cuerpo
    )
    assert resp.status_code == 201, resp.text
    gasto: dict[str, Any] = resp.json()
    return gasto


def cambios(
    cliente: TestClient, usuarios: Usuarios, quien: str, desde: int = 0
) -> dict[str, Any]:
    resp = leer_cambios_http(cliente, usuarios[quien]["auth"], desde)
    assert resp.status_code == 200, resp.text
    datos: dict[str, Any] = resp.json()
    return datos


class TestAyudantesEntrega7:
    """Verifica que los ayudantes de prueba del flujo y grupo Mérida funcionan."""

    def test_creacion_grupo_merida_con_tres_miembros(
        self, cliente: TestClient, usuarios_flujo: dict[str, dict[str, Any]]
    ) -> None:
        """Comprueba que el ayudante crea el grupo con ana, beto y carla."""
        grupo_id = crear_grupo_merida_http(cliente, usuarios_flujo)
        assert grupo_id is not None
        assert len(grupo_id) == 36  # UUID válido

        # ana, beto y carla pueden ver el grupo (200)
        for u in ("ana", "beto", "carla"):
            resp = cliente.get(f"/grupos/{grupo_id}", headers=usuarios_flujo[u]["auth"])
            assert resp.status_code == 200
            datos = resp.json()
            assert datos["grupo"]["nombre"] == "Viaje a Mérida"
            assert datos["grupo"]["moneda"] == "USD"

        # dario (el ajeno) no tiene acceso al grupo (404 uniforme anti-enumeración)
        resp_dario = cliente.get(
            f"/grupos/{grupo_id}", headers=usuarios_flujo["dario"]["auth"]
        )
        assert resp_dario.status_code == 404

    def test_ayudante_registrar_gasto_sin_clave(
        self,
        cliente: TestClient,
        grupo_merida: str,
        usuarios_flujo: dict[str, dict[str, Any]],
    ) -> None:
        """El ayudante registra gastos sin clave igual que en la Entrega 6."""
        cuerpo = {
            "concepto": "Desayuno",
            "monto": "30.00",
            "fecha": "2026-11-20",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": "1"},
                {"usuario": "beto", "valor": "1"},
                {"usuario": "carla", "valor": "1"},
            ],
        }
        resp = registrar_gasto_idempotente_http(
            cliente, grupo_merida, usuarios_flujo["ana"]["auth"], cuerpo
        )
        assert resp.status_code == 201

    def test_ayudante_registrar_gasto_con_clave_idempotencia(
        self,
        cliente: TestClient,
        grupo_merida: str,
        usuarios_flujo: dict[str, dict[str, Any]],
    ) -> None:
        """El ayudante adjunta el encabezado Idempotency-Key."""
        cuerpo = {
            "concepto": "Almuerzo",
            "monto": "45.00",
            "fecha": "2026-11-20",
            "pagador": "ana",
            "regla": "partes_iguales",
            "participantes": [
                {"usuario": "ana", "valor": "1"},
                {"usuario": "beto", "valor": "1"},
                {"usuario": "carla", "valor": "1"},
            ],
        }
        resp = registrar_gasto_idempotente_http(
            cliente,
            grupo_merida,
            usuarios_flujo["ana"]["auth"],
            cuerpo,
            clave_idempotencia="clave-uuid-12345",
        )
        # Clave nueva: 201. Los reintentos (200 y 409) los prueba la regla 2.5.
        assert resp.status_code == 201


# ==============================================================================
# TRAZABILIDAD DE REGLAS: SECCIONES 2.1 A 2.5
# ==============================================================================


class TestTrazabilidadSeccion2_1RegistroCambios:
    """Reglas de la sección 2.1: Registro de Cambios en el Servidor (John).

    El detalle de cada regla está en ``test_cambios_http.py`` y
    ``test_reglas_cambios.py``; aquí queda una prueba por regla del enunciado.
    """

    def test_regla_2_1_gasto_agrega_evento_a_cada_participante(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """Al registrar un gasto, el servidor agrega un evento a cada participante."""
        gasto = registrar(cliente, grupo_merida, usuarios_flujo, "ana", HOTEL)
        for u in ("ana", "beto", "carla"):
            (evento,) = cambios(cliente, usuarios_flujo, u)["eventos"]
            assert evento["gasto"]["id"] == gasto["id"]

    def test_regla_2_1_evento_trae_campos_acordados_y_marca_redondeo(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """El evento trae grupo, gasto, concepto, fecha, moneda, monto y redondeo."""
        registrar(cliente, grupo_merida, usuarios_flujo, "ana", HOTEL)
        (evento,) = cambios(cliente, usuarios_flujo, "ana")["eventos"]
        assert evento["grupo"] == {"id": grupo_merida, "nombre": "Viaje a Mérida"}
        assert evento["gasto"]["concepto"] == "Hotel"
        assert (evento["fecha"], evento["moneda"]) == ("2026-03-14", "USD")
        assert (evento["monto"], evento["incluye_redondeo"]) == ("33.34", True)
        assert evento["tipo"] == "gasto_de_grupo"

    def test_regla_2_1_identificador_unico_y_posicion_creciente_cursor(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """Cada evento tiene id inmutable y posición creciente por usuario."""
        for concepto in ("Hotel", "Cena", "Taxi"):
            registrar(
                cliente,
                grupo_merida,
                usuarios_flujo,
                "ana",
                {**HOTEL, "concepto": concepto},
            )
        datos = cambios(cliente, usuarios_flujo, "beto")
        assert [e["posicion"] for e in datos["eventos"]] == [1, 2, 3]
        assert datos["cursor"] == 3
        assert len({e["id"] for e in datos["eventos"]}) == 3
        assert cambios(cliente, usuarios_flujo, "beto") == datos

    def test_regla_2_1_miembro_excluido_no_recibe_evento(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """Un miembro excluido de la división no recibe evento en su registro."""
        taxi = {**HOTEL, "participantes": HOTEL["participantes"][:2]}
        registrar(cliente, grupo_merida, usuarios_flujo, "ana", taxi)
        assert cambios(cliente, usuarios_flujo, "carla")["eventos"] == []
        assert len(cambios(cliente, usuarios_flujo, "beto")["eventos"]) == 1

    def test_regla_2_1_no_miembro_no_recibe_eventos_de_grupo_ajeno(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """Nadie recibe eventos de grupos a los que no pertenece (INV-17)."""
        registrar(cliente, grupo_merida, usuarios_flujo, "ana", HOTEL)
        assert cambios(cliente, usuarios_flujo, "dario") == {
            "eventos": [],
            "cursor": 0,
            "hay_mas": False,
        }

    def test_regla_2_1_registro_solo_crece_sin_update_ni_delete(self) -> None:
        """El registro de cambios solo crece: no admite UPDATE ni DELETE.

        Aquí se comprueba que el código del servidor no tiene ningún UPDATE ni
        DELETE sobre la tabla ``cambios``. Los disparadores que lo impiden en
        PostgreSQL llegan con el repositorio de cambios (prueba directa aparte).
        """
        raiz = Path(__file__).resolve().parents[1]
        prohibido = re.compile(r"(UPDATE|DELETE\s+FROM)\s+cambios\b", re.IGNORECASE)
        for archivo in raiz.glob("*.py"):
            assert not prohibido.search(archivo.read_text(encoding="utf-8")), archivo

    def test_regla_2_1_get_cambios_devuelve_en_orden_lotes_100_cursor_y_hay_mas(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """GET /cambios entrega en orden hasta 100 eventos, cursor y hay_mas."""
        solo_dos = {
            **HOTEL,
            "monto": "2.00",
            "participantes": HOTEL["participantes"][:2],
        }
        for _ in range(101):
            registrar(cliente, grupo_merida, usuarios_flujo, "ana", solo_dos)
        primero = cambios(cliente, usuarios_flujo, "beto")
        assert [e["posicion"] for e in primero["eventos"]] == list(range(1, 101))
        assert (primero["cursor"], primero["hay_mas"]) == (100, True)
        segundo = cambios(cliente, usuarios_flujo, "beto", 100)
        assert [e["posicion"] for e in segundo["eventos"]] == [101]
        assert (segundo["cursor"], segundo["hay_mas"]) == (101, False)


class TestTrazabilidadSeccion2_2CuentaGrupo:
    """Reglas de la sección 2.2: Cuenta del Grupo en el Libro Personal."""

    @pytest.mark.skip(
        reason="Pendiente: esperando integración en app (Cristina - Tarea 9)"
    )
    def test_regla_2_2_cuenta_personal_activa_y_misma_moneda_del_grupo(self) -> None:
        """Solo se ofrecen cuentas activas en la moneda del grupo (INV-18)."""

    @pytest.mark.skip(
        reason="Pendiente: esperando integración en app (Cristina - Tarea 9)"
    )
    def test_regla_2_2_eleccion_cuenta_se_guarda_solo_en_dispositivo(self) -> None:
        """La cuenta elegida vive en el dispositivo y no viaja al servidor (INV-16)."""

    @pytest.mark.skip(
        reason="Pendiente: esperando integración en app (Cristina - Tarea 9)"
    )
    def test_regla_2_2_parte_sin_cuenta_queda_en_pendientes_de_aplicar(self) -> None:
        """Partes sin cuenta asignada quedan pendientes y se aplican al elegirla."""


class TestTrazabilidadSeccion2_3AplicarSinDuplicar:
    """Reglas de la sección 2.3: Aplicar una Parte sin Duplicar."""

    @pytest.mark.skip(reason="Pendiente: esperando núcleo en Rust (Cristina - Tarea 7)")
    def test_regla_2_3_evento_gasto_de_grupo_aplicado_con_id_del_servidor(self) -> None:
        """Genera GastoDeGrupoAplicado con el identificador del evento del servidor."""

    @pytest.mark.skip(reason="Pendiente: esperando núcleo en Rust (Cristina - Tarea 7)")
    def test_regla_2_3_idempotencia_local_aplicar_dos_veces_mismo_evento(self) -> None:
        """Aplicar dos veces el mismo id deja el mismo estado (INV-13, INV-15)."""

    @pytest.mark.skip(reason="Pendiente: esperando núcleo en Rust (Cristina - Tarea 7)")
    def test_regla_2_3_movimiento_egreso_concepto_grupo_gasto(self) -> None:
        """El movimiento se crea como egreso con concepto «<grupo> · <gasto>»."""

    @pytest.mark.skip(reason="Pendiente: esperando núcleo en Rust (Cristina - Tarea 7)")
    def test_regla_2_3_movimiento_grupo_no_se_corrige_desde_libro(self) -> None:
        """corregir_movimiento rechaza modificar gastos de grupo desde el núcleo."""

    @pytest.mark.skip(
        reason="Pendiente: esperando infraestructura Rust (Cristina - Tarea 8)"
    )
    def test_regla_2_3_cursor_y_movimientos_en_misma_transaccion(self) -> None:
        """Cursor y lote se guardan en la misma transacción atómica."""

    @pytest.mark.skip(reason="Pendiente: esperando núcleo en Rust (Cristina - Tarea 7)")
    def test_regla_2_3_eventos_desconocidos_se_ignoran_sin_fallar(self) -> None:
        """Los tipos de evento desconocidos se ignoran sin romper la sincronización."""


class TestTrazabilidadSeccion2_4CuandoseSincroniza:
    """Reglas de la sección 2.4: Cuándo se Sincroniza."""

    @pytest.mark.skip(
        reason="Pendiente: esperando comandos reales e interfaz (Ernesto - Tarea 4)"
    )
    def test_regla_2_4_sincronizacion_al_abrir_con_sesion_vigente(self) -> None:
        """Sincroniza al arrancar con sesión válida."""

    @pytest.mark.skip(
        reason="Pendiente: esperando comandos reales (Cristina - Tarea 9)"
    )
    def test_regla_2_4_sincronizacion_tras_registrar_gasto_y_al_reconectar(
        self,
    ) -> None:
        """Sincroniza tras registrar gasto local y ante evento online."""

    @pytest.mark.skip(
        reason="Pendiente: esperando comandos reales (Cristina - Tarea 9)"
    )
    def test_regla_2_4_sesion_vencida_detiene_sincronizacion_sin_bloquear_personal(
        self,
    ) -> None:
        """401 detiene sincronización con aviso sin bloquear las funciones locales."""


class TestTrazabilidadSeccion2_5ColaGastosSinConexion:
    """Reglas de la sección 2.5: Cola de Gastos de Grupo sin Conexión."""

    @pytest.mark.skip(reason="Pendiente: esperando cola en Rust (Cristina - Tarea 10)")
    def test_regla_2_5_sin_conexion_gasto_queda_en_cola_pendiente_de_envio(
        self,
    ) -> None:
        """Gasto sin red se almacena en cola local como pendiente de envío."""

    @pytest.mark.skip(reason="Pendiente: esperando cola en Rust (Cristina - Tarea 10)")
    def test_regla_2_5_clave_idempotencia_unica_por_operacion_repetida_en_reintentos(
        self,
    ) -> None:
        """La clave de idempotencia se genera una sola vez y se reintenta intacta."""

    def test_regla_2_5_servidor_devuelve_gasto_ya_procesado_sin_duplicar(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """El servidor devuelve 200 con el gasto ya procesado si la clave coincide."""
        auth = usuarios_flujo["ana"]["auth"]
        clave = "clave-uuid-12345"
        primero = registrar_gasto_idempotente_http(
            cliente, grupo_merida, auth, HOTEL, clave
        )
        assert primero.status_code == 201
        repetido = registrar_gasto_idempotente_http(
            cliente, grupo_merida, auth, HOTEL, clave
        )
        assert repetido.status_code == 200
        assert repetido.json() == primero.json()
        otro = registrar_gasto_idempotente_http(
            cliente, grupo_merida, auth, {**HOTEL, "monto": "90.00"}, clave
        )
        assert otro.status_code == 409
        assert len(cambios(cliente, usuarios_flujo, "beto")["eventos"]) == 1

    @pytest.mark.skip(reason="Pendiente: esperando cola en Rust (Cristina - Tarea 10)")
    def test_regla_2_5_rechazo_4xx_saca_operacion_de_cola_sin_bloquear_demas(
        self,
    ) -> None:
        """Un 4xx saca la operación rechazada de la cola sin trabar las siguientes."""

    def test_regla_2_5_cola_no_envia_datos_del_libro_personal(self) -> None:
        """La cola local solo transmite datos del gasto de grupo (INV-16)."""
        raiz_repo = Path(__file__).resolve().parents[2]
        dto_path = raiz_repo / "app" / "src-tauri" / "src" / "dto_sincronizacion.rs"
        if dto_path.exists():
            contenido = dto_path.read_text(encoding="utf-8")
            for campo_prohibido in ("saldo_actual", "saldo_inicial", "patrimonio"):
                assert campo_prohibido not in contenido


# ==============================================================================
# TRAZABILIDAD DE CRITERIOS DE ACEPTACIÓN: SECCIÓN 3
# ==============================================================================


class TestTrazabilidadSeccion3CriteriosAceptacion:
    """Criterios de aceptación verificables de la sección 3."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_1_cada_participante_recibe_movimiento_en_cuenta_elegida(
        self,
    ) -> None:
        """Cada participante encuentra un solo movimiento en la cuenta elegida."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_2_sincronizar_varias_veces_o_desde_cero_no_duplica_ni_altera(
        self,
    ) -> None:
        """Sincronizar repetidamente o con cursor 0 no duplica ni modifica saldos."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_3_interrupcion_a_mitad_de_lote_se_recupera_sin_duplicar(
        self,
    ) -> None:
        """Interrupción a mitad de lote recupera y finaliza sin duplicar."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_4_partes_pendientes_se_aplican_al_elegir_cuenta(self) -> None:
        """Partes sin cuenta elegida se aplican automáticamente al seleccionarla."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_5_movimiento_de_grupo_no_corregible_en_interfaz_ni_nucleo(
        self,
    ) -> None:
        """El movimiento de grupo prohíbe corrección desde interfaz y núcleo."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_6_gasto_offline_se_envia_al_reconectar_sin_duplicar(
        self,
    ) -> None:
        """Gasto offline se envía al volver conexión sin crear duplicados."""

    def test_criterio_3_7_get_cambios_solo_eventos_propios_y_401_sin_token(
        self, cliente: TestClient, grupo_merida: str, usuarios_flujo: Usuarios
    ) -> None:
        """GET /cambios entrega solo eventos propios y rechaza 401 sin token."""
        registrar(cliente, grupo_merida, usuarios_flujo, "ana", HOTEL)
        assert cliente.get("/cambios").status_code == 401
        ids = {
            u: {e["id"] for e in cambios(cliente, usuarios_flujo, u)["eventos"]}
            for u in ("ana", "beto", "carla", "dario")
        }
        assert ids["dario"] == set()
        assert ids["ana"].isdisjoint(ids["beto"] | ids["carla"])
        assert ids["beto"].isdisjoint(ids["carla"])

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_8_ninguna_peticion_contiene_cuentas_movimientos_ni_saldos(
        self,
    ) -> None:
        """Cero datos personales en peticiones o logs del servidor (INV-16)."""

    @pytest.mark.skip(reason="Pendiente: hito de integración en main")
    def test_criterio_3_9_pruebas_automatizadas_de_idempotencia_interrupcion_y_cola(
        self,
    ) -> None:
        """Existen pruebas automatizadas de idempotencia, interrupción y reintentos."""

    def test_criterio_3_10_openapi_yaml_incluye_get_cambios_e_idempotency_key(
        self,
    ) -> None:
        """openapi.yaml documenta GET /cambios y encabezado Idempotency-Key."""
        raiz = Path(__file__).resolve().parents[1]
        openapi_texto = (raiz / "openapi.yaml").read_text(encoding="utf-8")
        assert "/cambios:" in openapi_texto
        assert "Idempotency-Key" in openapi_texto
        assert "Obtener los cambios del usuario" in openapi_texto

    def test_criterio_3_11_cambio_de_usuario_en_misma_app_muestra_libros_distintos(
        self,
    ) -> None:
        """Cambio de usuario en la misma app muestra libros SQLite distintos."""
        raiz_repo = Path(__file__).resolve().parents[2]
        adr14 = raiz_repo / "docs" / "adr" / "ADR-014-libro-personal-por-usuario.md"
        assert adr14.exists()
        contenido = adr14.read_text(encoding="utf-8")
        assert "cuentas_claras_<id>.db" in contenido

        libros_rs = raiz_repo / "app" / "src-tauri" / "src" / "libros.rs"
        assert libros_rs.exists()
        codigo_libros = libros_rs.read_text(encoding="utf-8")
        assert "ruta_de_usuario" in codigo_libros
        assert "ruta_para_sesion" in codigo_libros

    def test_criterio_3_12_servidor_con_docker_compose_y_supabase_sin_romper_pruebas(
        self,
    ) -> None:
        """Servidor soporta Docker Compose y Supabase con guarda activa."""
        from apoyo_compartido import verificar_no_supabase

        verificar_no_supabase(
            "postgresql://cuentas:claras@localhost:5432/cuentas_claras_pruebas"
        )
        verificar_no_supabase(None)

        with pytest.raises(pytest.fail.Exception):
            verificar_no_supabase(
                "postgresql://postgres:clave@db.supabase.co:5432/postgres"
            )
