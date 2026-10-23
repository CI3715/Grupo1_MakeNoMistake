# Diseño de persistencia: eventos nuevos y proyecciones (Entrega 3)

**Estado:** borrador para revisión
**Fecha:** 2026-10-19
**Rama:** `base_de_datos`

---

## 1. Objetivo

Extender el modelo de persistencia de Cuentas Claras para soportar los nuevos
eventos del libro (`MovimientoRegistrado`, `MovimientoCorregido`,
`CategoriaCreada`) y las proyecciones que derivan de ellos, sin tocar el esquema
de la tabla `eventos` y sin perder nada al abrir una base creada con la
Entrega 2.

Este documento cubre:

1. Persistencia de los nuevos eventos sin cambiar el esquema de `eventos`.
2. Proyecciones `movimientos`, `categorias` y `cuentas` con el **saldo inicial
   separado del saldo actual** (D-11).
3. Apertura de una base de la Entrega 2 sin pérdida de cuentas.
4. Reconstrucción completa de las tres proyecciones desde los eventos.

---

## 2. Decisiones de diseño

### 2.1 El esquema de `eventos` no cambia

La tabla `eventos` sigue siendo exactamente la misma:

| columna | tipo | nota |
|---|---|---|
| `id` | TEXT PK | UUID v4 |
| `tipo` | TEXT | etiqueta de texto libre |
| `version` | INTEGER | `DEFAULT 1` |
| `momento_registro` | TEXT | ISO 8601 UTC |
| `dispositivo_origen` | TEXT | UUID del dispositivo |
| `datos` | TEXT | carga JSON del evento |
| `registrado_en` | TEXT | `DEFAULT` con `strftime` |

**No se agregó ninguna columna ni índice.** `tipo` es simplemente texto: los
valores nuevos (`CategoriaCreada`, `MovimientoRegistrado`,
`MovimientoCorregido`) conviven con `CuentaCreada` sin migración alguna. Solo
existe un índice `idx_eventos_tipo` para filtrar, que ya existía.

Los triggers `eventos_no_update` y `eventos_no_delete` siguen activos: el libro
es inmutable en cualquier tipo de evento (INV-13).

### 2.2 Tipos de evento nuevos y su JSON

Se centralizan como constantes para no dispersar literales:

- `aplicacion::proyector`: `TIPO_CATEGORIA_CREADA`, `TIPO_MOVIMIENTO_REGISTRADO`,
  `TIPO_MOVIMIENTO_CORREGIDO`
- `infraestructura::db`: los mismos tres nombres, usados por la siembra

La decodificación vive en `aplicacion::codec` (JSON → structs de dominio), y la
traducción a filas de proyección en `aplicacion::proyector`. Ambos adaptadores
(en memoria y SQLite) usan exactamente el mismo código.

#### `CategoriaCreada`

```json
{
  "id": "cat-fabrica-egreso-alimentacion",
  "nombre": "Alimentación",
  "tipo": "egreso",
  "es_fabrica": true
}
```

- `tipo` ∈ `{"ingreso", "egreso"}` (validado).
- `es_fabrica` es opcional; si no está presente se asume `false` (categoría
  creada por quien usa la aplicación).

#### `MovimientoRegistrado`

```json
{
  "id": "m1",
  "cuenta_id": "c1",
  "tipo": "ingreso",
  "monto": "40.00",
  "fecha": "2026-10-14",
  "categoria_id": "cat-fabrica-ingreso-salario",
  "nota": "pago quincenal"
}
```

- `monto` es **texto decimal positivo** con la escala de la moneda de la cuenta.
  Se valida que sea > 0; el signo lo aporta `tipo`.
- No existe `moneda` en el movimiento: se hereda de la cuenta (evita duplicar
  un dato y su error).
- `nota` es opcional.

#### `MovimientoCorregido`

```json
{
  "id": "m2",
  "movimiento_original_id": "m1",
  "cuenta_id": "c1",
  "tipo": "ingreso",
  "monto": "5.00",
  "fecha": "2026-10-14",
  "categoria_id": "cat-fabrica-ingreso-salario"
}
```

- Mismos campos que un registro, más `movimiento_original_id` (obligatorio).
- **Nunca borra ni sobrescribe el original**: es una fila nueva que lo
  referencia. El original queda con `estado = 'corregido'` y
  `corregido_por_evento` apuntando a este evento.

### 2.3 Proyecciones

Las proyecciones son **caché derivada** de `eventos`: se pueden borrar y
reconstruir enteras. Nunca son la fuente de verdad.

#### `cuentas` — saldo inicial separado del saldo actual (D-11)

