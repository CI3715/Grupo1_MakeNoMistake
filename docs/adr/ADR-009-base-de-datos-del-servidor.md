# ADR-009: Base de datos del servidor con PostgreSQL

## Contexto

La Entrega 5 introduce el registro y el inicio de sesión de personas usuarias
en el servidor. Hasta ahora el servidor solo exponía `GET /ping` y no tenía
persistencia. La aplicación de escritorio sigue siendo *local-first*: sus
cuentas, movimientos, saldos, categorías y tasas viven en SQLite dentro del
dispositivo (ADR-006) y nunca se envían al servidor (INV-16, RES-02).

El servidor, por tanto, solo necesita guardar identidad: id, nombre de usuario,
nombre visible, correo, hash de la contraseña y fecha de alta. Hoy el proyecto
se ejecuta en local, pero el servidor está pensado para que varias personas lo
usen y para desplegarse más adelante en Neon. Esa previsión es la que condiciona
la elección del motor.

También hay que resolver dos requisitos concretos:

- la conexión debe salir de una variable de entorno;
- la unicidad del correo no debe distinguir mayúsculas, y un duplicado debe
  indicar si falla el nombre de usuario o el correo.

## Decisión

Se usa **PostgreSQL 17** como base de datos del servidor, con el driver
**`psycopg` 3** en modo síncrono.

- El esquema se crea al arrancar la aplicación con sentencias idempotentes
  (`CREATE TABLE IF NOT EXISTS` y `CREATE UNIQUE INDEX IF NOT EXISTS`).
- La cadena de conexión se lee de `CUENTAS_CLARAS_DB_URL`. Si falta, el
  servidor no arranca.
- El correo se guarda en minúsculas y además existe un índice único sobre
  `lower(correo)`, de modo que la base también rechaza duplicados escritos con
  otras mayúsculas.
- Las violaciones de unicidad se traducen a excepciones propias
  (`CorreoYaRegistrado` y `NombreUsuarioYaRegistrado`) que indican el campo.
- Para el desarrollo local se incluye un clúster PostgreSQL propio en
  `servidor/.postgres`, levantado con `scripts/base_datos_local.sh`. No requiere
  Docker ni permisos de superusuario y usa el puerto 55432 para no chocar con
  un PostgreSQL del sistema.

SQLite sigue siendo la base de la **aplicación** (ADR-006). Esta ADR solo
decide la base del **servidor**.

## Alternativas consideradas

### SQLite para el servidor

Era la opción más sencilla para un servidor que hoy corre en una sola máquina:
cero configuración, `sqlite3` viene con Python y las pruebas no necesitan un
servicio externo. Se descartó por tres razones:

1. **Misma base en desarrollo y despliegue.** El destino previsto es Neon, que
   es PostgreSQL. Empezar con SQLite obligaría a reescribir el repositorio, los
   tipos y las pruebas al migrar, y a mantener dos dialectos mientras tanto.
2. **Concurrencia y unicidad.** SQLite serializa las escrituras y su
   `COLLATE NOCASE` no es portable. PostgreSQL ofrece concurrencia real y un
   índice funcional sobre `lower(correo)` que expresa la regla directamente en
   la base.
3. **Despliegue sin disco persistente.** En plataformas como Vercel el sistema
   de archivos es efímero y puede haber varias instancias. Un archivo SQLite se
   perdería entre invocaciones y no se compartiría entre instancias.

Se reconoce que, si el servidor fuera a quedarse para siempre en una sola
máquina y con una sola persona, SQLite habría sido suficiente. La decisión es
explícitamente previsora: se paga un poco de infraestructura local ahora para
no pagar una migración después.

### PostgreSQL autogestionado en un servidor propio

Implicaría administrar backups, actualizaciones, usuarios y disponibilidad. No
aporta nada frente a un PostgreSQL local para desarrollo ni frente a Neon para
producción.

### Usar Neon desde el primer día

Válido para producción, pero innecesario para el desarrollo: obliga a tener red
y a compartir credenciales. El clúster local permite trabajar sin conexión y sin
depender de un servicio externo. La migración a Neon se reduce a cambiar
`CUENTAS_CLARAS_DB_URL`.

### SQLAlchemy u otro ORM

Para una sola tabla añade dependencias y complejidad de tipado sin una ventaja
clara. `psycopg` con SQL parametrizado es suficiente y deja el SQL a la vista.

## Consecuencias

### Positivas

- El mismo motor se usa en desarrollo, pruebas de integración y el futuro
  despliegue; no hay dialectos que conciliar.
- La unicidad del correo sin distinguir mayúsculas queda garantizada por la
  base, no solo por el código de la aplicación.
- Los duplicados se identifican por campo sin filtrar detalles del driver hacia
  los comandos.
- La migración a Neon solo cambia la cadena de conexión.
- No se debilita el *local-first*: el servidor sigue sin recibir datos
  financieros.

### Negativas / deuda a considerar

- El desarrollo local necesita un proceso PostgreSQL. Se mitiga con
  `scripts/base_datos_local.sh`, pero sigue siendo más pesado que un archivo.
- El arranque falla si la base no responde o falta configuración. Es deliberado:
  preferimos un error claro al iniciar antes que un fallo en la primera
  petición.
- El esquema se crea con `CREATE ... IF NOT EXISTS`. Cuando el modelo cambie
  hará falta un mecanismo de migraciones; para la Entrega 5 la tabla es única y
  el esquema es estable.
- El servidor abre una conexión por operación. Con el volumen actual es
  suficiente; si crece, se añadirá un pool.

## Implementación

- `servidor/configuracion.py`: lee y valida las variables de entorno.
- `servidor/base_datos.py`: abre conexiones y crea la tabla y los índices.
- `servidor/repositorio_usuarios.py`: inserta, busca por correo, por nombre de
  usuario y por id, y traduce los duplicados.
- `servidor/main.py`: en el ciclo de vida de FastAPI, crea el esquema y deja el
  repositorio en `app.state`.

Variables de entorno:

| Variable | Obligatoria | Valor por defecto |
|---|---|---|
| `CUENTAS_CLARAS_DB_URL` | Sí | — |
| `CUENTAS_CLARAS_SECRETO_JWT` | Sí (mínimo 32 caracteres) | — |
| `CUENTAS_CLARAS_MINUTOS_TOKEN` | No | `60` |
| `CUENTAS_CLARAS_DB_URL_PRUEBAS` | No (solo pruebas) | — |

Tabla resultante:

```sql
CREATE TABLE usuarios (
    id              UUID PRIMARY KEY,
    nombre_usuario  VARCHAR(30) NOT NULL,
    nombre_visible  VARCHAR(50) NOT NULL,
    correo          VARCHAR(254) NOT NULL,
    hash_contrasena TEXT NOT NULL,
    fecha_alta      TIMESTAMPTZ NOT NULL,
    CONSTRAINT usuarios_correo_en_minusculas CHECK (correo = lower(correo)),
    CONSTRAINT usuarios_nombre_en_minusculas
        CHECK (nombre_usuario = lower(nombre_usuario))
);

CREATE UNIQUE INDEX usuarios_correo_unico ON usuarios (lower(correo));
CREATE UNIQUE INDEX usuarios_nombre_usuario_unico ON usuarios (nombre_usuario);
```

## Estado

Aceptado (final). Sustituye el punto 3 de ADR-001, que dejaba la base de datos
del servidor «por definir».

## Fecha

2026-11-09
