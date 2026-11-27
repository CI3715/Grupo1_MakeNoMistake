# ADR-011: Datos compartidos del servidor (amistades, grupos y gastos)

## Contexto

Hasta la Entrega 5 el servidor solo guardaba identidad: `usuarios` con el
hash de la contraseña. Todo lo demás vivía en el dispositivo del usuario
(ADR-006) y no viajaba al servidor (INV-16, RES-02).

La Entrega 6 introduce las primeras funciones **compartidas**: amistades,
grupos con miembros y gastos repartidos entre varias personas. Lo compartido
se define justamente porque lo ven varias personas, así que no puede vivir
solo en un dispositivo. Eso obliga a decidir:

1. qué se guarda en el servidor y qué sigue siendo local;
2. cómo se modela una amistad, que existe solo tras la aceptación (INV-10) y
   cuya pareja es única en cualquiera de los dos sentidos (RF-SOC-05);
3. cómo se guarda dinero sin perder ni un céntimo (INV-07, RNF-08);
4. qué pasa con los grupos y gastos ya compartidos cuando una amistad se
   elimina (RF-SOC-06).

Esta ADR decide el **esquema** y las **firmas de los repositorios**. Los
endpoints, el motor de división y las pantallas llegan en cambios
posteriores, y no cambian lo que aquí se decide.

## Decisión

### 1. Qué se guarda

Se guardan cinco tablas nuevas y nada más:

| Tabla | Qué representa |
|---|---|
| `amistades` | Invitaciones y amistades entre dos personas. |
| `grupos` | Grupo con nombre, descripción, moneda y actividad. |
| `miembros` | Quién pertenece a cada grupo, con qué rol y en qué estado. |
| `gastos` | Gasto compartido: concepto, monto, fecha, pagador y regla. |
| `partes_gasto` | Importe exacto de cada participante y marca de residuo. |

El criterio es «lo ve más de una persona». Las cuentas, movimientos, saldos,
tasas y categorías del libro personal **no** se guardan aquí: siguen en la
base local de la aplicación y ninguna operación del libro hace HTTP (INV-16).
La prueba `test_servidor_respeta_inv16_sin_tablas_financieras` fija la lista
de tablas permitidas, de modo que una tabla financiera nueva hace fallar la
batería.

### 2. Las tablas

Todas las marcas de tiempo son `TIMESTAMPTZ` en UTC y las fechas de negocio,
`DATE`. El DDL completo y comentado está en `servidor/esquema.py`.

**`amistades`**

| Columna | Tipo | Uso |
|---|---|---|
| `id` | `UUID` | Clave primaria. |
| `invitador_id` | `UUID` → `usuarios` | Quién invita. |
| `invitado_id` | `UUID` → `usuarios` | Quién recibe. |
| `estado` | `VARCHAR(10)` | `pendiente`, `aceptada`, `rechazada` o `eliminada`. |
| `fecha_invitacion` | `TIMESTAMPTZ` | Cuándo se envió la invitación. |
| `fecha_respuesta` | `TIMESTAMPTZ` | Cuándo aceptó o rechazó el destinatario. |
| `fecha_baja` | `TIMESTAMPTZ` | Cuándo se eliminó una amistad aceptada. |

Restricciones: `invitador_id <> invitado_id`, estados cerrados y coherencia
entre estado y fechas (`pendiente` sin respuesta; `aceptada` y `rechazada`
con respuesta; `eliminada` con respuesta y baja).

La unicidad de la pareja se impone con un índice único sobre
`least(invitador_id, invitado_id)` y `greatest(invitador_id, invitado_id)`,
que ordena la pareja sea cual sea el sentido de la invitación. Es un índice
**parcial**, limitado a `estado IN ('pendiente', 'aceptada')`: la regla de
RF-SOC-05 es que no haya dos relaciones vivas entre las mismas personas, no
que un rechazo o una baja bloqueen para siempre un intento nuevo. Las filas
respondidas quedan como histórico y una amistad eliminada se marca, nunca se
borra.

**`grupos`**

