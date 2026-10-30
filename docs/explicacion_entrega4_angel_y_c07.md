# Informe Técnico Explicativo: Entrega 4 — Cuentas Claras
**Autor:** Ángel Pacheco (Responsable de Aplicación y Casos de Uso)  
**Fecha:** 29 de octubre de 2026  
**Proyecto:** Cuentas Claras (CI3715 - Ingeniería de Software 1)

---

## 1. Resumen Ejecutivo del Trabajo Realizado por Ángel

En esta Entrega 4, mi responsabilidad principal comprendió el diseño, implementación y verificación de la **capa de aplicación**, los **casos de uso de negocio**, la **retrocompatibilidad del libro de eventos (C-07)** y la **batería completa de pruebas automatizadas en memoria**.

Todo el desarrollo se rige bajo la regla fundamental de la entrega:
> **INV-02 (Regla de Oro):** *Nunca se suman montos de monedas distintas sin una tasa explícita con fecha. Si falta una tasa, el sistema jamás inventa una conversión ni muestra un total incorrecto: expone los subtotales por moneda y declara exactamente qué par de tasas falta (RF-CAM-04).*

---

## 2. Detalle de Características y Casos de Uso Desarrollados

### 2.1. Casos de Uso de Tasas de Cambio y Moneda Base
Ubicados en `app/src-tauri/crates/aplicacion/src/casos_de_uso.rs`:

1. **`registrar_tasa(NuevaTasa)`**:
   - Valida que la moneda de origen y destino pertenezcan al catálogo (de fábrica o propias) y sean distintas.
   - Valida que el valor sea mayor que cero y admita hasta 8 decimales.
   - Rechaza fechas futuras respecto a la fecha del sistema (`reloj.hoy()`).
   - Requiere fuente no vacía.
   - Registra el evento inmutable `TasaRegistrada` con su id único, timestamp UTC y dispositivo.
2. **`eliminar_tasa(id)`**:
   - Comprueba que la tasa exista y esté activa.
   - En cumplimiento estricto del patrón Event Sourcing (el libro de eventos solo admite `INSERT`), **no realiza un `DELETE`**, sino que emite un nuevo evento inmutable `TasaEliminada`.
   - La proyección excluye la tasa eliminada para cálculos posteriores.
3. **`listar_tasas()`**:
   - Devuelve el historial completo de tasas vigentes y eliminadas agrupadas por par, ordenadas de la más reciente a la más antigua.
4. **`cambiar_moneda_base(nueva_base)`**:
   - Valida que la nueva moneda base exista en el catálogo de monedas.
   - Emite el evento `MonedaBaseCambiada`.
   - **Garantía RF-CAM-02:** Cambiar la moneda base jamás altera ningún movimiento ni saldo de ninguna cuenta; únicamente define la unidad en la que se presenta el patrimonio consolidado.
5. **`obtener_moneda_base()`**:
   - Devuelve la moneda base activa (por defecto `"USD"`).

### 2.2. Gestión de Monedas Propias
- **`crear_moneda(NuevaMoneda)`**:
  - Permite al usuario registrar monedas personalizadas con código de 3 a 5 letras mayúsculas, nombre, símbolo y entre 0 y 8 decimales.
  - Verifica unicidad contra las monedas de fábrica (VES, USD, EUR, USDT) y las propias preexistentes.
  - Emite el evento `MonedaCreada`.
- **`listar_monedas()`**:
  - Proyecta dinámicamente las 4 monedas inmutables de fábrica más las monedas propias registradas en el libro.

### 2.3. Consulta de Patrimonio Consolidado (`consultar_patrimonio`)
Permite consultar la posición financiera a la fecha actual o a una fecha histórica (viaje en el tiempo):
- **Corte temporal exacto:** Filtra las cuentas existentes a la fecha de consulta (`cl.fecha_creacion <= fecha`) y recalcula sus saldos sumando solo los movimientos y correcciones con fecha igual o menor a la fecha de corte.
- **Subtotales por divisa:** Agrupa los saldos individuales por código de moneda.
- **Selección de tasa vigente y desempate:**
  - Para cada moneda secundaria hacia la base, busca la tasa con fecha igual o anterior más cercana a la fecha de consulta.
  - Si existen múltiples tasas para el mismo par en la misma fecha, desempata eligiendo **la última registrada en el libro**.
