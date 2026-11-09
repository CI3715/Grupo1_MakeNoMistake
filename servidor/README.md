# Servidor - Cuentas Claras

Backend de Cuentas Claras, hecho con Python y FastAPI. Responde al ping
(Entrega 1) y, desde la Entrega 5, identifica a las personas: registro,
inicio de sesión con token firmado y perfil propio.

El servidor **nunca** recibe ni guarda cuentas, movimientos, saldos,
tasas ni categorías (INV-16): solo identidad. Las decisiones de la sesión
están en [ADR-010](../docs/adr/ADR-010-sesion-con-token-firmado.md).

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

## Configuración

El servidor se configura con variables de entorno:

| Variable | Obligatoria | Por defecto | Uso |
|---|---|---|---|
| `JWT_SECRET` | Sí | (ninguno) | Secreto para firmar los tokens. Al menos 32 caracteres. Sin él, el servidor no arranca. |
| `JWT_MINUTOS` | No | `60` | Vigencia del token en minutos. Usa `1` para la demostración del vencimiento. |
| `BD_USUARIOS_PATH` | No | `usuarios.db` | Archivo SQLite de usuarios (se crea solo). |

Nunca subas el secreto al repositorio. Para generar uno:

```bash
python -c "import secrets; print(secrets.token_urlsafe(48))"
```

## Ejecutar en modo desarrollo

```bash
   export JWT_SECRET="<el secreto generado>"
   uv run uvicorn main:app --reload --port 8000
```

En PowerShell: `$env:JWT_SECRET = "<el secreto generado>"`.

El servidor queda disponible en `http://localhost:8000`. La opción
`--reload` reinicia el servidor automáticamente al guardar cambios.
Para detenerlo, presiona `Ctrl + C`.

> El puerto es 8000 (y no 3000) para no chocar con Next.js, que usa el
> 3000 por defecto en desarrollo.

## Endpoints

| Ruta | Qué hace | Respuestas |
|---|---|---|
| `GET /ping` | Comprueba que el servidor está activo | 200 |
| `POST /usuarios` | Registro | 201, 400, 409 |
| `POST /sesiones` | Inicio de sesión: entrega el token | 200, 400, 401 |
| `GET /usuarios/yo` | Perfil propio (con `Authorization: Bearer <token>`) | 200, 401 |

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
levantar el servidor. `tests/conftest.py` pone un secreto de prueba y una
base SQLite temporal distinta para cada prueba.

- `test_reglas.py`: reglas de registro, sin HTTP ni base.
- `test_seguridad.py`: hash argon2id (sal única) y token (alterado,
  vencido, otro secreto, `alg: none`, sin secreto configurado).
- `test_registro.py`: `POST /usuarios` (201, 400 y 409 por campo; nunca
  repite la contraseña; nunca 422).
- `test_sesion.py`: `POST /sesiones` (mensaje idéntico, hash falso si el
  correo no existe) y `GET /usuarios/yo` (todos los casos de 401).

Para `/ping` (`test_main.py`) comprueban que:

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
├── main.py                # Aplicación FastAPI, rutas y manejador de errores 400
├── reglas.py              # Reglas de registro (funciones puras)
├── contrasenas.py         # Hash argon2id y hash falso
├── token_sesion.py        # Crear y verificar el JWT; lee JWT_SECRET y JWT_MINUTOS
├── bd.py                  # Tabla de usuarios en SQLite
├── openapi.yaml           # Contrato de la API, escrito a mano
├── tests/
│   ├── conftest.py        # Secreto de prueba y base temporal por prueba
│   ├── test_main.py       # /ping y documentación
│   ├── test_reglas.py
│   ├── test_seguridad.py
│   ├── test_registro.py
│   └── test_sesion.py
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
