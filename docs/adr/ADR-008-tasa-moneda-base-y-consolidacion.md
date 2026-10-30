# ADR-008: Modelado de tasas de cambio, monedas configurables y consolidación con redondeo único

## Contexto

En las Entregas 2 y 3, la aplicación Cuentas Claras soportaba un catálogo estático de 4 monedas de fábrica (VES, USD, EUR y USDT) con 2 decimales fijos, y la invariante INV-02 prohibía sumar montos de monedas distintas en los listados y subtotales.

La Entrega 4 introduce la necesidad de:
1. Permitir al usuario registrar sus propias monedas ("monedas propias", ej. COP, CLP, BTC, divisas o criptoactivos) con un número configurable de decimales (de 0 a 8), dejando de depender exclusivamente de un catálogo fijo.
2. Definir una moneda base de referencia patrimonial y reportes globales (RF-CAM-01 a RF-CAM-04).
3. Registrar tasas de cambio temporales entre monedas para permitir la conversión y consolidación de saldos en la moneda base.
4. Evitar arbitrajes inconsistentes y triangulaciones complejas prohibiendo tasas cruzadas arbitrarias entre monedas secundarias: toda tasa debe definirse directamente respecto a la moneda base.
5. Garantizar la máxima precisión contable y matemática en la consolidación mediante la regla de **redondeo único**: las conversiones intermedias se acumulan con precisión decimal arbitraria (`Decimal`) y el redondeo a los decimales de la moneda base se ejecuta una sola vez sobre el total final acumulado.
6. Cumplir estrictamente la invariante INV-03 (prohibición de coma flotante `f32` y `f64` en el dinero y tasas) y la corrección C-07 (funciones seguras de conversión bidireccional entre `Monto` y enteros en la unidad menor con comprobación de desbordamiento).

## Decisión

Se implementaron en el crate `dominio` (`app/src-tauri/crates/dominio/`) los siguientes modelos y reglas puras, sin dependencias de infraestructura ni red:

1. **Monedas dinámicas y propias**:
   - `Moneda` incluye el indicador `de_fabrica: bool`.
   - Se mantiene `Moneda::catalogo()` con las 4 monedas predefinidas para preservar total compatibilidad con las entregas previas.
   - Se introduce la función de dominio `crear_moneda(...)` y el evento `MonedaCreada`, validando unicidad de código (2 a 6 caracteres alfanuméricos) y nombre sin distinguir mayúsculas, símbolo no vacío y escala entre 0 y 8 decimales.

2. **Unidad menor con verificación de desbordamiento (C-07)**:
   - Se proporcionan `monto_a_unidad_menor` / `unidad_menor_a_monto` (usando `i128` y `i64`).
   - Se valida explícitamente mediante aritmética `checked_*` cualquier intento de desbordamiento en potencias de 10 o conversión de enteros, retornando `ErrorDominio` en el campo `Monto` ante cualquier desbordamiento.

3. **Tasas de cambio e Inversión**:
   - Se modela la entidad `Tasa` y el evento `TasaRegistrada`.
   - Validaciones: `moneda_origen != moneda_destino`, ambas monedas existentes, valor estrictamente positivo (`> 0`) con hasta 8 decimales, fecha válida en formato `AAAA-MM-DD` y no futura respecto a `hoy`.
   - Cálculo de tasa inversa: `tasa.inversa()` calcula $1 / \text{valor}$ con división decimal exacta, rescalando a 8 decimales sin truncar a cero.
   - Búsqueda de tasa vigente: `tasa_vigente(...)` prioriza la tasa directa más reciente en o antes de la fecha de corte; si no existe directa, busca la inversa más reciente y la invierte; para la misma moneda devuelve la tasa identidad (1.0).

4. **Prohibición de tasas cruzadas**:
   - La función `validar_sin_tasas_cruzadas(origen, destino, moneda_base)` exige que al menos una de las dos monedas sea la moneda base. No se admiten registros ni conversiones indirectas entre monedas secundarias.

5. **Consolidación con redondeo único**:
   - La función `consolidar_saldos(saldos, moneda_base, tasas, fecha_corte)` acumula los saldos convertidos en una suma intermedia de alta precisión sin redondear las partidas individuales.
   - El redondeo a los decimales de la `moneda_base` se efectúa una única vez al final del proceso (`round_dp`), devolviendo `ConsolidacionSaldo` con el desglose detallado de cada conversión.

## Alternativas consideradas

- **Triangulación automática con tasas cruzadas**: Generar automáticamente tasas entre pares de monedas secundarias pasando por la moneda base. Se descartó porque introduce incertidumbre contable, desajustes en spreads cambiarios y contradice el requisito explícito de modelar sin tasas cruzadas.
- **Redondear cada saldo convertido antes de sumarlo**: Redondear cada cuenta individualmente al convertirla a la moneda base. Se descartó porque acumula errores de redondeo sistemáticos (centavos perdidos o duplicados), violando las buenas prácticas financieras internacionales.
- **Uso de enteros de 64 bits exclusivamente para unidad menor**: Limitar la unidad menor a `i64`. Se ofreció soporte tanto para `i128` (máxima capacidad para monedas hiperinflacionarias o con muchos decimales) como para `i64` con comprobación obligatoria de desbordamiento (`i64::try_from`), cumpliendo cabalmente la corrección C-07.

## Consecuencias

### Positivas
- Cero deriva por redondeo en reportes consolidados multimoneda.
- Flexibilidad para que la persona usuaria administre cualquier moneda fiat o digital sin recompilar la aplicación.
- Total seguridad numérica frente a overflow y desbordamientos.
- Aislamiento absoluto: el crate `dominio` sigue siendo 100% puro y autónomo, sin dependencias externas y sin tipos de coma flotante.

### Negativas / Deuda a considerar
- La capa de persistencia y aplicación deberá extender sus repositorios para almacenar eventos de tasas y monedas propias en las siguientes fases.

## Estado

Aceptado

## Fecha

2026-10-27
