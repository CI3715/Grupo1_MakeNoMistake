# ADR-014: Libro personal por usuario en el dispositivo

## Contexto

La revisión de las Entregas 5 y 6 señaló que, al cerrar la sesión en la
aplicación de escritorio e iniciar con otro usuario, se seguían viendo las
mismas cuentas. Ocurría porque el libro personal vive en un único archivo
SQLite (`cuentas_claras.db`) que no depende de quién tenga la sesión. Mientras
el libro no recibía nada del servidor esto era tolerable; con la Entrega 7 deja
de serlo, porque la sincronización escribe en el libro las partes de los gastos
de grupo del usuario con sesión. Con un solo archivo, las partes de beto
caerían en el libro donde están las de ana, y la cuenta que cada quien elige
para un grupo se mezclaría. Eso rompe el criterio de que cada participante
encuentre exactamente un movimiento por su parte en su propio libro (INV-15).

## Decisión

Cada usuario tiene su propio archivo SQLite, `cuentas_claras_<id>.db`, dentro
del directorio de datos de la aplicación. Se usa el id que devuelve
`GET /usuarios/yo` y no el nombre de usuario, que podría contener caracteres
que no conviene llevar a un nombre de archivo. El id se guarda en la sesión
del dispositivo y se valida antes de usarlo (solo letras, dígitos y guiones),
de modo que ninguna ruta pueda salirse de la carpeta de datos.

Las reglas de qué archivo se abre son estas:

- **Sin sesión** se abre `cuentas_claras.db`, el libro de siempre.
- **El primer usuario** que inicia sesión en un dispositivo donde todavía no
  existe ningún libro de usuario adopta el `cuentas_claras.db` existente (se
  renombra), para no perder lo hecho en las Entregas 2 a 4. Los usuarios
  siguientes empiezan con un libro vacío, y el libro sin sesión queda vacío
  tras la adopción.
- **Con la sesión vencida** (401) el libro del usuario sigue abierto: es la
  misma persona y lo personal debe seguir funcionando. Solo al cerrar sesión
  se vuelve al libro sin sesión.
- **Una sesión guardada antes de esta entrega** no trae id y se trata como sin
  sesión hasta que el usuario inicie sesión de nuevo.

El cambio de libro se hace en caliente, sin reiniciar la aplicación: al
arrancar con una sesión guardada, al iniciar sesión y al cerrarla. El libro
anterior se suelta antes de pedir la ruta nueva, porque la adopción renombra el
archivo y en Windows no se puede renombrar un archivo abierto. Si el libro
nuevo no se puede abrir, se reabre el anterior y se devuelve un error claro,
de modo que la aplicación nunca se queda sin libro.

El cursor de sincronización, la cuenta elegida para cada grupo, las partes
pendientes y la cola de envío viven dentro del archivo de cada usuario. Por eso
también son de cada usuario, sin ningún mecanismo adicional.

## Alternativas descartadas

**Agregar `usuario_id` a las tablas y a los eventos locales.** Se descartó
porque rompe la inmutabilidad de los eventos de las primeras cuatro entregas
(INV-04, INV-05): `CuentaCreada` y `MovimientoRegistrado` cambiarían de forma, y
los libros ya escritos dejarían de leerse con el mismo contrato. Obligaría a
reescribir el dominio, el codec, el proyector, los repositorios y las pruebas de
arquitectura, justo cuando más se necesita que lo ya probado no se toque.
Además mezclaría en un mismo libro datos de varias personas, cuando el libro
está pensado como personal, y todo lo que el servidor jamás debe saber (INV-16)
quedaría a una consulta mal escrita de distancia.

**Modelo de bóveda personal: una instalación, un libro, y quien quiera otro
usuario que use otra cuenta del sistema operativo.** Es un modelo válido para
una aplicación local-first, y es la alternativa de reserva si el cambio en
caliente no llega a tiempo. Se descartó como decisión principal porque la
sincronización de la Entrega 7 escribe en el libro, y porque obliga a explicar
antes de cualquier demostración que cambiar de usuario no cambia las cuentas.

## Consecuencias

- Cada usuario ve solo sus cuentas y movimientos, y al volver a iniciar sesión
  ve los suyos. No se modifica ningún evento existente ni su contrato.
- La separación es de organización, no de seguridad: los archivos no están
  cifrados, así que quien tenga acceso al disco puede leer el libro de otro
  usuario. Proteger el libro frente al sistema operativo queda fuera de esta
  entrega.
- La interfaz debe recargarse completa al iniciar o cerrar sesión para no dejar
  a la vista datos del usuario anterior.
- Limitación: el servidor entrega los montos con dos decimales, así que solo se
  ofrecen para un grupo las cuentas activas de su moneda cuando esa moneda usa
  dos decimales.
- Un usuario que cambia de dispositivo empieza con un libro vacío: la
  sincronización del libro entre dispositivos del mismo usuario está fuera de
  alcance de la Entrega 7.
