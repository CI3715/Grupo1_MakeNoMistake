# Cuentas Claras

Asistente de finanzas personales multimoneda, **local-first**, con gastos compartidos. Permite llevar el control del dinero repartido entre varias cuentas y monedas (efectivo, banco, billetera digital) y compartir gastos con otras personas sin entregar la información financiera a un tercero.

Proyecto del laboratorio del **Grupo 1 — MakeNoMistake**.

## Idea central

El sistema tiene dos piezas desplegables, y el reparto de datos entre ellas es la decisión de diseño más importante:

| Pieza | Qué hace | Qué guarda |
|---|---|---|
| **Aplicación** (`app/`) | Programa de escritorio (y Android, más adelante) que el usuario instala. | Todo el libro personal: cuentas, saldos, movimientos, tasas y categorías, **solo en el dispositivo**. |
| **Servidor** (`servidor/`) | Servicio HTTP. | Únicamente lo que otras personas necesitan ver: identidad, amistades, invitaciones, grupos y los gastos compartidos con sus partes. |

El servidor **nunca** recibe ni almacena cuentas, saldos, movimientos, tasas ni categorías personales (INV-16, RES-02). Guarda el gasto compartido (concepto, monto, pagador y cuánto le corresponde a cada participante) porque eso lo ven todos los miembros del grupo, pero no las cuentas ni los movimientos con los que cada persona paga su parte. Por eso la aplicación funciona completa sin conexión y sin cuenta registrada (RF-SOC-03, RNF-03).

## Estado del proyecto

| Entrega | Contenido | Estado |
|---|---|---|
| 1 | Ambiente de desarrollo: aplicación Tauri + Next.js y servidor FastAPI con `GET /ping`. | Terminada |
| 2 | Crear y guardar cuentas en el dispositivo (SQLite embebido, sin internet). | Terminada |
| 3 | Registrar gastos e ingresos, corregirlos con ajustes y ver saldos, todo en el dispositivo. | Terminada |
| 4 | Monedas propias, tasas de cambio con fecha y fuente, moneda base y patrimonio consolidado, todo en el dispositivo. | Terminada |
| 5 | Registro e inicio de sesión opcionales contra el servidor; el token se guarda en el almacén de secretos del dispositivo. | Terminada |
| 6 | Amigos, grupos y gastos compartidos: rutas y persistencia en PostgreSQL, división exacta y balances. | Terminada |

## Estructura del repositorio

```
.
├── app/            # Aplicación: interfaz Next.js + núcleo en Rust (Tauri 2)
├── servidor/       # Servidor: Python + FastAPI + PostgreSQL
├── docs/
│   ├── adr/        # Registros de decisiones de arquitectura (ADR)
│   └── vectores-division.json   # Casos compartidos de división y balances
├── verificar-entrega*.sh        # Listas de comprobación de calidad
├── LICENSE.md      # Apache License 2.0
└── README.md
```

Cada carpeta tiene su propio README con los detalles:

- [`app/README.md`](app/README.md): prerrequisitos, instalación, ejecución en desarrollo, estructura y comprobaciones previas a un pull request.
- [`servidor/README.md`](servidor/README.md): instalación con `uv`, PostgreSQL, variables de entorno, rutas, sesión, pruebas y calidad.
- [`docs/adr/ADR.md`](docs/adr/ADR.md): índice de las decisiones de arquitectura.

## Tecnologías

| Pieza | Tecnología |
|---|---|
| Empaquetado de la aplicación | Tauri 2 |
| Núcleo de la aplicación | Rust |
| Interfaz | Next.js (React + TypeScript) con exportación estática, Tailwind |
| Almacenamiento local | SQLite embebido vía `rusqlite` |
| Dinero en la aplicación | `rust_decimal`: nunca coma flotante |
| Servidor | Python + FastAPI, gestionado con `uv` |
| Base de datos del servidor | PostgreSQL 17 |
| Identidad y sesión | Hash argon2id + token JWT firmado |
| Dinero en el servidor | `NUMERIC(14, 2)`; en JSON viaja como texto (`"100.00"`) |