- **Tasa Inversa:** Si no existe la tasa directa (ej. `USDT -> VES`), pero existe la inversa (`VES -> USDT`), utiliza la división exacta ($1 \div \text{valor}$).
- **Sin tasas cruzadas:** No triangula a través de terceras monedas.
- **Detección de pares faltantes:** Si falta alguna tasa necesaria, el `total` se devuelve como `None`, se mantienen los subtotales por moneda y se entrega la lista de `faltantes` (`origen` y `destino`).
- **Columna "En VES" / `en_moneda_base` por cuenta (Sección 2.3 del enunciado):**
  - Cada cuenta en `CuentaPatrimonioVista` incluye `pub en_moneda_base: Option<String>`.
  - Si la cuenta está en la moneda base: su saldo se proyecta directamente.
  - Si está en otra divisa y existe tasa vigente: se calcula y formatea su equivalente en la moneda base.
  - Si no existe tasa: viaja como `None`.
- **Redondeo Único Final (INV-03):** La consolidación acumula con precisión decimal exacta completa (`rust_decimal::Decimal`), y el redondeo a los decimales de la moneda base ocurre una sola vez al final.

### 2.4. Corrección C-03 (`monto_vigente` en Movimientos Corregidos)
- Se extendió `proyector.rs` y `vistas.rs` para que `MovimientoListado` y `DetalleCuenta` expongan el `monto_vigente` recalculado.
- Permite que el diálogo de edición y corrección de movimientos presente siempre el valor efectivo actual del movimiento.

---

## 3. Justificación y Arquitectura del Cambio C-07 (De Texto a Numérico)

### 3.1. ¿Por qué se hizo el cambio C-07?
El profesor observó que guardar montos financieros como cadenas de texto (`VARCHAR` / `"50.00"`) en la base de datos y eventos no es la práctica idónea en sistemas contables de misión crítica.

Sin embargo, existía una restricción inquebrantable de la cátedra:
> **INV-03:** *Prohibido el uso de tipos de coma flotante binaria (`f32`, `f64`, `float`, `double`, `REAL`) para representar dinero.*

### 3.2. ¿Por qué NO se pueden usar tipos flotantes (`f32`/`f64`/`REAL`)?
El estándar IEEE 754 de coma flotante binaria almacena números en potencias de 2. Fracciones decimales comunes como `0.1` ($1/10$) o `0.01` ($1/100$, un centavo) se convierten en números periódicos infinitos en base binaria:
$$0.1_{10} = 0.0001100110011..._2$$

Esto ocasiona que operaciones aritméticas elementales acumulen errores de precisión (ej. `0.1 + 0.2 = 0.30000000000000004`). En libros contables de partidas múltiples, este "goteo" produce balances descuadrados y deriva financiera inaceptable.

### 3.3. La Solución Arquitectónica Adoptada: Enteros en Unidad Mínima
Para satisfacer tanto la observación del profesor como la invariante INV-03:

1. **Montos en Unidades Mínimas (`i64` / `INTEGER`):**
   El dinero se almacena en eventos y en columnas SQLite como un **entero con signo de 64 bits** escalado a la menor unidad divisible de cada divisa:
   - Divisas con 2 decimales (USD, EUR, VES): se guardan en **centavos** ($50.00 \to 5000$).
   - Divisas con 0 decimales (CLP, JPY): se guardan en **unidades enteras** ($1000 \to 1000$).
   - Divisas con 8 decimales (BTC, Satoshi): se guardan en su **unidad atómica** ($0.00000001 \to 1$).
2. **Tasas de Cambio:**
   Las tasas admiten hasta 8 decimales y se almacenan como enteros multiplicados por un factor fijo de $10^8$ (escala fija `100_000_000`).
