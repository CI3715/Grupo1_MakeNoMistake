# Formato de la exportación del libro (versión 1)

Este documento describe el archivo que produce **Copia de seguridad → Exportar sin cifrar**, para que puedas leer tu libro y reconstruir tus saldos **sin la aplicación** (RF-INT-01, RNF-04). Una copia **cifrada** contiene exactamente este mismo texto, cifrado con tu frase de paso (ver [ADR-017](adr/ADR-017-cifrado-con-frase-de-paso.md)); los lotes de sincronización también.

En [`herramientas/reconstruir_saldos.py`](../herramientas/reconstruir_saldos.py) hay un programa de ejemplo que hace la reconstrucción solo con la biblioteca estándar de Python:

```bash
python3 herramientas/reconstruir_saldos.py mi-copia.txt          # tabla legible
python3 herramientas/reconstruir_saldos.py mi-copia.txt --json   # JSON
```

## Qué contiene y qué no

- **Contiene** todos los eventos de tu libro: cuentas, categorías, movimientos, correcciones, partes de gastos de grupo, monedas propias, tasas y moneda base. El libro es la fuente de verdad: los saldos no se guardan, se calculan con los eventos.
- **No contiene** la frase de paso, tokens, contraseñas ni credenciales de Git (RF-INT-05), ni tu sesión del servidor.

## Estructura general

- Texto **UTF-8**, sin BOM. Cada línea termina en `\n` (también la última). Un archivo con fines de línea `\r\n` no es válido.
- La **primera línea** es el encabezado.
- Cada línea siguiente es **un evento**, en JSON, en el orden del libro.

### Encabezado

```text
# cuentas-claras-respaldo version=1 exportado=2026-11-26T10:00:00Z eventos=7
```

Exactamente cuatro partes separadas por un espacio:

| Parte | Significado |
|---|---|
| `# cuentas-claras-respaldo` | Marca fija: identifica el archivo. |
| `version=N` | Versión del formato. Este documento describe la `1`. |
| `exportado=…` | Instante de la exportación en UTC: `AAAA-MM-DDTHH:MM:SSZ` (se admite fracción de segundo, `…:SS.123Z`). |
| `eventos=M` | Cantidad de líneas de evento que siguen. |

Los números van en decimal, sin signo ni ceros a la izquierda.

### Línea de evento

```json
{"id":"…","tipo":"…","momento_registro":"…","dispositivo_origen":"…","datos":{…}}
```

| Campo | Tipo | Significado |
|---|---|---|
| `id` | texto, no vacío | Identificador único del evento. No se repite en el archivo. |
| `tipo` | texto, no vacío | Tipo de evento (ver abajo). |
| `momento_registro` | texto, no vacío | Instante UTC en que se registró, en el dispositivo de origen. |
| `dispositivo_origen` | texto | Dispositivo que creó el evento (`fabrica` para las categorías de fábrica). |
| `datos` | objeto | La carga del evento, tal como está guardada en el libro. |

La aplicación escribe siempre las claves en ese orden y en una sola línea. Al leer, el orden de las claves no importa, pero no puede faltar ni sobrar ninguna, ni repetirse.

`datos` es normalmente un **objeto JSON**. Si alguna vez un evento guardara una carga que no es un objeto JSON, viajaría como **texto JSON** (entre comillas) para que la ida y vuelta siga siendo exacta; un lector puede ignorar esos eventos para calcular saldos.

### Montos y monedas

- Todo monto es un **texto con un número decimal** (`"10.50"`, `"0.01250000"`), nunca un número JSON: así no se pierde precisión. Súmalos con aritmética decimal exacta (en Python, `decimal.Decimal`), nunca con coma flotante.
- Los montos de una cuenta están en la moneda de la cuenta. **Nunca se suman montos de monedas distintas.**
- Monedas de fábrica, todas con 2 decimales: `VES` (Bolívar), `USD` (Dólar estadounidense), `EUR` (Euro) y `USDT` (Tether). Las monedas propias se crean con `MonedaCreada`, que fija sus decimales para siempre.

### Validación (lectura estricta)

La aplicación rechaza **el archivo completo**, sin importar ningún evento, si:

- la primera línea no es un encabezado como el descrito;
- la versión no es la `1` (el mensaje dice qué versión tiene el archivo y cuál se soporta);
- la cantidad de eventos no coincide con `eventos=M`;
- alguna línea no es un evento bien formado (JSON inválido, una línea vacía, claves que faltan o sobran, `id` o `tipo` vacíos, `datos` que no es objeto ni texto) o repite un `id`;
- el archivo no es UTF-8 o no termina en `\n`.

Un `tipo` desconocido **no** invalida el archivo: al importarlo, ese evento se informa como rechazado con su motivo y el resto se aplica.

## Tipos de evento

Algunos campos tienen dos nombres por compatibilidad con bases de datos anteriores (por ejemplo `cuenta_id` e `id_cuenta`, con el mismo valor). Se indican como `a` / `b`: usa el primero que exista.