| Columna | Tipo | Uso |
|---|---|---|
| `id` | `UUID` | Clave primaria. |
| `nombre` | `VARCHAR(80)` | Obligatorio y no vacío. |
| `descripcion` | `VARCHAR(500)` | Opcional. |
| `moneda` | `VARCHAR(3)` | Código ISO de tres letras, en mayúsculas. |
| `actividad` | `VARCHAR(120)` | Texto libre opcional («Viaje a Mérida, marzo»). |
| `fecha_actividad` | `DATE` | Fecha opcional, alternativa a `actividad`. |
| `creado_por` | `UUID` → `usuarios` | Quién creó el grupo. |
| `fecha_creacion` | `TIMESTAMPTZ` | Cuándo se creó. |

**`miembros`**

| Columna | Tipo | Uso |
|---|---|---|
| `grupo_id` | `UUID` → `grupos` | Parte de la clave primaria. |
| `usuario_id` | `UUID` → `usuarios` | Parte de la clave primaria. |
| `rol` | `VARCHAR(13)` | `administrador` o `miembro`. |
| `estado` | `VARCHAR(9)` | `invitado` o `aceptado`. |
| `invitado_por` | `UUID` → `usuarios` | Quién envió la invitación. |
| `fecha_invitacion` | `TIMESTAMPTZ` | Cuándo se invitó. |
| `fecha_respuesta` | `TIMESTAMPTZ` | Cuándo aceptó. |

La clave primaria `(grupo_id, usuario_id)` impide que alguien entre dos veces
al mismo grupo y sirve de destino a las claves foráneas de `gastos`. La
especificación solo admite los estados `invitado` y `aceptado`, así que
rechazar una invitación a un grupo **borra la fila**; se anota como deuda no
guardar ese rechazo.

**`gastos`**

| Columna | Tipo | Uso |
|---|---|---|
| `id` | `UUID` | Clave primaria. |
| `grupo_id` | `UUID` → `grupos` | Grupo al que pertenece. |
| `concepto` | `VARCHAR(200)` | Obligatorio y no vacío. |
| `monto` | `BIGINT` | Positivo, en céntimos. |
| `fecha` | `DATE` | Fecha del gasto. |
| `pagador_id` | `UUID` | Quién pagó; miembro del grupo. |
| `regla` | `VARCHAR(11)` | `iguales`, `porcentajes` o `montos`. |
| `registrado_por` | `UUID` | Quién lo registró (miembro del grupo). |
| `fecha_registro` | `TIMESTAMPTZ` | Cuándo se registró. |

Las claves foráneas compuestas `(grupo_id, pagador_id)` y
`(grupo_id, registrado_por)` referencian `miembros (grupo_id, usuario_id)`: la
base garantiza que el pagador y quien registra son miembros. Que estén
aceptados se comprueba en el caso de uso, porque una clave foránea no filtra
por columna.

**`partes_gasto`**

| Columna | Tipo | Uso |
|---|---|---|
| `gasto_id` | `UUID` → `gastos` | Parte de la clave primaria. |
| `usuario_id` | `UUID` → `usuarios` | Parte de la clave primaria. |
| `monto` | `BIGINT` | Importe exacto en céntimos, ya con el residuo aplicado. |
| `lleva_residuo` | `BOOLEAN` | Si esa parte recibió un céntimo de residuo. |
| `porcentaje` | `SMALLINT` | Centésimas de punto (3333 = 33,33 %); solo con la regla `porcentajes`. |

La suma de `monto` de las partes de un gasto tiene que ser igual a
`gastos.monto` (INV-07). El repositorio lo comprueba dentro de la misma
transacción que inserta el gasto y las partes: si no cuadra, no se guarda
nada. Se consideró un disparador diferido de PostgreSQL, que daría la misma
garantía en la base; se deja como endurecimiento opcional para cuando exista
el caso de uso, porque hoy no hay ninguna ruta que escriba en estas tablas.

### 3. Por qué los montos son decimales exactos

RNF-08 prohíbe la coma flotante binaria para dinero e INV-07 exige que la
suma de las partes sea **exactamente** el monto del gasto.

Un `double` no puede representar 0,10: guarda una aproximación, y
`0,10 + 0,20` no da `0,30`. Con una división en tres partes el error se
reparte de forma distinta según el orden de las sumas y el resultado cambia
con la plataforma. Eso rompe las dos cosas que la entrega tiene que
demostrar: que no se pierde ni se inventa un céntimo (ESC-10) y que los
balances del grupo suman cero (INV-08).

