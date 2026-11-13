# ADR-013: División exacta, balances y autorización de grupo en el servidor

## Contexto

ADR-011 fijó qué se guarda de los grupos y ADR-012 cómo se guarda el dinero
(`NUMERIC(14, 2)`) y que el repositorio persiste el gasto con sus partes en una
transacción. Faltaba decidir las reglas que viven **por encima** del
repositorio:

1. quién calcula las partes de un gasto y con qué procedimiento, de modo que lo
   que la app muestra en la vista previa y lo que guarda el servidor coincidan
   céntimo a céntimo (RF-COM-05, INV-07, RNF-08, ESC-10);
2. cómo se obtienen los balances de un grupo y por qué suman exactamente cero
   (RF-COM-08, INV-08);
3. quién puede leer y escribir en un grupo y qué responde el servidor a los
   demás (INV-17, RNF-07, ESC-08);
4. la forma de `POST /grupos/{id}/gastos` y `GET /grupos/{id}` que consume la
   app (`app/src-tauri/src/dto_social.rs`).

## Decisión

### 1. El servidor divide; el cliente solo propone

`POST /grupos/{id}/gastos` recibe el monto, la regla y, por participante, su
nombre de usuario y su valor (porcentaje o monto fijo). **Nunca recibe las
partes ya calculadas.** El servidor llama a `division.dividir`, guarda lo que
resulta y lo devuelve. Así un cliente modificado no puede guardar partes que no
sumen el total ni repartir el residuo a su favor.

La app calcula lo mismo en local para la vista previa
(`app/src-tauri/crates/dominio/src/division.rs`), sin red. Las dos
implementaciones siguen el mismo procedimiento, con aritmética entera sobre
céntimos y sin coma flotante:

1. Se lee el monto como texto con punto (`"100.00"`): mayor que 0, hasta dos
   decimales, como máximo `9999999999.99`, sin notación científica.
2. Se ordena a los participantes por nombre de usuario en minúsculas.
3. Se calcula la parte exacta de cada uno y se **trunca** a céntimos
   (`total // n`, `total × porcentaje // 10000` o el monto fijo).
4. Residuo = total − suma de las partes truncadas.
5. El residuo se reparte de a un céntimo, en ese orden alfabético, hasta
   agotarlo. Esas partes llevan `incluye_redondeo: true`.

Por construcción `Σ partes = total`: no es una comprobación que pueda fallar
por un céntimo, es cómo se calcula. El repositorio (ADR-012) vuelve a
comprobarlo antes del `COMMIT` como defensa en profundidad.

Las validaciones y los mensajes también son idénticos en los dos lados:

| Caso | Campo | Mensaje |
|---|---|---|
| Porcentajes que no suman 100 | `porcentajes` | «Los porcentajes suman 95; faltan 5.» |
| Montos fijos que no suman el total | `montos` | «Los montos suman 118,00; faltan 2,00.» |
| Monto inválido | `monto` | «El monto debe ser mayor que 0 y tener hasta dos decimales.» |
| Regla desconocida | `regla` | «Elige una regla de división válida.» |
| Sin participantes o repetidos | `participantes` | «Elige al menos un participante.» |

Con «sobran» cuando la suma se pasa. Los números de los mensajes van con coma.

**El contrato es `docs/vectores-division.json`.** Treinta divisiones y cinco
balances, con los ejemplos del enunciado. Lo recorren
`servidor/tests/test_vectores_division.py` (con `division.dividir` y
`division.calcular_balances`) y `app/src-tauri/tests/vectores_division.rs`. En
los errores el campo debe coincidir siempre y el mensaje donde
`mensaje_exacto` es verdadero. Si alguien cambia una de las dos
implementaciones sin la otra, falla la batería del lado que cambió.

Además de la división, la ruta valida `concepto` (obligatorio, hasta 200
caracteres), `fecha` (`AAAA-MM-DD` y una fecha real) y que el pagador y cada
participante sean **miembros aceptados**. Un miembro puede quedar fuera del
reparto. Todos los errores se informan juntos, por campo, con un 400.

### 2. Los balances se calculan al leer, desde las partes guardadas

No hay tabla de balances. `GET /grupos/{id}` lee los gastos y sus partes y
llama a `division.calcular_balances`, una función pura:

- `pago` = suma de los gastos que pagó la persona;
- `le_corresponde` = suma de sus partes;
- `balance = pago − le_corresponde` (positivo: le deben; negativo: debe).

Aparece cada miembro aceptado, aunque no tenga gastos (con `0.00`), en orden
alfabético. Como cada gasto cuadra exactamente (INV-07), lo pagado y lo que
corresponde suman lo mismo y **los balances suman `0.00` siempre** (INV-08).
Un balance guardado podría desincronizarse de los gastos; uno calculado no.

### 3. Autorización: miembro aceptado o el mismo 404

`autorizacion.pertenencia_aceptada` es la única función que decide si alguien
puede tocar un grupo. Devuelve el grupo y la membresía, o `None` en cuatro
casos que deben ser indistinguibles:

1. el identificador no es un UUID;
2. el grupo no existe;
3. la persona no pertenece al grupo;
4. la persona está invitada pero aún no aceptó.

En los cuatro, `GET /grupos/{id}`, `POST /grupos/{id}/gastos` y
`POST /grupos/{id}/miembros` responden **404 con el cuerpo
`{"mensaje": "Grupo no encontrado."}`**, byte a byte igual. Responder 403 a un
ajeno revelaría que el grupo existe y permitiría enumerar identificadores.

