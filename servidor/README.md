# Servidor - Cuentas Claras

Backend de Cuentas Claras, hecho con Python y FastAPI. En la Entrega 1
expone un único endpoint, `GET /ping`, para verificar la comunicación
con la aplicación cliente.

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

## Ejecutar en modo desarrollo

```bash
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

El servidor usa PostgreSQL 17 y crea la tabla `usuarios` al arrancar. La
conexión se lee de variables de entorno; el archivo `servidor/.env` no se
sube a Git.

1. Copia el ejemplo y ajusta los valores:

```bash
   cp .env.example .env
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

Genera el secreto con:

```bash
python -c "import secrets; print(secrets.token_urlsafe(48))"
```

> Si falta `CUENTAS_CLARAS_DB_URL` o el secreto es corto, el servidor no
> arranca. Es intencional: el error aparece al iniciar y no en la primera
> petición.

## Endpoint

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

Las pruebas están en `tests/test_main.py` y usan el `TestClient` de
FastAPI, así que no hace falta levantar el servidor. Para `/ping`
comprueban que:

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
Estas últimas se omiten si no defines `CUENTAS_CLARAS_DB_URL_PRUEBAS`, para que
`uv run pytest` funcione en un clon limpio sin base de datos.

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
├── main.py                     # Aplicación FastAPI y endpoint /ping
├── configuracion.py            # Lectura y validación de variables de entorno
├── base_datos.py               # Conexión a PostgreSQL y creación del esquema
├── repositorio_usuarios.py     # Consultas de la tabla usuarios
├── scripts/
│   └── base_datos_local.sh     # Clúster PostgreSQL local para desarrollo
├── tests/
│   ├── test_main.py            # Pruebas automatizadas de /ping
│   ├── test_configuracion.py   # Pruebas de las variables de entorno
│   └── test_repositorio_usuarios.py  # Pruebas de PostgreSQL (opcionales)
├── .env.example                # Plantilla de configuración
├── openapi.yaml                # Contrato de la API
├── pyproject.toml              # Dependencias y configuración de pytest, ruff y mypy
├── uv.lock                     # Versiones fijadas de todas las dependencias
├── .python-version             # Versión de Python que usa uv
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