Las razones de cada elección están en las ADR: [ADR-001](docs/adr/ADR-001-tecnologias-y-arquitectura.md), [ADR-002](docs/adr/ADR-002-lenguaje-servidor.md), [ADR-003](docs/adr/ADR-003-herramientas-de-calidad-servidor.md), [ADR-004](docs/adr/ADR-004-stack-aplicacion.md), [ADR-009](docs/adr/ADR-009-base-de-datos-del-servidor.md), [ADR-010](docs/adr/ADR-010-sesion-con-token-firmado.md), [ADR-011](docs/adr/ADR-011-datos-compartidos-de-grupos.md), [ADR-012](docs/adr/ADR-012-montos-numeric-y-repositorios-compartidos.md) y [ADR-013](docs/adr/ADR-013-division-balances-y-autorizacion-de-grupo.md).

## Arranque rápido

Necesitas Git, Rust, Node.js y las bibliotecas del sistema de Tauri para la aplicación, [`uv`](https://docs.astral.sh/uv/) para el servidor y PostgreSQL 17 (o el clúster local incluido). Los pasos de instalación de cada herramienta están en los README de `app/` y `servidor/`.

```bash
git clone https://github.com/CI3715/Grupo1_MakeNoMistake.git
cd Grupo1_MakeNoMistake
```

**1. Servidor** (desde `servidor/`). Hace falta para el registro, el inicio de sesión y las funciones sociales; el libro personal no lo necesita.

```bash
cd servidor
cp .env.example .env
chmod 600 .env
```

Edita `servidor/.env` y define al menos `CUENTAS_CLARAS_DB_URL` y `CUENTAS_CLARAS_SECRETO_JWT` (mínimo 32 caracteres). Para generar el secreto:

```bash
python -c "import secrets; print(secrets.token_urlsafe(48))"
```

Si no tienes PostgreSQL, levanta el clúster local incluido (crea la base de desarrollo y la de pruebas en el puerto 55432, sin Docker ni permisos de superusuario):

```bash
bash scripts/base_datos_local.sh iniciar
uv sync
uv run uvicorn main:app --reload --port 8000
```

Comprobación: `http://localhost:8000/ping` debe responder `{"estado": "ok", ...}`.

**2. Aplicación** (desde `app/`, en otra terminal):

```bash
cd app
npm install
npm run tauri dev
```

La primera compilación de Rust tarda varios minutos; las siguientes, segundos. La aplicación busca el servidor en `http://localhost:8000`; si lo levantaste en otra dirección, define `CUENTAS_CLARAS_URL` antes de arrancarla.

> Si solo vas a usar el libro personal (cuentas, movimientos, tasas y patrimonio), puedes saltarte el servidor: la aplicación abre y funciona sin él.

## Pruebas y calidad

| Qué | Comando | Desde |
|---|---|---|
| Pruebas del núcleo de Rust (todas) | `cargo test --workspace` | `app/src-tauri/` |
| Solo reglas de negocio y casos de uso (repositorio en memoria, sin base de datos ni red) | `cargo test -p dominio -p aplicacion` | `app/src-tauri/` |
| Persistencia SQLite | `cargo test -p infraestructura` | `app/src-tauri/` |
| Reglas de dependencia entre capas | `cargo test --test arquitectura` | `app/src-tauri/` |
| Tipos y compilación de la interfaz | `npx tsc --noEmit`, `npm run build` | `app/` |
| Formato, linter y auditoría de Rust | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo audit` | `app/src-tauri/` |
| Linter y auditoría de la interfaz | `npm run lint`, `npm audit` | `app/` |
| Pruebas del servidor | `uv run pytest` | `servidor/` |
| Calidad del servidor | `uv run ruff check .`, `uv run ruff format --check .`, `uv run mypy`, `uv audit` | `servidor/` |

`uv run pytest` funciona sin PostgreSQL: las pruebas de integración se omiten. Si defines `CUENTAS_CLARAS_DB_URL_PRUEBAS`, se ejecutan además contra esa base. Las pruebas de Rust y de Python comparten los casos de `docs/vectores-division.json` para garantizar que ambos lados reparten los gastos igual.

Los scripts `verificar-entrega*.sh` de la raíz ejecutan una lista completa de comprobaciones y dejan la salida en el `.log` correspondiente.

Antes de abrir un pull request deben pasar todas las comprobaciones de la pieza que se modificó.

## Datos locales

Las cuentas se guardan únicamente en un archivo SQLite dentro del directorio de datos de la aplicación (`app_data_dir` de Tauri), sin ningún proceso servidor adicional y sin enviar nada a internet. El archivo se llama `cuentas_claras.db` y vive en la carpeta de datos de la aplicación, cuyo nombre es el identificador `com.cuentasclaras.desktop`:

| Sistema | Ruta del archivo |
|---|---|
| Linux | `~/.local/share/com.cuentasclaras.desktop/cuentas_claras.db` |
| macOS | `~/Library/Application Support/com.cuentasclaras.desktop/cuentas_claras.db` |
| Windows | `%APPDATA%\com.cuentasclaras.desktop\cuentas_claras.db` |

La ruta de Linux está comprobada; las de macOS y Windows siguen la convención de Tauri 2. Al arrancar en modo desarrollo, la aplicación escribe la ruta real en el registro (`Base de datos local lista en ...`). Junto al archivo pueden aparecer `cuentas_claras.db-wal` y `cuentas_claras.db-shm`, que son parte normal de SQLite. Para empezar de cero basta con cerrar la aplicación y borrar esos archivos.

Desde la Entrega 3 el mismo archivo guarda también los movimientos y las categorías. Los movimientos son inmutables: nunca se modifican ni se eliminan, y una corrección se registra como un ajuste que referencia al original. El saldo siempre se calcula a partir de los movimientos. El ejemplo del enunciado (1.000,00 → 1.800,00 → 1.300,00 → 1.750,00) se reproduce paso a paso, tanto en la aplicación como con una prueba automatizada, en `app/README.md`, sección «Movimientos, saldos y el ejemplo 2.2».

Desde la Entrega 4 el mismo archivo guarda también las tasas de cambio, la moneda base y las monedas propias, como eventos inmutables (eliminar una tasa es un evento nuevo, no un borrado). Los montos se guardan como enteros en la unidad menor de cada moneda y las tasas como enteros por 10^8, sin coma flotante, y nada de esto se envía al servidor. El patrimonio consolidado nunca suma monedas distintas sin una tasa con fecha: si falta una, muestra los subtotales por moneda y nombra el par que falta. El ejemplo del enunciado (316.100,00 VES; con base USD falta la tasa USDT a USD y al registrarla el total es 350,00 USD) se reproduce paso a paso, en la aplicación y con una prueba automatizada, en `app/README.md`, sección «Monedas, tasas de cambio y patrimonio (Entrega 4)».

Desde la Entrega 5 el token de sesión no se guarda en SQLite ni en texto plano: vive en el almacén de secretos del sistema operativo y se borra al cerrar sesión. El registro y el inicio de sesión son opcionales; sin sesión, todo lo personal sigue funcionando.

### Datos del servidor

El servidor guarda su información en PostgreSQL, en **seis tablas**: `usuarios`, `amistades`, `grupos`, `miembros`, `gastos` y `partes_gasto`. No hay tablas ni columnas de cuentas, movimientos, saldos, tasas, categorías ni moneda base (INV-16). El esquema se crea al arrancar con sentencias idempotentes y no borra datos existentes.

La configuración vive en `servidor/.env`, que no se sube a Git. El clúster local incluido guarda sus archivos en `servidor/.postgres`, también ignorado por Git. Para empezar de cero en el servidor hay que detener el clúster y borrar esa carpeta, o usar otra base.

## Cómo contribuir

- Todo cambio entra por pull request, con al menos una revisión de otra persona. Las ramas principales están protegidas.
- Los commits llevan la firma del DCO: `git commit -s`.
- Antes de abrir el pull request, ejecuta las comprobaciones de la sección anterior.
- Las decisiones de diseño no triviales se registran como ADR en `docs/adr/`. Una ADR no se edita cuando cambia la decisión: se escribe otra que la sustituye.

## Licencia

[Apache License 2.0](LICENSE.md). Todas las piezas del sistema comparten la misma licencia.
