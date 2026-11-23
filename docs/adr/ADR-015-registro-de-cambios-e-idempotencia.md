# ADR-015: Registro de cambios por usuario y clave de idempotencia en el servidor

## Contexto

En la Entrega 7, cada participante de un gasto de grupo debe encontrar su parte
en su libro personal exactamente una vez (RF-COM-10, INV-15), aunque sincronice
muchas veces, reinicie el cursor o se corte a mitad de un lote (INV-13). Además,
la app puede registrar un gasto sin conexión y reenviarlo después, así que un
reintento no puede crear un segundo gasto (RNF-17, ESC-09).

ADR-013 fijó cómo se divide un gasto y quién puede leerlo. Faltaba decidir:

1. qué guarda el servidor para que cada persona baje solo **sus** partes, en
   orden y sin perder ni repetir ninguna;
2. cómo se pagina esa lectura y qué significa el cursor;
3. cómo se reconoce un reintento del mismo `POST /grupos/{id}/gastos`;
4. dónde viven esas reglas para poder probarlas sin PostgreSQL.

La forma de los eventos y de los errores se acordó en el equipo (columna
«Propuesta») y está en el borrador de `servidor/openapi.yaml` y en
`docs/vectores-sincronizacion.json`.

## Decisión

### 1. Un registro de cambios por usuario, que solo crece

Al registrar un gasto, en la **misma transacción** que el gasto y sus partes, el
servidor agrega a la tabla `cambios` una fila por participante con su parte:
grupo (id y nombre), gasto (id y concepto), fecha, moneda, monto e
`incluye_redondeo`. Cada fila tiene un `id` (UUID) inmutable y una `posicion`
que empieza en 1 y crece de uno en uno **dentro del registro de esa persona**.

- Quien queda fuera del reparto no recibe evento ni consume posición. Tampoco
  lo recibe una parte de 0,00 (por ejemplo, 0,02 entre tres): no hay nada que
  aplicar y el libro personal rechaza movimientos de cero.
- El nombre del grupo se copia al registrar el gasto, para que el movimiento
  de la app no cambie si el grupo se renombra.
- La tabla no tiene columna `tipo`: hoy todos los eventos son
  `gasto_de_grupo` y `GET /cambios` lo escribe como valor fijo. Un tipo nuevo
  agregará su columna o su tabla cuando exista.
- **Solo crece** (INV-13): dos disparadores rechazan cualquier `UPDATE` o
  `DELETE` sobre `cambios`, y una prueba revisa que el código no tenga
  ninguno. `UNIQUE (usuario_id, posicion)` y `UNIQUE (usuario_id, gasto_id)`
  impiden posiciones repetidas y dos eventos del mismo gasto para alguien.
- La última posición de cada persona vive en `posiciones_usuario`. El registro
  de un gasto bloquea esas filas con `SELECT … FOR UPDATE`, siempre ordenadas
  por id, para que dos gastos simultáneos no repitan ni salten posiciones y no
  se bloqueen entre sí.

INV-16: el evento solo describe la parte del gasto de grupo. El servidor no
sabe en qué cuenta la aplica cada persona ni cuál es su saldo.

### 2. `GET /cambios?desde=<cursor>`: lotes de 100 por posición

- Devuelve `{eventos, cursor, hay_mas}` con los eventos de **quien consulta**
  posteriores a `desde`, en orden de posición y como máximo 100. El cursor es
  la posición del último evento entregado o, si no hay ninguno, el mismo
  `desde`. Si `hay_mas` es verdadero, la app vuelve a pedir con ese cursor.
- Sin `desde` se toma 0. Pedir otra vez desde 0 devuelve los mismos eventos
  con los mismos ids: la app los reconoce por id y no los aplica dos veces.
- El filtro por usuario es lo único que se lee: nunca salen eventos de otras
  personas ni de grupos ajenos (INV-17), y por eso la ruta no responde 404.
- Orden de comprobaciones: 401 y luego 400. Un `desde` que no son solo
  dígitos ASCII (negativo, decimal, con signo, vacío) o que pasa de
  2⁶³ − 1 responde **400** con el campo `desde`, nunca el 422 de FastAPI: el
  parámetro se recibe como texto y lo valida `cambios.leer_desde`.
- El repositorio pide 101 filas y `cambios.armar_lote` decide `hay_mas` sin
  una segunda consulta.

### 3. `Idempotency-Key` opcional, por usuario, con huella del cuerpo

- El encabezado es **opcional** para el servidor: sin él, el registro se
  comporta como en la Entrega 6 y siempre responde 201. La app lo envía
  siempre: un UUID generado una vez por operación y repetido en cada reintento.
- Una clave válida tiene de 8 a 128 caracteres ASCII visibles. Su alcance es
  **por usuario** (clave primaria `(usuario_id, clave)`): la misma clave de dos
  personas no choca.
- Junto a la clave se guarda la **huella**: SHA-256 del JSON canónico (claves
  ordenadas, sin espacios) con grupo, concepto, monto, fecha, pagador, regla y
  las partes ya calculadas, ordenadas por usuario. Se calcula sobre los datos
  validados y la división hecha, no sobre el texto recibido: «50» y «50.00», o
  un participante escrito con otras mayúsculas, describen el mismo gasto.