| columna | tipo | nota |
|---|---|---|
| `saldo_inicial` | TEXT | **inmutable**: el saldo con el que se creó la cuenta |
| `saldo_actual` | TEXT | `saldo_inicial` + efecto de los movimientos vigentes |

Antes ambas columnas tenían el mismo valor. Ahora `saldo_inicial` se congela en
la creación y `saldo_actual` se recalcula cada vez que cambia un movimiento. El
cálculo se hace con aritmética decimal exacta en Rust (`rust_decimal`,
`aplicacion::proyector::calcular_saldo_actual`), nunca con `REAL`/float (INV-03).

#### `categorias`

| columna | tipo | restricción |
|---|---|---|
| `id` | TEXT PK | |
| `nombre` | TEXT | |
| `tipo` | TEXT | `CHECK (tipo IN ('ingreso','egreso'))` |
| `es_fabrica` | INTEGER | `CHECK (0,1)` |
| `estado` | TEXT | `CHECK ('activa','inactiva')` |
| `evento_origen` | TEXT | FK → `eventos(id)` |
| `actualizado_en` | TEXT | |

Unicidad: índice único `ON categorias (tipo, lower(nombre))`, de modo que
`Otros gastos` (egreso) y `Otros ingresos` (ingreso) conviven sin conflicto.

#### `movimientos`

| columna | tipo | restricción |
|---|---|---|
| `id` | TEXT PK | |
| `cuenta_id` | TEXT | FK → `cuentas(id)` |
| `tipo` | TEXT | `CHECK ('ingreso','egreso')` |
| `monto` | TEXT | `CHECK (CAST(monto AS REAL) > 0)` |
| `fecha` | TEXT | `CHECK` con `GLOB` para `AAAA-MM-DD` |
| `categoria_id` | TEXT | FK → `categorias(id)` |
| `nota` | TEXT | opcional |
| `estado` | TEXT | `CHECK ('vigente','corregido')` |
| `reemplaza_a` | TEXT | FK → `movimientos(id)`, solo correcciones |
| `corregido_por_evento` | TEXT | FK → `eventos(id)` |
| `evento_origen` | TEXT | FK → `eventos(id)` |
| `actualizado_en` | TEXT | |

`reemplaza_a` tiene un índice único parcial (`WHERE reemplaza_a IS NOT NULL`):
cada movimiento original se corrige **una sola vez**. El `CHECK (CAST(monto AS
REAL) > 0)` solo se usa para validar el texto; los cálculos de saldo nunca pasan
por `REAL`.

#### Vista `vista_composicion_saldo`

Devuelve una fila `tipo = 'inicial'` con el `saldo_inicial` de la cuenta más una
fila por cada movimiento **vigente** (los corregidos no aparecen). Sirve para
componer y auditar el saldo sin reimplementar la suma en la interfaz.

### 2.4 El adaptador: `guardar` es idempotente y atómico

`infraestructura::repositorios::proyectar_evento` aplica, para un evento, sus
efectos sobre las tres proyecciones. `RepositorioEventosSqlite::guardar`:

1. Abre **una transacción**.
2. Inserta el evento; si `id` ya existía → `ResultadoGuardado::YaExistia` y la
   transacción termina sin tocar proyecciones (INV-13).
3. Aplica `proyectar_evento` sobre la misma transacción.
4. Confirma.

O se persiste el evento y sus efectos, o ninguno. El adaptador en memoria
(`RepositorioEnMemoria::aplicar`) replica el mismo orden de efectos, y el mismo
cuadro de mando se comprueba en ambos con `aplicacion::contrato`.

Al proyectar un `MovimientoCorregido`:

1. Se marca el original (`estado = 'corregido'`, `corregido_por_evento`).
2. Se inserta la fila nueva vigente con `reemplaza_a`.
3. Se recalcula `saldo_actual` de la cuenta afectada.

### 2.5 Cálculo del saldo actual (D-11)

```
saldo_actual = saldo_inicial
             + Σ montos de movimientos vigentes con tipo = ingreso
             − Σ montos de movimientos vigentes con tipo = egreso
```

- Aritmética decimal exacta (`rust_decimal`).
- Se formatea con los decimales de la moneda de la cuenta (`VES/USD/EUR/USDT`
  tienen 2).
- `saldo_inicial` **jamás** se modifica fuera de la creación de la cuenta.
- Solo cuentan los movimientos **vigentes**: cuando un movimiento se corrige, el
  original sale de la cuenta y entra la corrección.

