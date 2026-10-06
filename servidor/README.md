# Servidor - Cuentas Claras

Backend de Cuentas Claras, hecho con Python y FastAPI. En la Entrega 1
expone un único endpoint, `GET /ping`, para verificar la comunicación
con la aplicación cliente.

## Prerrequisitos

- Python 3.10 o superior
- Git

Para comprobar la versión de Python:

```bash
python --version
```

## Instalación

Todos los comandos se ejecutan desde la carpeta `servidor/`.

1. Entrar a la carpeta:

```bash
   cd servidor
```

2. Crear el entorno virtual:

```bash
   python -m venv .venv
```

3. Activar el entorno virtual:

   - Windows (PowerShell):

```powershell
     .venv\Scripts\activate
```

     Si PowerShell bloquea la ejecución de scripts, ejecuta una vez
     `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` y vuelve a
     intentarlo.

   - macOS / Linux:

```bash
     source .venv/bin/activate
```

   Al activarse, la terminal muestra `(.venv)` al inicio de la línea.

4. Instalar las dependencias:

```bash
   pip install -r requirements.txt
```

   Si además vas a ejecutar las pruebas y las herramientas de calidad,
   instala en su lugar las dependencias de desarrollo (incluyen las
   anteriores):

```bash
   pip install -r requirements-dev.txt
```

## Ejecutar en modo desarrollo

```bash
uvicorn main:app --reload --port 8000
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

Todos los comandos se ejecutan desde la carpeta `servidor/`, con el
entorno virtual activado y las dependencias de desarrollo instaladas
(`pip install -r requirements-dev.txt`).

Antes de abrir un pull request deben pasar sin errores las cinco
comprobaciones siguientes.

**1. Pruebas automatizadas**

```bash
pytest
```

**2. Linter** (revisa errores y malas prácticas en el código)

```bash
ruff check .
```

**3. Formato** (comprueba el estilo sin modificar archivos)

```bash
ruff format --check .
```

Si falla, aplica el formato automáticamente con:

```bash
ruff format .
```

**4. Tipos** (modo estricto)

```bash
mypy
```

**5. Vulnerabilidades en las dependencias** (necesita conexión a internet)

```bash
pip-audit -r requirements-dev.txt
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
Las dependencias de desarrollo están separadas en
`requirements-dev.txt` para que `requirements.txt` solo contenga lo que
necesita el servidor en producción. Las decisiones detrás de estas
herramientas están en
[ADR-003](../docs/adr/ADR-003-herramientas-de-calidad-servidor.md).

## Estructura

```
servidor/
├── main.py                # Aplicación FastAPI y endpoint /ping
├── tests/
│   └── test_main.py       # Pruebas automatizadas de /ping
├── pyproject.toml         # Configuración de pytest, ruff y mypy
├── requirements.txt       # Dependencias de Python
├── requirements-dev.txt   # Dependencias de desarrollo y calidad
└── README.md
```

## Problemas comunes

- **`uvicorn` no se reconoce como comando:** el entorno virtual no está
  activado, o las dependencias no se instalaron. Repite los pasos 3 y 4.
- **El puerto 8000 está ocupado:** cierra el proceso que lo usa o inicia
  el servidor con otro puerto (`--port 8001`). Si cambias el puerto,
  avisa al equipo para actualizar la URL en la aplicación cliente.
- **`pytest`, `ruff` o `mypy` no se reconocen como comando:** instalaste
  solo `requirements.txt`. Ejecuta
  `pip install -r requirements-dev.txt`.
- **`ModuleNotFoundError: No module named 'main'` al correr las
  pruebas:** ejecuta `pytest` desde la carpeta `servidor/`, donde está
  el `pyproject.toml`.
- **`The starlette.testclient module requires the httpx2 package`:**
  falta `httpx2`. Instala las dependencias de desarrollo como se indica
  arriba.
