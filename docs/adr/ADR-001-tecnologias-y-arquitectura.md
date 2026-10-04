# ADR-001: Elección de Tecnologías y Arquitectura

## Contexto

El proyecto requiere la definición de un stack tecnológico y una arquitectura que permita el desarrollo de la aplicación, el servidor y la infraestructura de alojamiento.

## Decisiones

### 1. Stack para la Aplicación

- **Tecnología seleccionada**: Rust + Next.js
- **Razón**: Es el stack recomendado por defecto para este proyecto.

### 2. Stack para el Servidor

- **Tecnología seleccionada**: Python + FastAPI
- **Razones**:
  - Familiaridad de los integrantes del equipo con el lenguaje Python y el framework FastAPI
  - Integración nativa con OpenAPI (swagger/documentación automática)
  - Facilidad de desarrollo y validación de APIs

### 3. Base de Datos

- **Estado**: Por definir
- **Observación**: Aún no se ha tomado una decisión sobre el motor de base de datos a utilizar para el servidor.

### 4. Alojamiento y Despliegue

- **Estado**: Por definir
- **Observación**: Los diferentes alojamientos (frontend, backend, base de datos, etc.) están pendientes de definición según los requisitos y disponibilidad.

## Consecuencias

- Se alineará el desarrollo de la aplicación con el stack por defecto establecido.
- El equipo podrá avanzar rápidamente en el servidor gracias a la familiaridad con Python + FastAPI.
- Las decisiones sobre base de datos y alojamiento deberán tomarse en futuros ADRs cuando se disponga de más información sobre requisitos, escalabilidad y presupuesto.

## Estado

Aceptado (con elementos pendientes de resolución)

## Fecha

2026-10-04