La misma función (`proyector::calcular_saldo_actual`) la usan el adaptador en
memoria y el de SQLite, por lo que ambos producen exactamente el mismo string.

### 2.6 Inmutabilidad y correcciones

Triggers sobre `eventos`:

- `eventos_no_update` / `eventos_no_delete`: el libro no se modifica ni se borra.

Triggers sobre `movimientos` (operación normal):

- `movimientos_no_delete`: no se borran; se corrigen.
- `movimientos_campos_inmutables`: solo cambian `estado`,
  `corregido_por_evento` y `actualizado_en`; `id`, `cuenta_id`, `tipo`, `monto`,
  `fecha`, `categoria_id`, `nota`, `reemplaza_a` y `evento_origen` quedan
  congelados.
- `movimientos_corregido_no_vuelve`: un movimiento corregido nunca vuelve a
  `vigente`.

Triggers sobre `categorias`:

- `categorias_fabrica_no_delete`: las 11 categorías de fábrica no se borran, se
  desactivan (`estado = 'inactiva'`).

Los dos triggers que protegen las proyecciones contra el borrado
(`movimientos_no_delete` y `categorias_fabrica_no_delete`) se crean desde
`db::proteger_proyecciones`, separados del bloque principal de `db::inicializar`
precisamente para que la reconstrucción pueda retirarlos y volver a crearlos
(ver 2.7).

### 2.7 Reconstrucción de las tres proyecciones

`infraestructura::repositorios::reconstruir_proyecciones(conn)`:

1. `desproteger_proyecciones` → retira `movimientos_no_delete` y
   `categorias_fabrica_no_delete` **dentro de la transacción** (son los dos que
   impedirían limpiar las proyecciones).
2. Borra `movimientos`, `categorias` y `cuentas`.
3. Lee `eventos` en estricto orden de inserción (`rowid`) y vuelve a aplicar
   `proyectar_evento` a cada uno.
4. `proteger_proyecciones` → recrea los dos triggers, en la misma transacción.
5. Confirma.

Si algo falla, la transacción se revierte y los triggers quedan como estaban.

**Por qué se retiran y se vuelven a crear:** las proyecciones son caché y deben
poder reconstruirse desde cero; los triggers solo protegen la operación normal.
Los triggers `movimientos_campos_inmutables` y `movimientos_corregido_no_vuelve`
**no** se retiran: la reconstrucción solo hace `INSERT` de movimientos y un
`UPDATE` de estado, que ambos permiten.

El resultado es determinista: aplicar todos los eventos en orden da siempre las
mismas filas. Por eso `reconstruir` deja idénticas las tres proyecciones a las
que produce `guardar`, lo que comprueba la batería de contrato
(`aplicacion::contrato::bateria_contrato_repositorio`), ejecutada tanto contra
`RepositorioEnMemoria` como contra `RepositorioEventosSqlite`.

El adaptador en memoria (`RepositorioEnMemoria::reconstruir`) limpia los tres
vectores y vuelve a aplicar los mismos eventos con la misma función `aplicar`.

### 2.8 Apertura sin pérdida de una base de la Entrega 2

Una base de la Entrega 2 tiene `eventos` (con `CuentaCreada`), `cuentas` y
`configuracion`, pero **no** `categorias` ni `movimientos`.

Al abrir (`app/src-tauri/src/lib.rs`):

1. `BaseDatos::persistente` → `abrir` (WAL + `foreign_keys = ON`).
2. `db::inicializar` → crea con `IF NOT EXISTS` las tablas que falten
   (`categorias`, `movimientos`), índices, triggers y la vista; no toca las
   existentes.
3. `db::sembrar_categorias_fabrica` → inserta los 11 eventos `CategoriaCreada`
   con sus IDs deterministas; es idempotente (`INSERT OR IGNORE`).
4. `repositorio.reconstruir()` → reconstruye `cuentas`, `categorias` y
   `movimientos` desde `eventos`.

Las cuentas de la Entrega 2 **no se pierden**: se descartan las filas de la
proyección y se regeneran desde sus propios eventos `CuentaCreada`
(`saldo_inicial = saldo_actual` en ese punto, porque todavía no hay movimientos).

La reconstrucción se ejecuta en **cada apertura**, así que cualquier
desincronización de la caché se corrige sola y el libro de eventos nunca se
modifica. Es el mecanismo que da confianza de "abrir sin pérdida".

### 2.9 Categorías de fábrica

- 7 de egreso: Alimentación, Transporte, Servicios, Salud, Educación,
  Entretenimiento, Otros gastos.
