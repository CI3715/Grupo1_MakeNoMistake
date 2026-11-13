"""Pruebas del algoritmo de división de gastos, redondeo y balances (Entrega 6).

Cubre los requisitos RF-COM-05, INV-07, INV-08, RNF-08 y ESC-10:
- División en partes iguales con residuo asignado en orden alfabético de usuario.
- División por porcentajes que deben sumar 100 (con mensaje de diferencia exacta).
- División por montos fijos que deben sumar el total (con mensaje de diferencia).
- Suma exacta de balances igual a 0.00 tras cada gasto registrado.
- Soporte para montos grandes sin coma flotante.
"""

from decimal import Decimal

import pytest
from hypothesis import given
from hypothesis import strategies as st

from division import (
    ErrorValidacionDivision,
    a_centesimas,
    calcular_balances_grupo,
    dividir,
)


class TestDivisionPartesIguales:
    """Pruebas para la regla de división en partes iguales (sección 2.3 y 2.4)."""

    def test_tres_participantes_cien_unidades_residuo_alfabetico(self) -> None:
        """100,00 entre ana, beto y carla asigna 33,34 a ana y 33,33 a los demás."""
        div = dividir("100.00", "partes_iguales", ["ana", "beto", "carla"])
        assert div.residuo == "0.01"
        assert len(div.partes) == 3
        # ana recibe el céntimo adicional por precedencia alfabética
        assert div.partes[0].usuario == "ana"
        assert div.partes[0].monto == "33.34"
        assert div.partes[0].incluye_redondeo is True

        assert div.partes[1].usuario == "beto"
        assert div.partes[1].monto == "33.33"
        assert div.partes[1].incluye_redondeo is False

        assert div.partes[2].usuario == "carla"
        assert div.partes[2].monto == "33.33"
        assert div.partes[2].incluye_redondeo is False

        suma = sum(a_centesimas(p.monto) or 0 for p in div.partes)
        assert suma == 10000

    def test_un_solo_participante_recibe_monto_completo_sin_residuo(self) -> None:
        """Gasto dividido entre un solo miembro asigna el 100% sin residuo."""
        div = dividir("100.00", "partes_iguales", ["ana"])
        assert div.residuo == "0.00"
        assert len(div.partes) == 1
        assert div.partes[0].usuario == "ana"
        assert div.partes[0].monto == "100.00"
        assert div.partes[0].incluye_redondeo is False

    def test_exclusion_de_miembro_no_participante(self) -> None:
        """Excluir a un miembro calcula la división solo entre los seleccionados."""
        div = dividir("100.00", "partes_iguales", ["beto", "carla"])
        assert len(div.partes) == 2
        assert div.partes[0].usuario == "beto"
        assert div.partes[0].monto == "50.00"
        assert div.partes[1].usuario == "carla"
        assert div.partes[1].monto == "50.00"

    def test_monto_grande_no_pierde_precision_ni_desborda(self) -> None:
        """Montos de hasta 9.999.999.999,99 mantienen aritmética decimal exacta."""
        div = dividir("9999999999.99", "partes_iguales", ["ana", "beto", "carla"])
        assert len(div.partes) == 3
        assert div.residuo == "0.00"
        assert div.partes[0].monto == "3333333333.33"
        assert div.partes[1].monto == "3333333333.33"
        assert div.partes[2].monto == "3333333333.33"
        suma = sum(a_centesimas(p.monto) or 0 for p in div.partes)
        assert suma == 999999999999