### `CuentaCreada`

| Campo | Significado |
|---|---|
| `id` | Identificador de la cuenta (no es el `id` del evento). |
| `nombre` | Nombre de la cuenta. |
| `tipo` | `efectivo`, `banco`, … |
| `moneda` | Código de la moneda de la cuenta. |
| `saldo_inicial` | Monto inicial, con los decimales de la moneda. Nunca cambia. |

### `MovimientoRegistrado`

| Campo | Significado |
|---|---|
| `id` / `id_movimiento` | Identificador del movimiento. |
| `cuenta_id` / `id_cuenta` | Cuenta a la que pertenece. |
| `tipo` | `ingreso` (suma) o `egreso` (resta). `gasto` significa lo mismo que `egreso`. |
| `tipo_dominio` | `ingreso` o `gasto` (informativo). |
| `monto` | Monto positivo, en la moneda de la cuenta. |
| `moneda` | Moneda de la cuenta (informativo). |
| `fecha` | Fecha del movimiento, `AAAA-MM-DD`. |
| `categoria_id` / `id_categoria` | Categoría. Las de fábrica tienen ids como `gasto:alimentacion` o `ingreso:salario` (en bases antiguas, `cat-fabrica-<tipo>-<nombre>`). |
| `nota` | Nota opcional. |

### `MovimientoCorregido`

Los movimientos nunca se editan ni se borran (INV-04): el movimiento original se queda en el libro con su monto, y la corrección **suma al saldo su `efecto`**, la diferencia entre el monto que tenía y el correcto. Si un movimiento se corrige varias veces, cada corrección trae el efecto respecto de la anterior, así que basta con sumarlos todos.

| Campo | Significado |
|---|---|
| `id_ajuste` | Identificador de la corrección. |
| `id_movimiento_original` | Movimiento que se corrige (siempre el original, aunque se corrija por segunda vez). |
| `monto_correcto` | Monto correcto del movimiento. |
| `efecto` | Lo que la corrección suma al saldo de la cuenta del movimiento original, con signo: `-1.50` si un gasto de 10.50 pasa a 12.00; `+5.00` si un gasto de 35.00 pasa a 30.00. |
| `moneda` | Moneda de la cuenta. |

Bases de datos anteriores pueden tener correcciones con los mismos campos que `MovimientoRegistrado` más `movimiento_original_id` y **sin `efecto`**: la aplicación actual no las cuenta en el saldo (efecto cero). Una corrección cuyo movimiento original no está en el archivo tampoco cuenta.

### `GastoDeGrupoAplicado`

Tu parte de un gasto de grupo, aplicada a una de tus cuentas. **Resta** del saldo como un gasto (categoría «Otros gastos»). El `id` del evento es el del evento del servidor de grupos, para no aplicar dos veces la misma parte.

| Campo | Significado |
|---|---|
| `id_evento_servidor` | Identificador del evento del grupo; es también el id del movimiento. |
| `id_cuenta` | Cuenta en la que se aplicó. |
| `monto` | Tu parte, en la moneda de la cuenta. |
| `moneda` | Moneda del gasto (la de la cuenta). |
| `fecha` | Fecha del gasto, `AAAA-MM-DD`. |
| `concepto` | «Grupo · Gasto». |
| `id_grupo`, `id_gasto` | Grupo y gasto de origen. |
| `incluye_redondeo` | `true` si tu parte absorbió el centavo de redondeo de la división. |

### `CategoriaCreada`

`id` / `id_categoria`, `nombre`, `tipo` (`ingreso` o `egreso`; en bases antiguas, `gasto`) y `es_fabrica` (`true` para las de fábrica). No cambia saldos.

### `MonedaCreada`

`codigo`, `nombre`, `simbolo` y `decimales` (número JSON entero). No cambia saldos, pero fija los decimales de esa moneda.

### `TasaRegistrada`

`id` (de la tasa), `moneda_origen`, `moneda_destino`, `valor` (texto decimal: 1 `moneda_origen` = `valor` `moneda_destino`), `fecha` (`AAAA-MM-DD`) y `fuente`. No cambia saldos.

### `TasaEliminada`

`tasa_id`: la tasa que se registró por error. Deja de usarse; no cambia saldos.

### `MonedaBaseCambiada`

`moneda`: la nueva moneda base para consolidar el patrimonio. No cambia saldos.

## Cómo reconstruir los saldos

1. Lee el encabezado y comprueba la versión y la cantidad de eventos.
2. Recorre los eventos:
   - `CuentaCreada`: anota la cuenta con su `saldo_inicial`.
   - `MovimientoRegistrado`: suma el `monto` si es `ingreso` y réstalo si es `egreso` (o `gasto`), en su cuenta.
   - `GastoDeGrupoAplicado`: resta el `monto` en `id_cuenta`.
   - `MovimientoCorregido`: suma el `efecto` (con su signo) en la cuenta del movimiento `id_movimiento_original`.
   - `MonedaCreada`: anota los decimales de la moneda.
   - El resto no cambia saldos.