3. **Aritmética Comprobada (*Checked Math*):**
   En `unidades.rs`, toda conversión entre decimal exacto (`rust_decimal`) y entero en unidad menor cuenta con validaciones estrictas de rango `i64::MIN..=i64::MAX`. Si un monto excede la capacidad de 64 bits, falla con error explícito sin provocar pánicos en tiempo de ejecución.
4. **Validación de Escala:**
   Cualquier monto que intente ingresar con más decimales de los configurados para su moneda se rechaza en la capa de dominio.

### 3.4. Retrocompatibilidad e Inmutabilidad del Libro de Eventos
Por definición de *Event Sourcing*, los eventos registrados en las Entregas 2 y 3 dentro del archivo SQLite `cuentas_claras.db` son **inmutables** y no pueden ser reescritos ni migrados destructivamente. Dichos eventos antiguos guardaban montos como `"50.00"` (texto).

En `codec.rs`:
- **Escritura:** Todo nuevo evento emite montos serializados como enteros (`unidades_menores: i64`).
- **Lectura Dual:** Al deserializar eventos históricos para proyectar el estado contable, el decodificador inspecciona el campo JSON:
  - Si viene como número entero: lo lee directamente.
  - Si viene como string: lo parsea y convierte con el catálogo de monedas.
- Esto garantiza que una base de datos de las Entregas 2 o 3 abra de forma idéntica, sin pérdida de datos ni discrepancia de saldos.

### 3.5. ¿Por qué el DTO hacia la Interfaz Web sigue viajando como Texto?
En el límite entre Tauri (Rust) y el Frontend (Next.js / TypeScript):
- En JavaScript, el tipo primitivo `number` es obligatoriamente un flotante IEEE 754 de doble precisión (`f64`). Si enviáramos enteros grandes de 64 bits en JSON, JavaScript truncaría los valores por encima de $2^{53} - 1$ (`Number.MAX_SAFE_INTEGER = 9,007,199,254,740,991`).
- La interfaz de usuario es una capa de **presentación**: no realiza sumas, conversiones ni cálculos de saldos.
- Al enviar los montos formateados como texto (`"88.700,00"`, `"1.000,00"`), se preserva la exactitud tipográfica, la separación de miles/decimales y la inmutabilidad de los datos visualizados.

---

## 4. Estado de Verificación y Pruebas

Toda la implementación fue verificada mediante la suite completa del proyecto:

| Conjunto de Pruebas | Alcance | Resultado |
|---|---|---|
| `dominio` | Reglas puras, C-07 ida y vuelta, tasas, redondeo | **77 pasadas, 0 fallos** |
| `aplicacion` (unitarias) | Codec dual, unidades, proyector | **15 pasadas, 0 fallos** |
| `aplicacion` (integración) | Ejemplo 2.3, desempate, eliminación, moneda propia, C-03 | **20 pasadas, 0 fallos** |
| `app_lib` | Comandos Tauri, DTOs, serialización | **42 pasadas, 0 fallos** |
| `infraestructura` | Persistencia SQLite, triggers inmutabilidad, C-07 SQL | **37 pasadas, 0 fallos** |
| `arquitectura` | Prohibición de flotantes, no red en comandos | **28 pasadas, 0 fallos** |
| `categorias_sqlite_real` | C-02 prueba con base SQLite temporal | **2 pasadas, 0 fallos** |
| **Total Rust Workspace** | **Suite completa de backend** | **195 pasadas, 0 fallos (100%)** |
| `npx tsc --noEmit` & `npm run build` | Compilación e integridad de TypeScript en UI | **0 errores** |
| `cargo clippy -- -D warnings` | Linter estricto de Rust | **0 advertencias / 0 errores** |
| `cargo audit` & `npm audit` | Auditoría de seguridad de dependencias | **0 vulnerabilidades** |
| `uv run pytest` (servidor) | Pruebas del backend Python | **10 pasadas en 8s** |