Por eso:

- **Los montos se guardan como enteros en la unidad mínima**
  (`BIGINT` de céntimos) y **los porcentajes como enteros en centésimas de
  punto** (`SMALLINT`, 3333 = 33,33 %). Un entero de escala fija 2 *es* un
  decimal exacto; no hay aproximación que perder.
- **Toda la aritmética es entera**: multiplicar, dividir con truncado hacia
  abajo y sumar. El procedimiento de la especificación —truncar la parte
  exacta de cada participante, calcular el residuo y repartirlo de a un
  céntimo en orden alfabético de nombre de usuario— se implementa tal cual,
  sin ninguna operación que redondee por su cuenta.
- **La suma cuadra por construcción**: la parte de cada persona es un entero
  y el residuo se reparte hasta agotarlo, así que `Σ partes = monto` siempre.
  No es una comprobación que pueda fallar por un céntimo, es cómo se calcula.
- **El residuo queda visible**: `lleva_residuo` marca las partes que
  recibieron un céntimo de más, para que la interfaz pueda mostrar «incluye
  0,01 de redondeo».
- **El determinismo no depende de la base**: el reparto del residuo se ordena
  por nombre de usuario en el código, no con `ORDER BY`, para no depender del
  *collation* de PostgreSQL. Los mismos datos producen siempre el mismo
  resultado, que es lo que pide INV-07.

Cuando existan los endpoints, los montos viajarán como **texto** (`"100.00"`),
no como número JSON: un `number` se convierte en `double` en el cliente y
volvería a introducir el problema en el borde.

Se descartó `NUMERIC(18,2)` con `Decimal` en Python. También es exacto, pero
obliga a fijar la escala en dos sitios, a decidir el redondeo de cada
operación y a convivir con dos representaciones del dinero en el proyecto: la
aplicación ya guarda los montos como enteros en unidades mínimas (ADR-006).

### 4. Por qué eliminar una amistad no toca los grupos

`grupos`, `miembros` y `gastos` no tienen ninguna clave foránea a
`amistades`. La amistad solo habilita **invitar** a un grupo; el vínculo que
autoriza leer y escribir es la pertenencia (`miembros`), que es lo que exige
INV-17.

Además de ser lo que pide RF-SOC-06 («no borra el historial de deudas, gastos
ni grupos ya existentes»), hay una razón aritmética: si al eliminar una
amistad se borraran sus gastos o su pertenencia, los balances de los demás
miembros dejarían de sumar cero y un tercero perdería o ganaría dinero por
una decisión tomada entre otras dos personas. INV-08 no lo permite.

Por eso eliminar una amistad es un cambio de estado (`estado = 'eliminada'`,
`fecha_baja`), no un `DELETE`:

- los grupos y sus gastos siguen intactos y visibles para sus miembros;
- las dos personas dejan de poder invitarse a grupos nuevos, porque la
  comprobación de amistad consulta una amistad **aceptada** y esa ya no
  existe;
- se conserva el histórico y el índice único parcial sigue funcionando.

### 5. Firmas de los repositorios

Se definen tres módulos, con el mismo patrón que `repositorio_usuarios.py`:
dataclass `frozen` para las filas, la conexión inyectada en el constructor y
excepciones propias que la capa HTTP traducirá a 400, 403, 404 o 409, sin
filtrar detalles del driver.

| Módulo | Tipos principales | Operaciones |
|---|---|---|
| `repositorio_amistades.py` | `EstadoAmistad`, `Amistad`, `AmistadVista` | crear, buscar por id, buscar la amistad viva entre dos personas, listar, responder, eliminar, `son_amigos` |
| `repositorio_grupos.py` | `Rol`, `EstadoMiembro`, `Grupo`, `Miembro`, `GrupoVista`, `MiembroVista` | crear grupo con su administrador, buscar, listar grupos de una persona, listar miembros, invitar, aceptar, rechazar |
| `repositorio_gastos.py` | `Gasto`, `ParteGasto`, `GastoVista` | registrar gasto con sus partes en una transacción, listar los de un grupo |

