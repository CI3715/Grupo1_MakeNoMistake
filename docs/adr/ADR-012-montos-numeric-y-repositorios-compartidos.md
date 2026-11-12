# ADR-012: Montos `NUMERIC` y repositorios compartidos implementados

## Contexto

ADR-011 dejó decidido el esquema de amistades, grupos, miembros y gastos, y
las firmas de sus repositorios, pero eligió guardar el dinero como `BIGINT` en
céntimos y dejó todos los métodos con `NotImplementedError`.

La revisión de la Entrega 6 pide dos cambios concretos:

1. los montos deben ser **`NUMERIC`, nunca coma flotante**;
2. los repositorios deben estar implementados y probados directamente contra
   la base de pruebas, incluyendo la transacción única del gasto y sus partes.

Esta ADR sustituye el punto 3 de ADR-011 (representación del dinero) y
completa su punto 5 (firmas) con la implementación.

## Decisión

### 1. `NUMERIC(14, 2)` para los montos

`gastos.monto` y `partes_gasto.monto` pasan de `BIGINT` en céntimos a
`NUMERIC(14, 2)`: doce dígitos enteros y dos decimales exactos.

- PostgreSQL calcula, suma y compara `NUMERIC` con precisión decimal, sin
  aproximación binaria (RNF-08).
- Python usa `Decimal`, nunca `float`. El repositorio rechaza un `float` de
  forma explícita para que no entre por accidente.
- Un valor con más de dos decimales se rechaza antes del `INSERT`; no se
  redondea en silencio. Así lo que se valida es lo que se guarda.
- El máximo admitido es `999.999.999.999,99`, el mismo que el motor de
  división de la aplicación.

Los porcentajes siguen siendo `SMALLINT` en centésimas de punto (`3333` =
`33,33 %`). Son enteros exactos y no necesitan `NUMERIC`.

La base de desarrollo que ya tuviera las columnas como `BIGINT` se convierte
una sola vez con un bloque idempotente en `esquema.py`: comprueba el tipo en
`information_schema.columns` y solo entonces divide por 100. En una base nueva
las tablas nacen directamente `NUMERIC` y el bloque no hace nada.

### 2. Reglas canónicas y alias de la interfaz

La base acepta `iguales`, `porcentajes` y `montos`. La interfaz usa
`partes_iguales` y `montos_fijos`; el repositorio traduce esos dos alias al
valor canónico antes del `INSERT`. Así no hay que migrar el `CHECK` ni cambiar
la aplicación, y la base sigue guardando un único vocabulario.

### 3. El gasto y sus partes, en una transacción

`RepositorioGastos.registrar` hace todo dentro de una sola conexión:

1. valida monto, concepto, regla y partes;
2. comprueba que la suma sea exactamente el monto;
3. comprueba la regla: reparto igual, porcentajes que suman `10000` o montos
   fijos que suman el total;
4. comprueba que pagador, registrador y participantes sean miembros
   **aceptados** del grupo;
5. inserta el gasto y todas las partes;
6. deja que el gestor de contexto de `psycopg` confirme o revierta.

Si cualquier paso falla, no queda ni el gasto ni una parte suelta. El
repositorio no vuelve a dividir: recibe `ParteCalculada` (sin `gasto_id`, porque
el identificador lo genera él) y solo persiste. Se sigue aceptando
`ParteGasto` como entrada por compatibilidad, pero el `gasto_id` se reescribe
con el del gasto recién creado.

### 4. Errores con campo y regla

`GastoInvalido` lleva `campo` y `regla`, además del mensaje:

| Ejemplo | `campo` | `regla` |
|---|---|---|
| Las partes no suman el monto | `partes` | `suma` |
| Los porcentajes no suman 100 % | `porcentajes` | `porcentajes` |
| Regla desconocida | `regla` | `regla` |
| Participante ajeno al grupo | `participantes` | `miembro` |
| Pagador no aceptado | `pagador` | `miembro` |
| Monto con más de dos decimales | `monto` | — |

La capa HTTP podrá responder `400` con ese campo sin mirar el driver.

### 5. Eliminar una amistad no borra nada compartido

`RepositorioAmistades.eliminar` cambia el estado a `eliminada` y guarda
`fecha_baja`; no ejecuta `DELETE`. `grupos`, `miembros` y `gastos` no tienen
claves foráneas hacia `amistades`, así que la operación no puede borrarlos ni
romper sus claves. Hay una prueba que crea un grupo con un gasto, elimina la
amistad y comprueba que el grupo, sus miembros y el gasto siguen ahí.

## Alternativas consideradas

**Mantener `BIGINT` en céntimos.** También es exacto y era la decisión de
ADR-011, pero la especificación pide `NUMERIC` de forma explícita. Cambiar la
columna ahora, antes de que haya datos en producción, es más barato que
hacerlo después.

**Guardar `NUMERIC` y seguir recibiendo enteros de céntimos en Python.**
Obligaría a traducir en cada lectura y escritura y a decidir en qué unidad
habla cada capa. Se descartó: una sola representación (`Decimal` en euros) es
más difícil de confundir.

**Validar la suma con un disparador diferido de PostgreSQL.** Daría la misma
garantía incluso para SQL escrito a mano, pero complica el esquema y la
traducción de errores. Queda como endurecimiento opcional; hoy la validación
está en el repositorio, antes del `COMMIT`.

## Consecuencias

### Positivas

- El tipo de la columna y el tipo de Python dicen lo mismo: dinero decimal
  exacto.
- La regla se comprueba en el repositorio, no solo en la interfaz.
- Un fallo a mitad de inserción no deja gastos huérfanos ni partes sueltas.
- Los errores identifican el campo y la regla que fallan.
- La unicidad de la pareja sigue garantizada en los dos sentidos por el
  índice único parcial de `amistades`.

### Negativas / deuda

- La escala queda fija en dos decimales para toda moneda. Una moneda con 0 o
  3 decimales exigiría guardar la escala junto al código ISO.
- Sigue sin haber un mecanismo general de migraciones: el cambio de `BIGINT` a
  `NUMERIC` se resolvió con un bloque idempotente concreto.
- Rechazar una invitación a un grupo borra la fila, porque la tabla solo
  admite `invitado` y `aceptado` (deuda heredada de ADR-011).

## Implementación

- `servidor/esquema.py`: columnas `NUMERIC(14, 2)` y conversión idempotente.
- `servidor/repositorio_amistades.py`: SQL de invitaciones, respuesta, baja y
  listado; la pareja se consulta en los dos sentidos.
- `servidor/repositorio_grupos.py`: grupo y administrador en una transacción,
  invitaciones, aceptación y rechazo.
- `servidor/repositorio_gastos.py`: validación por campo/regla y gasto con sus
  partes en una transacción.
- `servidor/tests/test_repositorio_amistades.py`,
  `servidor/tests/test_repositorio_grupos.py` y
  `servidor/tests/test_repositorio_gastos.py`: pruebas directas contra
  PostgreSQL.

## Estado

Aceptada. Sustituye el punto 3 de ADR-011 (representación del dinero) y
completa su punto 5 (firmas) con la implementación.

## Fecha

2026-11-11
