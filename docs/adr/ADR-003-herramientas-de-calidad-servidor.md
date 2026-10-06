# ADR-003: Herramientas de pruebas y calidad del servidor

## Contexto
La ADR-002 definió el servidor con Python y FastAPI. La Guía Técnica
exige, para un servidor en Python, `mypy --strict` (sección 1.2) y
establece como obligatorios un formateador, un linter y una herramienta
de auditoría de dependencias. Para estos últimos nombra herramientas de
Rust (`cargo fmt`, `clippy -D warnings`, `cargo-audit`) y pide "el
equivalente en el lenguaje del servidor" (sección 2.6). Esta ADR fija
esos equivalentes en Python, el framework de pruebas del servidor y
dónde se configuran.

## Decisión
- Pruebas: `pytest` con el `TestClient` de FastAPI. Las pruebas están en
  `servidor/tests/` y no requieren levantar el servidor.
- Cliente HTTP del `TestClient`: `httpx2`, sobre el que Starlette 1.7.0
  construye su `TestClient` (el uso de `httpx` queda en desuso).
- Formateador y linter: `ruff` (`ruff format` y `ruff check`),
  equivalente de `cargo fmt` y `clippy`.
- Verificación de tipos: `mypy` en modo estricto, sobre `main.py` y
  `tests/`.
- Auditoría de dependencias: `pip-audit`, equivalente de `cargo-audit`,
  ejecutado sobre `servidor/requirements-dev.txt`.
- Configuración de `pytest`, `ruff` y `mypy` en `servidor/pyproject.toml`.
- Dependencias de desarrollo en `servidor/requirements-dev.txt`, con
  versiones fijadas, separadas de `servidor/requirements.txt`. El primero
  incluye al segundo.

## Alternativas consideradas
- **`unittest`:** forma parte de la biblioteca estándar, pero es más
  verboso y ofrece menos soporte para fixtures y parametrización.
- **`black` + `flake8` + `isort`:** cubren lo mismo que `ruff`, pero con
  tres herramientas y tres configuraciones.
- **`pyright`:** es una alternativa válida a `mypy`, pero la Guía
  Técnica nombra `mypy --strict` para Python.
- **Un único `requirements.txt`:** es más simple, pero incluiría
  herramientas de desarrollo en las dependencias de ejecución del
  servidor.

## Consecuencias
- Positivas: las comprobaciones se reproducen con comandos cortos
  desde `servidor/`; las dependencias de ejecución no incluyen
  herramientas de desarrollo; se cubren los equivalentes que exige la
  Guía Técnica.
- Negativas: las versiones fijadas se actualizan manualmente; `httpx2`
  es un paquete reciente (fork de `httpx`); `pip-audit` requiere
  conexión a internet.

## Estado
Aceptado

## Fecha
2026-10-05