Detalles que fijan las firmas:

- `monto: int` en céntimos y `porcentaje: int | None` en centésimas: en la
  frontera del repositorio no existe `float` ni un `Decimal` ambiguo.
- `crear` de grupos recibe la moneda ya normalizada; el repositorio no
  reinventa validaciones que pertenecen a las reglas.
- `registrar` de gastos recibe las partes **ya calculadas** y valida la suma
  antes del `COMMIT`, de modo que quien calcule sea una función pura
  probable, y el repositorio solo persista.
- `AmistadVista` y `MiembroVista` no llevan correo: RF-SOC-04 solo permite
  nombre de usuario y nombre visible.

## Alternativas consideradas

**Una sola fila por pareja para siempre, sin histórico.** Un índice único
total sobre la pareja es más simple, pero un rechazo impediría volver a
invitar y una amistad eliminada bloquearía para siempre. Se descartó porque
RF-SOC-05 habla de invitaciones pendientes, no de intentos de por vida.

**`rechazado` como tercer estado de `miembros`.** Guardaría el histórico de
rechazos a grupos, pero la especificación fija exactamente `invitado` y
`aceptado`. Añadirlo sería inventar contrato; queda como deuda.

**Referenciar `amistades` desde `miembros`.** Haría que borrar una amistad
rompiera las claves foráneas de grupos existentes o exigiera cascadas, justo
lo que RF-SOC-06 prohíbe.

**`NUMERIC(18,2)` o `float` para los montos.** Descritas en la sección 3.

**Guardar las partes del gasto en un `JSONB` dentro de `gastos`.** Un solo
`INSERT`, pero sin claves foráneas, sin índices por participante y con las
sumas en el código de la aplicación: imposible que la base ayude a sostener
INV-07.

**`Protocol` y dobles en memoria en lugar de repositorios concretos.** Sin
implementación todavía no aporta nada que las firmas concretas no den, y el
proyecto ya prueba contra PostgreSQL real.

## Consecuencias

### Positivas

- INV-07 e INV-08 quedan sostenidos por el tipo de dato y por cómo se calcula,
  no por una comprobación que pueda fallar en un caso raro.
- La unicidad de la pareja y la de la pertenencia a un grupo las impone la
  base, no solo el código.
- La autorización de las rutas de grupo se apoya en la clave primaria
  `(grupo_id, usuario_id)`: una consulta basta para decidir si alguien es
  miembro.
- Eliminar una amistad no puede alterar balances ajenos.
- INV-16 sigue protegido por una prueba que enumera las tablas permitidas.

### Negativas / deuda a considerar

- **Supuesto de dos decimales** para toda moneda: una moneda con 0 o 3
  decimales exigiría guardar la escala junto al código ISO.
- **Sin histórico de rechazos** a invitaciones de grupo, porque la tabla solo
  admite dos estados.
- **Sin edición ni borrado de gastos**: fuera del alcance de la entrega; el
  gasto es inmutable de momento.
- **Sin migraciones**: el esquema se sigue creando con `CREATE ... IF NOT
  EXISTS` al arrancar, como decidió ADR-009. Con cinco tablas nuevas eso
  empieza a pesar; el primer cambio de una columna existente exigirá un
  mecanismo de migraciones.
- **INV-07 se valida en el repositorio**, no en la base, salvo que se añada el
  disparador diferido mencionado.
- **La base no valida que la moneda exista** en ISO 4217: el servidor no tiene
  catálogo de monedas y no debe tenerlo (INV-16).

## Implementación

- `servidor/esquema.py`: DDL completo, agrupado por dominio y comentado.
- `servidor/base_datos.py`: aplica las sentencias al arrancar y en las
  pruebas.
- `servidor/repositorio_amistades.py`, `servidor/repositorio_grupos.py` y
  `servidor/repositorio_gastos.py`: tipos y firmas, sin implementar todavía.
- `servidor/tests/test_seguridad.py`: lista de tablas permitidas por INV-16.
- `docs/adr/ADR.md`: índice actualizado.

## Estado

Borrador, pendiente de revisión del equipo. Sustituye el punto de ADR-009
sobre el alcance del esquema (que entonces era solo `usuarios`).

## Fecha

2026-11-10
