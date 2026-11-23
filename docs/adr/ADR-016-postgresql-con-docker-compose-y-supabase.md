# ADR-016: PostgreSQL con Docker Compose y Supabase para el servidor

## Contexto

La ADR-009 fijó PostgreSQL 17 y `psycopg` 3 como base del servidor, con el
esquema creado al arrancar y la cadena de conexión en `CUENTAS_CLARAS_DB_URL`.
Para el desarrollo local se incluía un clúster propio levantado por
`scripts/base_datos_local.sh`, pensado para Linux y sin Docker.

La Entrega 7 añade dos necesidades nuevas:

- un PostgreSQL reproducible para desarrollo y pruebas que también funcione en
  Windows, donde el script de bash no es una opción cómoda (D-11);
- una base compartida por el equipo para probar el flujo completo y hacer la
  demostración, sin que las pruebas automáticas puedan borrar sus datos.

Además, el profesor pidió que todas las tablas de `public` queden protegidas con
Row Level Security (RLS), incluso las que se agreguen en el futuro.

## Decisión

Se mantiene **PostgreSQL 17** con **`psycopg` 3** y la creación idempotente del
esquema al arrancar (ADR-009). Se adopta una topología con tres formas de
conseguir la base:

1. **Docker Compose** para desarrollo y pruebas.
2. **Supabase** para el trabajo compartido y la demostración.
3. **PostgreSQL propio o el clúster local incluido** como alternativa sin
   Docker.

### Docker Compose

`servidor/docker-compose.yml` levanta PostgreSQL 17 en `127.0.0.1:55432`, el
mismo puerto que usaba el clúster local, para no chocar con un PostgreSQL del
sistema en el 5432.

- Usa un volumen con nombre (`cuentas-claras-postgres-datos`) para conservar los
  datos entre reinicios y `docker compose down`.
- Crea dos bases: `cuentas_claras` para desarrollo y `cuentas_claras_pruebas`
  para las pruebas de integración. La segunda se crea con
  `scripts/base_pruebas.sql` en la primera inicialización del volumen.
- El usuario y la contraseña se leen de `CUENTAS_CLARAS_DB_USUARIO` y
  `CUENTAS_CLARAS_DB_CLAVE`; no se escriben en el archivo. Si faltan, Compose
  falla al arrancar en lugar de crear un usuario vacío.
- Solo publica el puerto en `127.0.0.1`; la base no queda expuesta a la red.
- `scripts/base_datos_local.sh` se conserva como alternativa para quien no
  quiera o no pueda usar Docker.

### Supabase

El trabajo compartido y la demostración usan un proyecto de Supabase. El
servidor se conecta por la cadena del **Session pooler**:

- host `aws-<n>-<region>.pooler.supabase.com`, puerto `5432`, usuario
  `postgres.<referencia>`, base `postgres` y `sslmode=require`;
- **no** se usa la conexión directa, que solo ofrece IPv6;
- **no** se usa el Transaction pooler del puerto 6543, porque no admite los
  *prepared statements* que utiliza `psycopg`.

Supabase se usa solo como PostgreSQL alojado. El servidor no usa Supabase Auth,
ni la `anon key`, ni las librerías de Supabase: la sesión sigue siendo el token
JWT propio del servidor (ADR-010).

La cadena y la contraseña se guardan únicamente en `servidor/.env` y se
comparten por un canal privado. Nunca se escriben en el repositorio, en un issue
ni en el chat del equipo. `.env.example` solo contiene marcadores.

### RLS en todas las tablas

El esquema activa RLS en todas las tablas de `public` con sentencias
idempotentes. Si existen los roles `anon` y `authenticated` de Supabase, se les
revocan todos los permisos sobre esas tablas. El servidor se conecta con el rol
dueño del esquema, por lo que su acceso no cambia.

