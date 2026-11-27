# ADR-018: Puerto `TransporteSincronizacion` y sus tres adaptadores (falso, carpeta y Git)

## Contexto

En la Entrega 8 el usuario sincroniza su libro entre sus propios dispositivos a
través de un destino que él elige y controla: un repositorio Git o una carpeta
que otro servicio mantiene sincronizada (Syncthing, Dropbox, Google Drive)
(RF-INT-03, RF-INT-07). El servidor de Cuentas Claras no participa (INV-16,
RNF-01).

Hacen falta tres cosas a la vez:

1. **Que agregar un destino nuevo no toque el dominio ni los casos de uso**
   (RNF-15). El caso de uso `sincronizar` (traer, fusionar, enviar, publicar) no
   debe saber si habla con Git, con una carpeta o con la memoria.
2. **Que todos los destinos se comporten igual** y que eso se compruebe con las
   mismas pruebas (RNF-14).
3. **Que dos dispositivos nunca escriban el mismo archivo**, para que Git no
   produzca conflictos de fusión y los servicios de carpetas no generen «copias
   en conflicto».

Además, la sincronización debe seguir funcionando sin conexión sin perder nada
(RNF-17) y transferir solo lo que falta (RNF-10).

## Decisión

### 1. Un puerto de lotes opacos, declarado en la capa de aplicación

El puerto vive en `aplicacion::transporte` (la Guía Técnica, sección 2.3, exige
que los puertos se declaren en la capa de aplicación; `dominio` no se toca).
Transporta **lotes**: secuencias opacas de bytes, ya cifradas por quien llama
(ADR-017), identificadas por `(dispositivo, lote)`. El puerto no sabe qué es un
evento, un libro, Git ni una carpeta.

```text
trait TransporteSincronizacion: Send
  dispositivo_local() -> &str
  traer()                                  // paso 1: fetch en Git; comprueba la carpeta
  listar_dispositivos() -> Vec<String>
  listar_lotes(dispositivo) -> Vec<String> // solo nombres, en orden de texto
  leer_lote(dispositivo, lote) -> Vec<u8>
  escribir_lote(lote, contenido)           // paso 3: solo en la carpeta propia
  publicar()                               // paso 4: commit y push en Git
```

`Send` como el resto de los puertos, para poder vivir dentro del `Arc<Mutex<>>`
del cableado de Tauri.

### 2. Reglas del contrato

1. **Cada dispositivo escribe solo en su propia carpeta.** El transporte se
   crea para un dispositivo y `escribir_lote` solo escribe ahí.
2. **Un lote es inmutable.** Reescribir un nombre existente falla con
   `LoteYaExiste` y no cambia nada. Un lote nunca se borra ni se reescribe.
3. **Todo o nada.** Si `escribir_lote` falla no queda un lote a medias, y un
   reintento posterior con el mismo nombre funciona.
4. **`publicar` es el paso que hace visible un lote** para los demás; `traer` es
   el que lo recibe. Un adaptador puede mostrarlo antes (la carpeta lo hace al
   instante), pero quien usa el puerto no cuenta con ello.
5. **Los fallos de red no pierden nada.** Sin conexión o con el destino
   inaccesible, lo escrito se conserva y `publicar` se reintenta en la siguiente
   sincronización. `SinConexion`, `DestinoInaccesible` y `DestinoCambio` son
   reintentables; `CredencialesRechazadas` no lo es (hay que actualizarlas).
6. **Solo se transfiere lo que falta.** `listar_lotes` devuelve nombres y el
   contenido se pide lote por lote; `lotes_por_leer` devuelve solo los que el
   dispositivo todavía no aplicó. Los lotes propios se marcan como leídos al
   escribirlos.
7. **Los nombres son seguros.** Dispositivos y lotes usan solo letras ASCII,
   números, `-` y `_` (`validar_nombre`): ningún nombre se sale de la carpeta del
   destino ni se confunde con un temporal (`.tmp`) o una copia en conflicto.
8. **Nombres de lote ordenables.** `nombre_de_lote` produce
   `20261124T120000Z-0001`: el orden de texto coincide con el orden de tiempo.

Los mensajes de `ErrorTransporte` están escritos para mostrarse tal cual al
usuario (RNF-18): dicen qué ocurrió y qué hacer, sin trazas.

### 3. Tres adaptadores, una sola batería de contrato

