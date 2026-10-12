# Cuentas Claras

Asistente de finanzas personales multimoneda, **local-first**, con gastos compartidos. Permite llevar el control del dinero repartido entre varias cuentas y monedas (efectivo, banco, billetera digital) y, más adelante, compartir gastos con otras personas sin entregar la información financiera a un tercero.

Proyecto del laboratorio del **Grupo 1 — MakeNoMistake**.

## Idea central

El sistema tiene dos piezas desplegables, y el reparto de datos entre ellas es la decisión de diseño más importante:

| Pieza | Qué hace | Qué guarda |
|---|---|---|
| **Aplicación** (`app/`) | Programa de escritorio (y Android, más adelante) que el usuario instala. | Todo el libro personal: cuentas, saldos y movimientos, **solo en el dispositivo**. |
| **Servidor** (`servidor/`) | Servicio HTTP. | Únicamente lo que otras personas necesitan ver: identidad, amistades, solicitudes y grupos. |

El servidor **nunca** recibe ni almacena cuentas, saldos, movimientos ni categorías personales (INV-16, RES-02). Por eso la aplicación funciona completa sin conexión y sin cuenta registrada (RF-SOC-03, RNF-03).

## Estado del proyecto

| Entrega | Contenido | Estado |
|---|---|---|
| 1 | Ambiente de desarrollo: aplicación Tauri + Next.js y servidor FastAPI con `GET /ping`. | Terminada |
| 2 | Crear y guardar cuentas en el dispositivo (SQLite embebido, sin internet). | En desarrollo |

## Estructura del repositorio

```
.
├── app/            # Aplicación: interfaz Next.js + núcleo en Rust (Tauri 2)
├── servidor/       # Servidor: Python + FastAPI
├── docs/
│   └── adr/        # Registros de decisiones de arquitectura (ADR)
├── LICENSE.md      # Apache License 2.0
└── README.md
```

Cada carpeta tiene su propio README con los detalles:

- [`app/README.md`](app/README.md): prerrequisitos, instalación, ejecución en desarrollo, estructura y comprobaciones previas a un pull request.
- [`servidor/README.md`](servidor/README.md): instalación con `uv`, endpoints, pruebas y calidad.
- [`docs/adr/ADR.md`](docs/adr/ADR.md): índice de las decisiones de arquitectura.

## Tecnologías

| Pieza | Tecnología |
|---|---|
| Empaquetado de la aplicación | Tauri 2 |
| Núcleo de la aplicación | Rust |
| Interfaz | Next.js (React + TypeScript) con exportación estática, Tailwind |
| Almacenamiento local | SQLite embebido vía `rusqlite` (Entrega 2) |
| Dinero | `rust_decimal`: nunca coma flotante |
| Servidor | Python + FastAPI, gestionado con `uv` |

Las razones de cada elección están en las ADR: [ADR-001](docs/adr/ADR-001-tecnologias-y-arquitectura.md), [ADR-002](docs/adr/ADR-002-lenguaje-servidor.md), [ADR-003](docs/adr/ADR-003-herramientas-de-calidad-servidor.md) y [ADR-004](docs/adr/ADR-004-stack-aplicacion.md).

## Arranque rápido

Necesitas Git, Rust, Node.js y las bibliotecas del sistema de Tauri para la aplicación, y [`uv`](https://docs.astral.sh/uv/) para el servidor. Los pasos de instalación de cada herramienta están en los README de `app/` y `servidor/`.

```bash
git clone https://github.com/CI3715/Grupo1_MakeNoMistake.git
cd Grupo1_MakeNoMistake
```

**Aplicación** (desde `app/`):

```bash
cd app
npm install
npm run tauri dev
```

La primera compilación de Rust tarda varios minutos; las siguientes, segundos.

**Servidor** (desde `servidor/`, en otra terminal; solo hace falta para la comprobación de conexión de la Entrega 1):

```bash
cd servidor
uv sync
uv run uvicorn main:app --reload --port 8000
```

Comprobación: `http://localhost:8000/ping` debe responder `{"estado": "ok", ...}`.

## Pruebas y calidad

| Qué | Comando | Desde |
|---|---|---|
| Pruebas del núcleo de Rust | `cargo test --workspace` | `app/src-tauri/` |
| Formato, linter y auditoría de Rust | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo audit` | `app/src-tauri/` |
| Linter y auditoría de la interfaz | `npm run lint`, `npm audit` | `app/` |
| Pruebas del servidor | `uv run pytest` | `servidor/` |
| Calidad del servidor | `uv run ruff check .`, `uv run ruff format --check .`, `uv run mypy`, `uv audit` | `servidor/` |

Antes de abrir un pull request deben pasar todas las comprobaciones de la pieza que se modificó.

## Datos locales

Las cuentas se guardan únicamente en un archivo SQLite dentro del directorio de datos de la aplicación (`app_data_dir` de Tauri), sin ningún proceso servidor adicional y sin enviar nada a internet. La ruta exacta del archivo en cada sistema operativo se documenta aquí al cerrar la Entrega 2.

## Cómo contribuir

- Todo cambio entra por pull request, con al menos una revisión de otra persona. Las ramas principales están protegidas.
- Los commits llevan la firma del DCO: `git commit -s`.
- Antes de abrir el pull request, ejecuta las comprobaciones de la sección anterior.
- Las decisiones de diseño no triviales se registran como ADR en `docs/adr/`. Una ADR no se edita cuando cambia la decisión: se escribe otra que la sustituye.

## Licencia

[Apache License 2.0](LICENSE.md). Todas las piezas del sistema comparten la misma licencia.