class TestDivisionPorcentajes:
    """Pruebas para la regla de división por porcentajes (sección 2.3)."""

    def test_porcentajes_que_suman_noventa_y_cinco_indica_faltan_cinco(self) -> None:
        """Porcentajes que suman 95 se rechazan indicando exactamente 'faltan 5'."""
        with pytest.raises(ErrorValidacionDivision) as exc_info:
            dividir(
                "100.00",
                "porcentajes",
                [("ana", "50"), ("beto", "25"), ("carla", "20")],
            )
        error = exc_info.value.errores[0]
        assert error.campo == "porcentajes"
        assert "Los porcentajes suman 95; faltan 5." in error.mensaje
        assert error.no_cuadra is True

    def test_porcentajes_que_suman_ciento_cinco_indica_sobran_cinco(self) -> None:
        """Porcentajes que suman 105 se rechazan indicando exactamente 'sobran 5'."""
        with pytest.raises(ErrorValidacionDivision) as exc_info:
            dividir(
                "100.00",
                "porcentajes",
                [("ana", "50"), ("beto", "30"), ("carla", "25")],
            )
        error = exc_info.value.errores[0]
        assert error.campo == "porcentajes"
        assert "Los porcentajes suman 105; sobran 5." in error.mensaje
        assert error.no_cuadra is True

    def test_porcentajes_validos_cien_por_ciento_calcula_partes_exactas(self) -> None:
        """50% / 25% / 25% sobre 100,00 produce 50,00, 25,00 y 25,00."""
        div = dividir(
            "100.00",
            "porcentajes",
            [("ana", "50"), ("beto", "25"), ("carla", "25")],
        )
        assert [p.monto for p in div.partes] == ["50.00", "25.00", "25.00"]
        assert all(not p.incluye_redondeo for p in div.partes)
        suma = sum(a_centesimas(p.monto) or 0 for p in div.partes)
        assert suma == 10000


class TestDivisionMontosFijos:
    """Pruebas para la regla de división por montos fijos (sección 2.3)."""

    def test_montos_fijos_que_no_suman_total_indica_diferencia_exacta(self) -> None:
        """Si los montos suman 118,00 y el total es 120,00 indica 'faltan 2,00'."""
        with pytest.raises(ErrorValidacionDivision) as exc_info:
            dividir(
                "120.00",
                "montos_fijos",
                [("ana", "60.00"), ("beto", "38.00"), ("carla", "20.00")],
            )
        error = exc_info.value.errores[0]
        assert error.campo == "montos"
        assert "Los montos suman 118,00; faltan 2,00." in error.mensaje
        assert error.no_cuadra is True

    def test_montos_fijos_que_exceden_total_indica_sobrante_exacto(self) -> None:
        """Si los montos suman 102,00 y el total es 100,00 indica 'sobran 2,00'."""
        with pytest.raises(ErrorValidacionDivision) as exc_info:
            dividir(
                "100.00",
                "montos_fijos",
                [("ana", "50.00"), ("beto", "32.00"), ("carla", "20.00")],
            )
        error = exc_info.value.errores[0]
        assert error.campo == "montos"
        assert "Los montos suman 102,00; sobran 2,00." in error.mensaje
        assert error.no_cuadra is True

    def test_montos_fijos_exactos_registra_correctamente(self) -> None:
        """Montos que suman exactamente el total se aceptan sin error."""
        div = dividir(
            "100.00",
            "montos_fijos",
            [("ana", "50.00"), ("beto", "30.00"), ("carla", "20.00")],
        )
        assert [p.monto for p in div.partes] == ["50.00", "30.00", "20.00"]
        suma = sum(a_centesimas(p.monto) or 0 for p in div.partes)
        assert suma == 10000