| Adaptador | Dónde | Para qué |
|---|---|---|
| `TransporteEnMemoria` / `DestinoEnMemoria` | `aplicacion::transporte_memoria` | Adaptador falso. Varios dispositivos sobre un mismo destino, con los fallos de la sección 2.7 (sin conexión, destino inaccesible, credenciales rechazadas, destino que cambia al publicar). Se comporta como Git: lo escrito solo lo ven otros tras `publicar` + `traer`. |
| `TransporteCarpeta` | `infraestructura::transporte_carpeta` | Una carpeta compartida. Un archivo `<lote>.lote` por lote, en la carpeta del dispositivo; se escribe con nombre temporal (`.lote.tmp`), se fuerza a disco y se renombra. Al leer se ignoran temporales, copias en conflicto y archivos ajenos. |
| `TransporteGit` | `infraestructura::transporte_git` | Un repositorio Git del usuario usando el programa `git`. Escribir confirma en la copia local sin red; publicar empuja y, si el remoto cambió, trae, aplica `rebase` y reintenta. Las credenciales vienen del almacén de secretos y nunca se escriben en la copia local ni en la línea de órdenes. |

Los tres pasan `aplicacion::contrato_transporte::bateria_contrato_transporte`,
que cubre: conocer su dispositivo; escribir y leer un lote (incluido uno vacío);
listar en orden; dos dispositivos que no se pisan aunque usen el mismo nombre de
lote; un lote escrito que no se modifica; `publicar` y `traer` idempotentes;
que solo llegan y solo se leen los lotes nuevos; lote inexistente; nombres
peligrosos; y destino inaccesible que conserva lo pendiente.

### 4. Cómo agregar un destino nuevo (RNF-15)

1. Implementar `TransporteSincronizacion` en `infraestructura` (por ejemplo,
   `transporte_webdav.rs`).
2. En su archivo de pruebas, implementar `BancoDeDestino` (conectar un
   dispositivo, dejar el destino inaccesible y restablecerlo) y llamar a
   `bateria_contrato_transporte`.
3. Agregar el tipo a `transporte_de` en `src/sincronizacion_destino.rs` y a la
   interfaz de configuración.

No se modifica `dominio`, `aplicacion::sincronizar_libro` ni el puerto.

### 5. Prueba de extremo a extremo

`infraestructura/tests/dos_instalaciones_extremo_a_extremo.rs` reproduce el flujo
de la sección 2.8 con dos instalaciones reales (carpeta de datos y SQLite propios,
cifrado real) contra una carpeta y contra un repositorio Git, en los dos órdenes
de sincronización (ESC-03, ESC-04). `scripts/sync-test.sh` la ejecuta.

## Alternativas consideradas

- **Que el puerto trabaje con eventos en vez de bytes.** Obligaría a cada
  adaptador a conocer el formato del libro y el cifrado, y un cambio de formato
  tocaría tres adaptadores. Con lotes opacos el cifrado y el formato viven en un
  solo lugar.
- **Una sola carpeta o archivo compartido que todos modifican.** Es lo más
  simple, pero dos dispositivos escribirían el mismo archivo: Git daría
  conflictos de fusión y las carpetas sincronizadas, «copias en conflicto».
- **Fusionar los lotes en el destino** (compactarlos). Reescribiría archivos y
  rompería la inmutabilidad; queda fuera de alcance de la entrega.
- **`git2` en lugar del programa `git`.** Evita depender de un programa externo,
  pero exige compilar bibliotecas nativas (`libgit2`, OpenSSL) en las tres
  plataformas, incluido Android, y no respeta por sí sola el proxy, los
  certificados ni SSH del usuario. La Guía admite ambas opciones.
- **Probar cada adaptador con sus propias pruebas.** Permite que se desvíen. La
  batería común hace que cambiar de adaptador sea cambiar de adaptador y nada más
  (RNF-14).

## Consecuencias

### Positivas

- Agregar un destino es escribir un adaptador y pasar la batería de contrato.
- El caso de uso `sincronizar` se prueba entero con el adaptador falso, con
  fallos reproducibles, sin red ni disco.
- Sin conflictos de fusión ni copias en conflicto por construcción: cada archivo
  tiene un único escritor y nunca se modifica.
- Sincronizar sin novedades no escribe ni lee lotes (RNF-10).

### Negativas / deuda

- Los lotes se acumulan en el destino (uno por sincronización con novedades) y
  cada dispositivo nuevo los lee todos. La compactación queda para una entrega
  posterior.
- El adaptador Git necesita el programa `git` instalado en el equipo; en Android
  hay que valorar `git2` o empaquetarlo.
- La carpeta compartida depende de que el servicio externo respete el renombrado
  atómico y no entregue archivos a medio sincronizar; los lotes terminados solo
  se reconocen por su extensión `.lote`.

## Estado

Aceptada.

## Fecha

2026-11-26
