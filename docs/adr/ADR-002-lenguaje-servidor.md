# ADR-002: Implementación del servidor con Python y FastAPI

## Contexto
La ADR-001 definió Python + FastAPI como stack del servidor. Esta ADR
detalla las alternativas evaluadas y las decisiones de implementación
de la Entrega 1 (endpoint GET /ping).

## Decisión
- Framework: FastAPI, servido con Uvicorn.
- Puerto de desarrollo: 8000. Se evita el 3000 porque Next.js lo usa
  por defecto en desarrollo.
- Ubicación: carpeta `servidor/` en la raíz del monorepo.
- Dependencias fijadas en `servidor/requirements.txt`, con entorno
  virtual local (`.venv`, excluido de Git).
- Contrato de /ping: JSON con `estado`, `mensaje`, `version` y
  `timestamp` (UTC, formato ISO 8601), respuesta 200.

## Alternativas consideradas
- **Rust + Axum:** muy rápido y de tipado estricto, pero con curva de
  aprendizaje alta y compilación más lenta para el equipo.
- **TypeScript + Hono:** ligero y moderno, pero el equipo tiene menos
  experiencia con él que con Python.

## Consecuencias
- Positivas: desarrollo rápido, validación con Pydantic y documentación
  automática en /docs (OpenAPI).
- Negativas: menor rendimiento bruto que Rust; hay que gestionar
  entornos virtuales de Python.

## Estado
Aceptado

## Fecha
2026-10-05