El orden de las comprobaciones es fijo: sesión (401), pertenencia (404) y
datos (400). Un ajeno que envía datos inválidos recibe el 404, no el 400: no
aprende nada del grupo ni de sus reglas. La única excepción son los errores de
tipo del cuerpo (por ejemplo, un monto enviado como número JSON), que FastAPI
detecta antes de entrar en la ruta; no dependen del grupo y no revelan nada.

### 4. Forma de las respuestas

Las respuestas siguen los DTO de la app para que los comandos reales no tengan
que traducir:

- `POST /grupos/{id}/gastos` → 201 con `GastoGrupoDto`:
  `{id, concepto, monto, fecha, pagador, regla, partes: [{usuario, monto,
  incluye_redondeo}]}`.
- `GET /grupos/{id}` → 200 con `GrupoDetalleDto`:
  `{grupo, miembros: [{usuario, nombre_visible, rol}], gastos, balances:
  [{usuario, pago, le_corresponde, balance}]}`. Los gastos van del más
  reciente al más antiguo; las partes, los miembros y los balances, en orden
  alfabético.

La regla viaja con el vocabulario de la app (`partes_iguales`, `porcentajes`,
`montos_fijos`) aunque la base guarde los nombres canónicos de ADR-012
(`iguales`, `porcentajes`, `montos`). Los montos salen como texto con dos
decimales. A las personas se las nombra por su nombre de usuario; nunca
aparece un correo (RF-SOC-04) ni un dato del libro personal (INV-16).

### 5. Las rutas declaran lo que esperan de los repositorios

Las rutas dependen de protocolos (`AlmacenGrupos`, `AlmacenGastos`, ...) y no
de las clases concretas. Al integrar esta ADR se vio que las rutas de grupos
llamaban a métodos que solo tenía el almacén en memoria de las pruebas
(`crear_grupo`, `listar_grupos_de`, ...): todas las pruebas pasaban y la ruta
fallaba contra PostgreSQL. Por eso:

- los protocolos usan los nombres de `RepositorioGrupos`;
- `ciclo_de_vida` y `conftest.py` asignan cada almacén a una variable con el
  tipo del protocolo, y `mypy --strict` falla si un repositorio (real o en
  memoria) deja de cumplirlo;
- el flujo completo se prueba también contra PostgreSQL real
  (`test_integracion_postgres.py`).

## Alternativas consideradas

**Que el cliente envíe las partes calculadas y el servidor solo valide.**
Ahorra un cálculo, pero el servidor tendría que verificar el reparto del
residuo igualmente y la API quedaría abierta a partes «válidas» pero injustas
(el céntimo siempre para quien registra). Calcular en el servidor es más
simple y no admite trampas.

**Guardar los balances en una tabla y actualizarlos con cada gasto.** Más
rápido de leer, pero dos fuentes de verdad que pueden divergir, y una
corrección de gasto futura obligaría a recalcular de todos modos. Con grupos
pequeños, calcular al leer es barato.

**403 para quien no es miembro.** Más explícito, pero distingue «existe y no
eres miembro» de «no existe» (RNF-07, ESC-08).

**Redondear al céntimo más cercano** en lugar de truncar. Haría que el residuo
pudiera ser negativo (habría que quitar céntimos) y complicaría explicar quién
lleva el redondeo. El enunciado pide truncar y repartir.

**Un solo motor de división compartido** (por ejemplo, compilar el de Rust a
Python). Elimina la duplicación, pero añade una cadena de compilación al
servidor. Dos implementaciones cortas fijadas por los mismos vectores dan la
misma garantía con menos piezas.

## Consecuencias

### Positivas

- Lo previsualizado en la app y lo guardado en el servidor coinciden, y hay
  pruebas en los dos lados que lo demuestran con los mismos números.
- Ningún céntimo se pierde ni se inventa: la suma de partes y la de balances
  cuadran por construcción.
- Un ajeno no puede saber si un grupo existe.
- Una divergencia entre las rutas y los repositorios reales la detecta `mypy`
  antes de llegar a PostgreSQL.

### Negativas / deuda

- La división está escrita dos veces (Python y Rust). Los vectores lo
  mitigan, pero un caso nuevo hay que añadirlo al JSON para que cubra a ambos.
- Los balances se recalculan en cada lectura. Con grupos muy grandes habría
  que paginar los gastos o guardar totales.
- Los errores de tipo del cuerpo se responden antes de comprobar la
  pertenencia (ver punto 3).
- Gastos inmutables: corregir o borrar un gasto queda fuera de la entrega.

## Implementación

- `servidor/division.py`: `dividir`, `calcular_balances` y `centimos_exactos`.
- `servidor/autorizacion.py`: `pertenencia_aceptada` y el 404 uniforme.
- `servidor/main.py`: `POST /grupos/{id}/gastos`, `GET /grupos/{id}` y los
  protocolos tipados de los almacenes.
- `servidor/openapi.yaml`: las dos rutas y sus esquemas.
- `servidor/tests/test_vectores_division.py`: el servidor contra los vectores.
- `servidor/tests/test_gastos_grupo.py`, `test_autorizacion.py` y
  `test_integracion_postgres.py`: rutas, 404 uniforme y flujo completo como
  usuario nuevo, en memoria y contra PostgreSQL.
- `app/src-tauri/src/autonomia.rs`: con el servidor apagado, el libro
  personal y la vista previa siguen funcionando.

## Estado

Aceptada. Complementa ADR-011 y ADR-012; no sustituye ninguna.

## Fecha

2026-11-12