- 4 de ingreso: Salario, Ventas, Regalos, Otros ingresos.
- Se siembran como **eventos** (no solo como filas) con IDs deterministas
  `cat-fabrica-<tipo>-<slug>` para que `reconstruir` reproduzca exactamente el
  mismo catálogo.
- Idempotentes: se pueden insertar en cada arranque sin duplicar.
- No se pueden borrar; se desactivan.
- `BaseDatos::persistente` las siembra; `db::inicializar` solo crea el esquema
  para no alterar los tests de contrato que trabajan en memoria.

---

## 3. Invariantes que refuerza este diseño

| Invariante | Cómo se cumple |
|---|---|
| INV-13 (eventos inmutables, `id` idempotente) | triggers `eventos_no_update/_delete`; `guardar` devuelve `YaExistia` sin tocar proyecciones |
| INV-03 (dinero sin floats) | montos y saldos en TEXT + `rust_decimal`; el `CHECK REAL` es solo validación de texto |
| D-11 (saldo inicial ≠ saldo actual) | columnas separadas; `saldo_inicial` inmutable; `saldo_actual` recalculado desde movimientos vigentes |
| Correcciones nunca borran | trigger `movimientos_no_delete` + índice único parcial en `reemplaza_a` + trigger `movimientos_corregido_no_vuelve` |
| Atomicidad evento + proyecciones | una sola transacción en `guardar` y en `reconstruir_proyecciones` |
| Paridad de adaptadores | misma lógica en `aplicacion::proyector` + misma batería de contrato en ambos |
| Apertura sin pérdida | `reconstruir()` en cada apertura; test `abrir_una_base_de_entrega_2_reconstruye_sin_perder_cuentas` |

---

## 4. Estrategia de pruebas y estado actual

Estado: **implementado y verde.**

| suite | qué comprueba | total |
|---|---|---|
| `aplicacion` (unit + contrato) | batería de contrato de `RepositorioEventos` (10 secciones: orden, idempotencia, tipos desconocidos, categorías, movimientos, saldo D-11, corrección, reconstrucción) y casos de uso | 14 |
| `infraestructura::db` | esquema, unicidades, FKs/CHECKs, inmutabilidad, corrección, vista, siembra idempotente, persistencia en archivo | 22 |
| `infraestructura::repositorios` | contrato de SQLite, reconstrucción de las tres proyecciones, corrección + saldo, saldo inicial separado, **apertura de base Entrega 2** | 5 |
| resto del workspace | dominio, servidor, arquitectura | verde |

Comandos de verificación (desde `app/src-tauri`):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets   # sin warnings
rustfmt --edition 2024 <archivos tocados>
```

---

## 5. Pendientes / fuera de alcance de este cambio

- **Comandos Tauri y DTOs** para crear movimientos, corregirlos, crear
  categorías y listar movimientos: no se tocaron, para no arrastrar cambios al
  frontend. `CuentaListada` tampoco expone `saldo_inicial` todavía.
- **Vista UI** del saldo inicial / composición del saldo (`vista_composicion_saldo`
  ya está disponible en la base).
- **Reconstrucción manual** desde la interfaz (el botón / comando): el hook de
  apertura ya llama a `reconstruir()`.
- **Fecha y montos**: la validación de "fecha futura" pertenece a la capa de
  dominio/casos de uso, no a la persistencia.

---

## 6. Archivos tocados

| archivo | cambios |
|---|---|
| `crates/infraestructura/src/db.rs` | tablas `categorias`/`movimientos`, índices, triggers, vista, `proteger/desproteger_proyecciones`, CRUD de proyecciones, `sembrar_categorias_fabrica`, siembra en `persistente` |
| `crates/infraestructura/src/repositorios.rs` | `proyectar_evento`, `recalcular_saldo`, `reconstruir_proyecciones`, `listar_categorias`/`listar_movimientos`, tests |
| `crates/aplicacion/src/puertos.rs` | `CuentaProyectada.saldo_inicial`, `CategoriaProyectada`, `MovimientoProyectado`, métodos `listar_categorias`/`listar_movimientos` |
| `crates/aplicacion/src/codec.rs` | `deserializar_categoria`, `deserializar_movimiento` |
| `crates/aplicacion/src/proyector.rs` | `proyectar_categoria`, `proyectar_movimiento`, `calcular_saldo_actual`, constantes de tipo |
| `crates/aplicacion/src/memoria.rs` | proyecciones de categorías/movimientos y `reconstruir` |
| `crates/aplicacion/src/contrato.rs` | batería extendida (10 secciones) |
| `app/src-tauri/src/lib.rs` | `repositorio.reconstruir()` al abrir |