- Orden de comprobaciones: **401 → 404 → 400 → 200/409**. El error de una clave
  mal formada se informa en el mismo 400 que los demás campos
  (`idempotency_key`).
  - Clave nueva: se registra y responde **201**.
  - Clave ya usada con la misma huella: **200** con el gasto original, igual
    que la primera respuesta, sin gasto ni eventos nuevos.
  - Clave ya usada con otra huella: **409** con el campo `idempotency_key`.
    Es otra operación y necesita otra clave.
- La clave se inserta en la misma transacción que el gasto con
  `ON CONFLICT DO NOTHING`. Si dos reintentos llegan a la vez, el segundo
  espera al primero, no inserta nada y se deshace entero (`ClaveYaUsada`); la
  ruta vuelve a leer la clave y responde 200 o 409. Un 400 o un 404 no
  consumen la clave, porque no llega a guardarse.

### 4. Reglas puras separadas del SQL

- `servidor/cambios.py`: `eventos_de_gasto` (un evento por participante y
  posición siguiente), `leer_desde`, `armar_lote` y la forma JSON del evento.
- `servidor/idempotencia.py`: `validar_clave`, `huella_gasto` y `decidir`
  (registrar, devolver el existente o conflicto).

No tocan la base, la red ni el reloj. El repositorio de PostgreSQL y el
almacén en memoria de las pruebas usan las mismas funciones, y
`eventos_de_gasto` se compara campo a campo con
`docs/vectores-sincronizacion.json` (los ids se inyectan desde el archivo).

## Alternativas consideradas

- **Calcular los cambios al leer** desde `gastos` y `partes_gasto`, sin tabla
  propia. No da ids ni posiciones estables: si un gasto se corrigiera o se
  agregara un tipo nuevo, la numeración cambiaría y la app aplicaría partes dos
  veces o se saltaría alguna.
- **Una secuencia global** (`BIGSERIAL`) como cursor. Es más simple, pero deja
  huecos visibles para cada persona y revela cuánta actividad hay en el
  servidor. Los vectores del equipo fijan posiciones 1, 2, 3… por usuario.
- **Paginar por fecha** de registro. Dos gastos en el mismo segundo empatan y
  una página podría saltarse uno.
- **Guardar la respuesta completa** junto a la clave. Se descartó: el gasto
  ya está guardado y se reconstruye igual que en `GET /grupos/{id}`.
- **Clave obligatoria.** Rompería a los clientes de la Entrega 6 y las pruebas
  existentes sin ganar seguridad: la idempotencia protege al cliente que
  reintenta, que siempre la envía.
- **Clave global** (no por usuario). Una persona podría bloquear o adivinar
  operaciones ajenas reutilizando su clave.

## Consecuencias

### Positivas

- Sincronizar muchas veces, desde 0 o después de un corte no duplica nada: los
  ids no cambian y la app los reconoce.
- Un reintento por respuesta perdida devuelve el mismo gasto; un error de la
  app que reusa una clave se detecta (409) en vez de pasar en silencio.
- Las reglas se prueban sin PostgreSQL, con propiedades (Hypothesis) y con los
  vectores compartidos que también lee la app.

### Negativas / deuda

- Tres tablas nuevas (`cambios`, `posiciones_usuario`,
  `claves_idempotencia`). Las claves no caducan; si crecen demasiado habrá que
  purgarlas por antigüedad (no forman parte del registro inmutable).
- Si un día se corrige o elimina un gasto de grupo, hará falta un tipo de
  evento nuevo; el registro no se reescribe.
- El nombre del grupo en el evento es una copia: renombrar el grupo no cambia
  los movimientos ya aplicados (es lo buscado, pero hay que saberlo).

## Implementación

- `servidor/cambios.py` e `servidor/idempotencia.py`: reglas puras.
- `servidor/main.py`: `GET /cambios` y la clave en `POST /grupos/{id}/gastos`,
  con los protocolos `AlmacenGastos` (clave, huella, `buscar_clave`, `buscar`)
  y `AlmacenCambios` (`listar_desde`) que deben cumplir los repositorios.
- Almacenamiento en PostgreSQL (tablas `cambios` con sus disparadores,
  `posiciones_usuario` y `claves_idempotencia`; contador con bloqueo de fila;
  eventos y clave en la transacción del gasto): lo implementan los
  repositorios de cambios y de claves de la Entrega 7, siguiendo los puntos 1
  a 3. Hasta entonces `repositorio_cambios.py` y `repositorio_gastos.py` solo
  traen las firmas.
- Pruebas: `test_reglas_cambios.py`, `test_reglas_idempotencia.py`,
  `test_cambios_http.py`, `test_idempotencia_http.py` y
  `test_flujo_entrega7.py` (flujo completo como usuario nuevo) sobre el
  almacén en memoria de `tests/repositorio_en_memoria.py`, que sigue las
  mismas reglas. El mismo flujo contra PostgreSQL está en
  `test_integracion_postgres.py`, omitido hasta que existan los repositorios.
- `app/src-tauri/crates/infraestructura/tests/autonomia_entrega7.rs`: con el
  servidor apagado, lo ya sincronizado y lo personal siguen al reabrir, y cada
  usuario tiene su libro con solo sus partes.

## Estado

Aceptada. Complementa ADR-013 y ADR-014; no sustituye ninguna.

## Fecha

2026-11-22