La prueba `servidor/tests/test_rls_postgres.py` recorre todas las tablas de
`public` y falla si alguna no tiene RLS; así también cubre las tablas que se
agreguen en entregas posteriores.

### Pruebas y secretos

- `CUENTAS_CLARAS_DB_URL` apunta a la base que usa el servidor.
- `CUENTAS_CLARAS_DB_URL_PRUEBAS` apunta siempre a una base local o al Compose.
  Las pruebas hacen `TRUNCATE`; nunca deben apuntar a Supabase.
- Si falta `CUENTAS_CLARAS_DB_URL_PRUEBAS`, las pruebas de integración se
  omiten y el resto de la suite sigue funcionando.
- El servidor se niega a arrancar si falta la URL o si el secreto JWT es
  demasiado corto, con un mensaje claro.
- `.env`, el volumen de Docker y el clúster local no se versionan.

## Alternativas consideradas

### Mantener solo el clúster local

Era suficiente en Linux, pero no resuelve el trabajo en Windows ni ofrece una
base común para el equipo. Se conserva como alternativa, no como única forma.

### Neon

Era el destino previsto en la ADR-009. Se descarta para esta entrega porque el
profesor pidió Supabase para el trabajo compartido y porque Docker Compose
cubre el desarrollo y las pruebas sin depender de la nube.

### Conexión directa a Supabase

Solo funciona por IPv6 en muchos entornos y no es la opción recomendada para
`psycopg` desde una red doméstica o universitaria. Se descarta.

### Transaction pooler de Supabase

El puerto 6543 no admite los *prepared statements* que usa `psycopg`. Se
descarta; el Session pooler mantiene una sesión estable por conexión.

### Supabase Auth

Habría acoplado la autenticación de la aplicación a un proveedor externo y
habría obligado a cambiar el token propio ya probado en la Entrega 5. No era
necesario: el servidor solo necesita PostgreSQL.

## Consecuencias

### Positivas

- El mismo motor se usa en desarrollo, pruebas, Supabase y el despliegue.
- Docker Compose da un entorno reproducible y una alternativa viable en
  Windows, lo que ayuda a cerrar D-11.
- Las pruebas tienen una base separada y una guarda explícita contra Supabase.
- RLS cubre todas las tablas de `public`, incluidas las nuevas.
- No se agregan credenciales ni secretos al repositorio.

### Negativas / deuda a considerar

- Docker Compose requiere Docker instalado; el clúster local sigue siendo la
  alternativa sin Docker.
- Supabase en plan gratuito puede pausarse por inactividad y hay que abrirlo
  antes de una demostración.
- La cadena del Session pooler y la contraseña deben tratarse como secretos.
- La ejecución de las pruebas contra PostgreSQL en Windows y en CI sigue
  pendiente de verificarse en una máquina con Docker; esta ADR no la declara
  resuelta por sí sola.
- El esquema se sigue creando con `CREATE ... IF NOT EXISTS`; cuando el modelo
  necesite migraciones habrá que añadir un mecanismo específico.

## Implementación

- `servidor/docker-compose.yml`: PostgreSQL 17, volumen y dos bases.
- `servidor/scripts/base_pruebas.sql`: crea `cuentas_claras_pruebas`.
- `servidor/.env.example`: tres opciones de conexión y variables de desarrollo.
- `servidor/README.md`: pasos para Docker Compose, Supabase y PostgreSQL propio.
- `servidor/esquema.py`: tablas de la Entrega 7, RLS y revocación de permisos.
- `servidor/tests/test_rls_postgres.py`: comprobación de RLS en todas las
  tablas de `public`.
- `servidor/configuracion.py`: validación de la URL y del secreto al arrancar.

## Estado

Aceptada. Sustituye las partes de la ADR-009 relativas a Neon y a que el clúster
local era la única forma de desarrollo. Mantiene PostgreSQL 17, `psycopg` 3 y
la creación idempotente del esquema.

## Fecha

2026-11-23