3. Para cada cuenta (INV-05):

   ```text
   saldo = saldo_inicial + Σ ingresos − Σ gastos + Σ efectos de las correcciones
   ```

4. Muestra cada saldo con los decimales de la moneda de su cuenta. Para el total por moneda, suma los saldos de las cuentas de esa moneda; **nunca sumes monedas distintas** (para consolidar en la moneda base hacen falta las tasas, como en la pantalla de patrimonio).

## Ejemplo completo

Dos cuentas, un ingreso, un gasto corregido, una tasa y tu parte de un gasto de grupo (los ids son cortos para que se lean mejor; la aplicación usa UUID). Una prueba automatizada comprueba que este ejemplo es válido y que la aplicación y `reconstruir_saldos.py` obtienen los saldos de la tabla.

<!-- ejemplo-completo -->
```text
# cuentas-claras-respaldo version=1 exportado=2026-11-26T10:00:00Z eventos=7
{"id":"evt-01","tipo":"CuentaCreada","momento_registro":"2026-11-20T13:00:00Z","dispositivo_origen":"laptop-ana","datos":{"id":"cta-efectivo","moneda":"USD","nombre":"Efectivo","saldo_inicial":"100.00","tipo":"efectivo"}}
{"id":"evt-02","tipo":"CuentaCreada","momento_registro":"2026-11-20T13:01:00Z","dispositivo_origen":"laptop-ana","datos":{"id":"cta-banco","moneda":"VES","nombre":"Banco","saldo_inicial":"1500.00","tipo":"banco"}}
{"id":"evt-03","tipo":"MovimientoRegistrado","momento_registro":"2026-11-20T13:02:00Z","dispositivo_origen":"laptop-ana","datos":{"categoria_id":"ingreso:salario","cuenta_id":"cta-banco","fecha":"2026-11-20","id":"mov-quincena","id_categoria":"ingreso:salario","id_cuenta":"cta-banco","id_movimiento":"mov-quincena","moneda":"VES","monto":"4000.00","nota":"Quincena","tipo":"ingreso","tipo_dominio":"ingreso"}}
{"id":"evt-04","tipo":"MovimientoRegistrado","momento_registro":"2026-11-20T13:03:00Z","dispositivo_origen":"laptop-ana","datos":{"categoria_id":"gasto:alimentacion","cuenta_id":"cta-efectivo","fecha":"2026-11-20","id":"mov-almuerzo","id_categoria":"gasto:alimentacion","id_cuenta":"cta-efectivo","id_movimiento":"mov-almuerzo","moneda":"USD","monto":"10.50","nota":"Almuerzo","tipo":"egreso","tipo_dominio":"gasto"}}
{"id":"evt-05","tipo":"MovimientoCorregido","momento_registro":"2026-11-20T13:04:00Z","dispositivo_origen":"laptop-ana","datos":{"efecto":"-1.50","id_ajuste":"mov-almuerzo-corregido","id_movimiento_original":"mov-almuerzo","moneda":"USD","monto_correcto":"12.00"}}
{"id":"evt-06","tipo":"TasaRegistrada","momento_registro":"2026-11-20T13:05:00Z","dispositivo_origen":"laptop-ana","datos":{"fecha":"2026-11-20","fuente":"BCV","id":"tasa-usd-ves","moneda_destino":"VES","moneda_origen":"USD","valor":"36.5"}}
{"id":"srv-evt-0001","tipo":"GastoDeGrupoAplicado","momento_registro":"2026-11-21T09:00:00Z","dispositivo_origen":"telefono-ana","datos":{"concepto":"Viaje a Mérida · Hotel","fecha":"2026-11-21","id_cuenta":"cta-efectivo","id_evento_servidor":"srv-evt-0001","id_gasto":"gasto-1","id_grupo":"grupo-1","incluye_redondeo":false,"moneda":"USD","monto":"5.00"}}
```

Reconstrucción:

- **Efectivo** (USD): 100.00 − 10.50 (almuerzo) − 1.50 (efecto de su corrección a 12.00) − 5.00 (parte del grupo) = **83.00**.
- **Banco** (VES): 1500.00 + 4000.00 = **5500.00**.
- La tasa no cambia ningún saldo.

<!-- saldos-del-ejemplo -->
| Cuenta | Moneda | Saldo |
|---|---|---|
| Efectivo | USD | `83.00` |
| Banco | VES | `5500.00` |

## Versiones del formato

Un cambio que un lector de la versión 1 no entendería (un campo obligatorio nuevo en el encabezado o en la línea de evento, otra forma de escribir `datos`) aumenta la versión. Un **tipo de evento nuevo** no la aumenta: la aplicación lo rechaza evento por evento al importarlo; si cambia saldos, se documenta aquí y se actualiza `reconstruir_saldos.py`. La aplicación rechaza versiones que no conoce en lugar de leerlas a medias.