class TestBalancesGrupo:
    """Pruebas para el cálculo y balance de los miembros del grupo (sección 2.5)."""

    def test_suma_de_balances_es_exactamente_cero(self) -> None:
        """La suma algebraica de (pago - le_corresponde) es exactamente 0.00."""
        gastos = [
            ("ana", [("ana", "33.34"), ("beto", "33.33"), ("carla", "33.33")]),
            ("beto", [("ana", "20.00"), ("beto", "20.00")]),
            ("carla", [("ana", "10.00"), ("carla", "10.00")]),
        ]
        balances = calcular_balances_grupo(gastos)
        suma_centimos = sum(int(Decimal(b) * Decimal(100)) for b in balances.values())
        assert suma_centimos == 0

    def test_balances_del_flujo_completo_dos_punto_ocho(self) -> None:
        """Verifica que la suma de balances sea cero en cualquier flujo compartido."""
        gastos = [
            ("ana", [("ana", "33.34"), ("beto", "33.33"), ("carla", "33.33")]),
            ("beto", [("ana", "15.00"), ("beto", "15.00"), ("carla", "15.00")]),
        ]
        balances = calcular_balances_grupo(gastos)
        # Ana pagó 100.00 y debe (33.34 + 15.00 = 48.34) -> balance +51.66
        # Beto pagó 45.00 y debe (33.33 + 15.00 = 48.33) -> balance -3.33
        # Carla pagó 0.00 y debe (33.33 + 15.00 = 48.33) -> balance -48.33
        # Suma: 51.66 - 3.33 - 48.33 = 0.00
        assert balances["ana"] == "51.66"
        assert balances["beto"] == "-3.33"
        assert balances["carla"] == "-48.33"

        suma = sum(int(Decimal(b) * Decimal(100)) for b in balances.values())
        assert suma == 0


class TestPropiedadesDivisionHypothesis:
    """Pruebas con hypothesis para división y balances (RNF-08, INV-07, INV-08)."""

    @given(
        centimos=st.integers(min_value=1, max_value=1_000_000_000),
        participantes=st.lists(
            st.from_regex(r"[a-z]{3,10}", fullmatch=True),
            min_size=1,
            max_size=12,
            unique=True,
        ),
    )
    def test_propiedad_partes_iguales_conserva_total_y_residuo_alfabetico(
        self, centimos: int, participantes: list[str]
    ) -> None:
        """Para cualquier monto y grupo, la suma de partes coincide con el total."""
        monto_str = f"{centimos // 100}.{centimos % 100:02d}"
        div = dividir(monto_str, "partes_iguales", participantes)

        # 1. Conservación de la suma exacta (INV-07, RNF-08)
        suma_partes = sum(a_centesimas(p.monto) or 0 for p in div.partes)
        assert suma_partes == centimos

        # 2. Cantidad de partes igual a cantidad de participantes
        assert len(div.partes) == len(participantes)

        # 3. Residuo alfabético determinista
        n = len(participantes)
        r = centimos % n
        residuo_esperado = f"{r // 100}.{r % 100:02d}"
        assert div.residuo == residuo_esperado

        # Los participantes ordenados alfabéticamente
        ordenados = sorted(participantes)
        con_redondeo = [p.usuario for p in div.partes if p.incluye_redondeo]
        assert con_redondeo == ordenados[:r]

    @given(
        gastos=st.lists(
            st.tuples(
                st.sampled_from(["ana", "beto", "carla", "dario"]),
                st.integers(min_value=1, max_value=100_000),
                st.lists(
                    st.sampled_from(["ana", "beto", "carla", "dario"]),
                    min_size=1,
                    max_size=4,
                    unique=True,
                ),
            ),
            min_size=1,
            max_size=10,
        )
    )
    def test_propiedad_suma_de_balances_siempre_cero(
        self,
        gastos: list[tuple[str, int, list[str]]],
    ) -> None:
        """La suma algebraica de todos los balances siempre es exactamente 0.00."""
        lista_gastos: list[tuple[str, list[tuple[str, str]]]] = []
        for pagador, centimos, participantes in gastos:
            monto_str = f"{centimos // 100}.{centimos % 100:02d}"
            div = dividir(monto_str, "partes_iguales", participantes)
            lista_gastos.append((pagador, [(p.usuario, p.monto) for p in div.partes]))

        balances = calcular_balances_grupo(lista_gastos)
        suma_centimos = sum(int(Decimal(b) * Decimal(100)) for b in balances.values())
        assert suma_centimos == 0

