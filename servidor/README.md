# Servidor - Cuentas Claras

Backend de Cuentas Claras, hecho con Python y FastAPI. En la Entrega 1
expone un único endpoint, `GET /ping`, para verificar la comunicación
con la aplicación cliente.

## Prerrequisitos

- [uv](https://docs.astral.sh/uv/) 0.12 o superior
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
├── main.py                # Aplicación FastAPI y endpoint /ping
├── tests/
│   └── test_main.py       # Pruebas automatizadas de /ping
├── pyproject.toml         # Dependencias y configuración de pytest, ruff y mypy
├── uv.lock                # Versiones fijadas de todas las dependencias
├── .python-version        # Versión de Python que usa uv
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
