# Servidor - Cuentas Claras

Backend de Cuentas Claras, hecho con Python y FastAPI. Responde al ping
(Entrega 1) y, desde la Entrega 5, identifica a las personas: registro,
inicio de sesión con token firmado y perfil propio.

El servidor **nunca** recibe ni guarda cuentas, movimientos, saldos,
tasas ni categorías (INV-16): solo identidad. Las decisiones de la sesión
están en [ADR-010](../docs/adr/ADR-010-sesion-con-token-firmado.md).

## Prerrequisitos

- [uv](https://docs.astral.sh/uv/) 0.12 o superior
- PostgreSQL 17 (puedes usar el clúster local incluido, que necesita los
  binarios de PostgreSQL instalados)
- Git

`uv` se encarga de instalar y gestionar la versión de Python que
necesita el proyecto (3.13 o superior), así que no hace falta instalar
Python por separado. Para comprobar la versión de `uv`:

```bash
uv --version
```

## Instalación

Todos los comandos se ejecutan desde la carpeta `servidor/`.

1. Entrar a la carpeta:

```bash
   cd servidor
```

2. Sincronizar el entorno y las dependencias:

```bash
   uv sync
```

   Este comando crea el entorno virtual en `.venv`, instala las
   dependencias de producción y las de desarrollo (grupo `dev`, definido
   en `pyproject.toml`), y deja todo listo a partir de `uv.lock`.

   Si solo quieres las dependencias de producción:

```bash
   uv sync --no-dev
```

   No es necesario activar el entorno virtual: los comandos de `uv`
   (por ejemplo `uv run`) lo usan automáticamente.

## Configuración

El servidor se configura con variables de entorno, normalmente en
`servidor/.env`. Las obligatorias son `CUENTAS_CLARAS_DB_URL` (PostgreSQL) y
`CUENTAS_CLARAS_SECRETO_JWT`; la lista completa y cómo crear el `.env` están
en «Base de datos y configuración», más abajo.

`JWT_SECRET` y `JWT_MINUTOS`, los nombres anteriores, se siguen aceptando para
firmar el token y, si están definidos, tienen prioridad sobre
`CUENTAS_CLARAS_SECRETO_JWT` y `CUENTAS_CLARAS_MINUTOS_TOKEN`. Aun así,
`CUENTAS_CLARAS_SECRETO_JWT` es obligatoria para arrancar.

## Ejecutar en modo desarrollo

```bash
   bash scripts/base_datos_local.sh iniciar   # si usas el clúster local
   uv run uvicorn main:app --reload --port 8000
```

El servidor queda disponible en `http://localhost:8000`. La opción
`--reload` reinicia el servidor automáticamente al guardar cambios.
Para detenerlo, presiona `Ctrl + C`.

> El puerto es 8000 (y no 3000) para no chocar con Next.js, que usa el
> 3000 por defecto en desarrollo.
>
> La primera vez necesitas configurar PostgreSQL y el archivo `.env`; lo
> explica la sección «Base de datos y configuración» más abajo.

## Base de datos y configuración

El servidor usa PostgreSQL 17 y crea la tabla `usuarios` al arrancar. Toda la
configuración se lee de variables de entorno; el archivo `servidor/.env` no se
sube a Git.

### Dónde se colocan las variables

El archivo debe estar en **`servidor/.env`**, junto a `main.py`, porque
`configuracion.py` carga exactamente esa ruta. No sirve ponerlo en la raíz del
repositorio ni dentro de `app/`: el servidor no lo leería.

1. Copia la plantilla y ajusta los valores:

```bash
   cp .env.example .env
   chmod 600 .env
```

2. Si no tienes PostgreSQL configurado, levanta el clúster local incluido.
   Crea la base de desarrollo y la de pruebas en el puerto 55432, sin Docker
   ni permisos de superusuario:

```bash
   bash scripts/base_datos_local.sh iniciar
```

   Para detenerlo:

```bash
   bash scripts/base_datos_local.sh detener
```

3. Arranca el servidor:

```bash
   uv run uvicorn main:app --reload --port 8000
```

### Variables de entorno

| Variable | Obligatoria | Valor por defecto | Para qué sirve |
|---|---|---|---|
| `CUENTAS_CLARAS_DB_URL` | Sí | — | Cadena de conexión a PostgreSQL (`postgresql://usuario:clave@host:puerto/base`). |
| `CUENTAS_CLARAS_SECRETO_JWT` | Sí | — | Secreto para firmar los JWT. Mínimo 32 caracteres. |
| `CUENTAS_CLARAS_MINUTOS_TOKEN` | No | `60` | Vigencia del token en minutos. |
| `CUENTAS_CLARAS_DB_URL_PRUEBAS` | No | — | Base para las pruebas de integración del repositorio. Si falta, se omiten. |
| `CUENTAS_CLARAS_DB_USUARIO` | Solo script local | — | Usuario con el que el script crea el rol del clúster local. |
| `CUENTAS_CLARAS_DB_CLAVE` | Solo script local | — | Clave de ese usuario. |

#### `CUENTAS_CLARAS_DB_URL` (obligatoria)

Indica a qué PostgreSQL se conecta el servidor. Formato:

```text
postgresql://usuario:clave@host:puerto/base
```

Con el clúster local incluido:

```env
CUENTAS_CLARAS_DB_URL=postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras
```

Si usas un PostgreSQL propio en el puerto habitual, cambia el puerto a `5432`
y el resto de datos de conexión. Si esta variable falta o está vacía, el
servidor no arranca.

#### `CUENTAS_CLARAS_DB_USUARIO` y `CUENTAS_CLARAS_DB_CLAVE` (solo script local)

El servidor no las lee. Las usa `scripts/base_datos_local.sh` para crear el
rol y las bases del clúster local. Deben coincidir con el usuario y la clave
de `CUENTAS_CLARAS_DB_URL`.

El script las busca primero en el entorno y, si no están, en `servidor/.env`.
Si faltan, se detiene con un mensaje claro en lugar de usar credenciales
hardcodeadas.

#### `CUENTAS_CLARAS_SECRETO_JWT` (obligatoria)

Secreto con el que se firman y verifican los tokens de sesión. No es la
contraseña de una persona usuaria: es una clave privada del servidor.

- Mínimo **32 caracteres**.
- Debe ser distinto en desarrollo y en un despliegue real.
- No se comparte ni se sube a Git.

Genera uno con:

```bash
python -c "import secrets; print(secrets.token_urlsafe(48))"
```

#### `CUENTAS_CLARAS_MINUTOS_TOKEN` (opcional)

Vigencia de los tokens en minutos. Se expresa como número entero positivo. Si
se omite, vale `60`:

```env
CUENTAS_CLARAS_MINUTOS_TOKEN=60
```

#### `CUENTAS_CLARAS_DB_URL_PRUEBAS` (opcional, solo pruebas)

Base que usan las pruebas de integración del repositorio. Apunta a una base
separada, porque las pruebas vacían la tabla `usuarios`:

```env
CUENTAS_CLARAS_DB_URL_PRUEBAS=postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras_pruebas
```

Si no la defines, `uv run pytest` sigue funcionando y las pruebas de
integración se omiten.

### Ejemplo de `.env` local

```env
CUENTAS_CLARAS_DB_URL=postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras
CUENTAS_CLARAS_DB_URL_PRUEBAS=postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras_pruebas
CUENTAS_CLARAS_DB_USUARIO=usuario
CUENTAS_CLARAS_DB_CLAVE=clave
CUENTAS_CLARAS_SECRETO_JWT=<secreto-generado-de-al-menos-32-caracteres>
CUENTAS_CLARAS_MINUTOS_TOKEN=60
```

### Cómo arrancar el servidor con estas variables

No hace falta exportar nada si usas `servidor/.env`: `configuracion.py` lo
carga automáticamente al iniciar. El orden es:

```bash
cd servidor
cp .env.example .env      # solo la primera vez; después edita .env
chmod 600 .env
bash scripts/base_datos_local.sh iniciar
uv run uvicorn main:app --reload --port 8000
```

El script del clúster local lee `CUENTAS_CLARAS_DB_USUARIO` y
`CUENTAS_CLARAS_DB_CLAVE` de `.env` o del entorno, y crea el rol con esas
credenciales. Deben coincidir con las de `CUENTAS_CLARAS_DB_URL`; si faltan,
el script se detiene con un mensaje claro.

Las variables exportadas en la terminal tienen prioridad sobre `.env`, porque
`load_dotenv` no sobrescribe las que ya existen. Por ejemplo, para ejecutar
solo las pruebas contra otra base:

```bash
CUENTAS_CLARAS_DB_URL_PRUEBAS=postgresql://... uv run pytest
```

En un despliegue no se usa `.env`: se configuran las mismas variables en el
panel del proveedor o en el entorno del proceso. La cadena de conexión cambia
a la del PostgreSQL de producción (por ejemplo, Neon) y el secreto debe ser
propio de ese entorno.

> Si falta `CUENTAS_CLARAS_DB_URL` o el secreto es corto, el servidor no
> arranca. Es intencional: el error aparece al iniciar y no en la primera
> petición.

## Endpoints

| Ruta | Qué hace | Respuestas |
|---|---|---|
| `GET /ping` | Comprueba que el servidor está activo | 200 |
| `POST /usuarios` | Registro | 201, 400, 409 |
| `POST /sesiones` | Inicio de sesión: entrega el token | 200, 400, 401 |
| `GET /usuarios/yo` | Perfil propio (con `Authorization: Bearer <token>`) | 200, 401 |
| `GET /grupos/{id}` | Miembros, gastos y balances del grupo (solo miembros aceptados) | 200, 401, 404 |
| `POST /grupos/{id}/gastos` | Registra un gasto: el servidor lo divide y lo guarda con sus partes | 201, 400, 401, 404 |

El contrato completo está en [`openapi.yaml`](openapi.yaml). Resumen:

- **Registro**: `nombre_usuario` de 3 a 30 caracteres (minúsculas,
  dígitos, punto y guion bajo), `nombre_visible` no vacío (máximo 50),
  `correo` válido (se guarda en minúsculas) y `contrasena` de 10 o más
  caracteres. Los errores informan todos los campos a la vez:
  `{"errores": [{"campo": ..., "mensaje": ...}]}`. La contraseña se guarda
  como hash argon2id y nunca aparece en ninguna respuesta.
- **Errores**: nunca se responde 422. Un campo con tipo incorrecto o un
  cuerpo que no es JSON también da 400 por campo, sin repetir el valor
  enviado.
- **Inicio de sesión**: devuelve `{"token", "tipo": "Bearer", "vence_en"}`.
  Si el correo no existe o la contraseña no coincide, la respuesta es la
  misma: 401 con `{"mensaje": "Correo o contraseña incorrectos."}`.
- **Perfil**: sin token, con token alterado, vencido o firmado con otro
  secreto, responde 401 con `{"mensaje": "Sesión no válida o vencida."}`.

- **Gastos de grupo** (Entrega 6, [ADR-013](../docs/adr/ADR-013-division-balances-y-autorizacion-de-grupo.md)):
  el cuerpo lleva `concepto`, `monto` (texto, `"100.00"`), `fecha`
  (`AAAA-MM-DD`), `pagador`, `regla` (`partes_iguales`, `porcentajes` o
  `montos_fijos`) y `participantes` (`[{"usuario", "valor"}]`). El servidor
  trunca cada parte a céntimos y reparte el residuo de a un céntimo en orden
  alfabético: 100,00 entre ana, beto y carla da 33,34 (con redondeo), 33,33 y
  33,33. Si los porcentajes o los montos no cuadran, responde 400 con la
  diferencia exacta: «Los porcentajes suman 95; faltan 5.».
- **Balances**: `GET /grupos/{id}` devuelve `balance = pago − le_corresponde`
  por miembro; la suma es siempre `0.00`.
- **404 uniforme**: grupo inexistente, identificador mal formado, persona
  ajena e invitado sin aceptar reciben el mismo `404`
  `{"mensaje": "Grupo no encontrado."}`.

Ejemplo completo con `curl`:

```bash
curl -X POST localhost:8000/usuarios -H "Content-Type: application/json" \
  -d '{"nombre_usuario":"ana.perez","nombre_visible":"Ana","correo":"ana@example.com","contrasena":"una-clave-larga-123"}'
curl -X POST localhost:8000/sesiones -H "Content-Type: application/json" \
  -d '{"correo":"ana@example.com","contrasena":"una-clave-larga-123"}'
curl localhost:8000/usuarios/yo -H "Authorization: Bearer <token>"
```

### `GET /ping`

Comprueba que el servidor está activo.

- Código de respuesta: `200 OK`
- Respuesta:

```json
{
  "estado": "ok",
  "mensaje": "Servidor Cuentas Claras activo",
  "version": "0.1.0",
  "timestamp": "2026-10-05T10:00:00Z"
}
```

El campo `timestamp` está en UTC con formato ISO 8601.

Para probarlo desde la terminal:

```bash
curl http://localhost:8000/ping
```

En PowerShell, usa `curl.exe` en lugar de `curl`. También se puede
abrir `http://localhost:8000/ping` directamente en el navegador.

### Endpoints del contrato de la Entrega 5

El contrato completo está en [`openapi.yaml`](openapi.yaml). Además de `/ping`,
define estos endpoints para la identidad de las personas usuarias:

| Método y ruta | Qué hace | Respuestas principales |
|---|---|---|
| `POST /usuarios` | Registro. Crea la persona usuaria y devuelve sus datos sin contraseña ni hash. | `201`; `400` si los datos no son válidos; `409` si el correo o el nombre de usuario ya existen. |
| `POST /sesiones` | Inicio de sesión. Con credenciales válidas entrega un JWT (`Authorization: Bearer <token>`). | `200`; `400` si falta correo o contraseña; `401` si las credenciales no son correctas. |
| `GET /usuarios/yo` | Perfil de la persona autenticada. Requiere el token en la cabecera `Authorization`. | `200`; `401` si el token falta, es inválido o está vencido. |

La vigencia del token la fija `CUENTAS_CLARAS_MINUTOS_TOKEN` y el secreto con
el que se firma es `CUENTAS_CLARAS_SECRETO_JWT`.

Forma de los errores:

- `400` y `409`: `{"errores": [{"campo": "...", "mensaje": "..."}]}`.
- `401`: `{"mensaje": "..."}`.
- Los mensajes nunca repiten el valor enviado ni incluyen la contraseña.

> **Estado de implementación:** hoy `main.py` solo expone `/ping`. Los tres
> endpoints de identidad están definidos en el contrato y tienen el repositorio
> de PostgreSQL preparado, pero todavía no están conectados. Cuando se
> implementen, esta sección y `/openapi.json` deben coincidir con
> `openapi.yaml`.

## HTTP en local y HTTPS en despliegue (RNF-06)

En desarrollo se usa **HTTP** porque el servidor escucha únicamente en la
máquina local (`http://localhost:8000`) y el tráfico no sale de ella:

```bash
uv run uvicorn main:app --reload --port 8000
```

**Cualquier despliegue exige HTTPS** (RNF-06), incluido un entorno de pruebas
accesible por red. El correo, la contraseña y el token de sesión nunca deben
viajar por HTTP sin cifrar.

- La aplicación cliente debe apuntar a una URL `https://...`, no a
  `http://...`.
- `uvicorn` por sí solo no termina TLS. En producción se coloca detrás de un
  proxy con certificado (Caddy, Nginx) o se usa el TLS que ofrece la plataforma
  de despliegue.
- No se permite contenido mixto: si la interfaz se sirve por HTTPS, todas las
  llamadas al servidor también deben ser HTTPS.
- El certificado debe ser válido y estar vigente; una redirección permanente
  de HTTP a HTTPS en el borde es la forma habitual de cerrar la entrada sin
  cifrar.

## Documentación interactiva

FastAPI genera la documentación de la API automáticamente:

- Swagger UI: `http://localhost:8000/docs`
- OpenAPI (JSON): `http://localhost:8000/openapi.json`

## Pruebas y calidad

Todos los comandos se ejecutan desde la carpeta `servidor/`, con las
dependencias de desarrollo instaladas (`uv sync`).

Antes de abrir un pull request deben pasar sin errores las cinco
comprobaciones siguientes.

**1. Pruebas automatizadas**

```bash
uv run pytest
```

**2. Linter** (revisa errores y malas prácticas en el código)

```bash
uv run ruff check .
```

**3. Formato** (comprueba el estilo sin modificar archivos)

```bash
uv run ruff format --check .
```

Si falla, aplica el formato automáticamente con:

```bash
uv run ruff format .
```

**4. Tipos** (modo estricto)

```bash
uv run mypy
```

**5. Vulnerabilidades en las dependencias** (necesita conexión a internet)

```bash
uv audit
```

### Qué verifican las pruebas

Las pruebas usan el `TestClient` de FastAPI, así que no hace falta
levantar el servidor. `tests/conftest.py` pone un secreto de prueba y da a
cada prueba un almacén de usuarios en memoria nuevo
(`tests/repositorio_en_memoria.py`), que imita al repositorio de PostgreSQL.
Así las pruebas de las rutas corren sin base de datos.

- `test_reglas.py`: reglas de registro, sin HTTP ni base.
- `test_seguridad.py`: hash argon2id (sal única) y token (alterado,
  vencido, otro secreto, `alg: none`, sin secreto configurado).
- `test_registro.py`: `POST /usuarios` (201, 400 y 409 por campo; nunca
  repite la contraseña; nunca 422).
- `test_sesion.py`: `POST /sesiones` (mensaje idéntico, hash falso si el
  correo no existe) y `GET /usuarios/yo` (todos los casos de 401).
- `test_vectores_division.py`: `docs/vectores-division.json` es coherente y
  `division.dividir` y `division.calcular_balances` dan exactamente sus
  números (los mismos que comprueba la app en Rust).
- `test_gastos_grupo.py`: `POST /grupos/{id}/gastos` y `GET /grupos/{id}`
  (división, mensajes de «faltan»/«sobran», errores por campo, 404 uniforme,
  balances del ejemplo 2.5 y el flujo completo como usuario nuevo).
- `test_autorizacion.py`: la decisión de pertenencia, sin HTTP.

Para `/ping` (`test_main.py`) comprueban que:

- responde `200 OK` y con contenido JSON;
- contiene exactamente los campos `estado`, `mensaje`, `version` y
  `timestamp`;
- `estado`, `mensaje` y `version` tienen los valores esperados;
- `timestamp` tiene el formato `AAAA-MM-DDThh:mm:ssZ` (UTC) y
  corresponde a la hora actual;
- `POST /ping` es rechazado con `405` (el endpoint es de solo lectura);
- `/docs` y `/openapi.json` están disponibles y documentan `/ping`.

Además, `tests/test_configuracion.py` comprueba que el servidor rechaza una
configuración incompleta o inválida, y `tests/test_repositorio_usuarios.py`
prueba el repositorio contra PostgreSQL: correo en minúsculas, búsqueda sin
distinguir mayúsculas, duplicados por campo y creación idempotente del esquema.
`tests/test_repositorio_amistades.py`, `test_repositorio_grupos.py` y
`test_repositorio_gastos.py` prueban directamente los repositorios de la
Entrega 6: unicidad de la pareja en los dos sentidos, invitaciones de grupo,
gasto y partes en una sola transacción, montos `NUMERIC(14, 2)` sin `float`,
errores por campo/regla y que eliminar una amistad no borra grupos ni gastos.
`tests/test_integracion_postgres.py` arranca la app conectada a PostgreSQL y
recorre registro, inicio de sesión, perfil y el flujo de grupo con los gastos
del ejemplo 2.5 (montos `NUMERIC`, regla canónica y nada guardado si el gasto
falla): fecha en UTC aunque la base use
otra zona horaria, 409 con todos los campos repetidos, duplicado detectado por
la restricción `UNIQUE` y que la base solo tenga las seis tablas permitidas
(INV-16). Todas las pruebas de PostgreSQL se omiten si no defines
`CUENTAS_CLARAS_DB_URL_PRUEBAS`, para que `uv run pytest` funcione en un clon
limpio sin base de datos.

Con el clúster local, ejecútalas así:

```bash
   bash scripts/base_datos_local.sh iniciar
   CUENTAS_CLARAS_DB_URL_PRUEBAS=postgresql://usuario:clave@127.0.0.1:55432/cuentas_claras_pruebas uv run pytest
```

### Configuración

La configuración de `pytest`, `ruff` y `mypy` está en `pyproject.toml`.
Las dependencias de ejecución están en `[project.dependencies]` y las
de desarrollo en el grupo `dev` de `[dependency-groups]`, también en
`pyproject.toml`, con todas las versiones fijadas y resueltas en
`uv.lock`. Las decisiones detrás de estas herramientas están en
[ADR-003](../docs/adr/ADR-003-herramientas-de-calidad-servidor.md).

## Estructura
```
servidor/
├── main.py                            # Aplicación FastAPI, rutas y manejo de errores
├── configuracion.py                   # Lectura y validación de variables de entorno
├── base_datos.py                      # Conexión a PostgreSQL y creación del esquema
├── esquema.py                         # DDL de las tablas e índices (idempotente)
├── repositorio_usuarios.py            # Consultas de la tabla usuarios
├── repositorio_amistades.py           # Amistades: invitación, respuesta y baja (Entrega 6)
├── repositorio_grupos.py              # Grupos y miembros: invitar, aceptar, rechazar (Entrega 6)
├── repositorio_gastos.py              # Gastos y partes con montos NUMERIC exactos (Entrega 6)
├── division.py                        # División exacta y balances (funciones puras, Entrega 6)
├── autorizacion.py                    # Miembro aceptado o el mismo 404 (Entrega 6)
├── reglas.py                          # Reglas de registro (funciones puras)
├── contrasenas.py                     # Hash argon2id y hash falso
├── token_sesion.py                    # Creación y verificación del JWT
├── scripts/
│   └── base_datos_local.sh            # Clúster PostgreSQL local para desarrollo
├── tests/
│   ├── conftest.py                    # Fixtures: secreto de prueba y almacén en memoria
│   ├── repositorio_en_memoria.py      # Almacén de usuarios falso para las rutas
│   ├── test_main.py                   # /ping y documentación
│   ├── test_configuracion.py          # Variables de entorno
│   ├── test_repositorio_usuarios.py   # PostgreSQL (se omiten sin CUENTAS_CLARAS_DB_URL_PRUEBAS)
│   ├── test_repositorio_amistades.py  # Amistades contra PostgreSQL (ídem)
│   ├── test_repositorio_grupos.py     # Grupos y miembros contra PostgreSQL (ídem)
│   ├── test_repositorio_gastos.py     # Gastos, partes y NUMERIC contra PostgreSQL (ídem)
│   ├── apoyo_compartido.py            # Utilidades de las pruebas de la Entrega 6
│   ├── test_integracion_postgres.py   # Rutas contra PostgreSQL (ídem)
│   ├── test_reglas.py                 # Reglas de registro
│   ├── test_seguridad.py              # Hash, token, arranque e INV-16
│   ├── test_registro.py               # Registro de usuarios
│   ├── test_sesion.py                 # Inicio de sesión y JWT
│   ├── flujo_http.py                  # Pasos del flujo social por HTTP (memoria y PostgreSQL)
│   ├── test_vectores_division.py      # División y balances contra docs/vectores-division.json
│   ├── test_gastos_grupo.py           # Gastos, detalle del grupo y flujo completo
│   └── test_autorizacion.py           # Pertenencia y 404 uniforme
├── .env                               # Configuración local (no se versiona)
├── .env.example                       # Plantilla de configuración
├── .postgres/                         # Clúster PostgreSQL local generado (no se versiona)
├── openapi.yaml                       # Contrato de la API
├── pyproject.toml                     # Dependencias y configuración de pytest, ruff y mypy
├── uv.lock                            # Versiones fijadas de todas las dependencias
├── .python-version                    # Versión de Python que usa uv
└── README.md
```
## Problemas comunes

- **`uvicorn` no se reconoce como comando:** usa siempre
  `uv run uvicorn ...`, o ejecuta `uv sync` para volver a sincronizar el
  entorno.
- **Faltan `pytest`, `ruff` o `mypy`:** no sincronizaste las
  dependencias de desarrollo. Ejecuta `uv sync` (sin `--no-dev`).
- **El puerto 8000 está ocupado:** cierra el proceso que lo usa o inicia
  el servidor con otro puerto (`--port 8001`). Si cambias el puerto,
  avisa al equipo para actualizar la URL en la aplicación cliente.
- **`ModuleNotFoundError: No module named 'main'` al correr las
  pruebas:** ejecuta `uv run pytest` desde la carpeta `servidor/`, donde
  está el `pyproject.toml`.
- **`The starlette.testclient module requires the httpx2 package`:**
  falta `httpx2`. Ejecuta `uv sync` para instalar las dependencias de
  desarrollo.
