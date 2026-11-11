# AGENTS.md

Guía para agentes de código (Claude Code, Codex, etc.) que trabajan en este repositorio.

**Cuentas Claras**: asistente de finanzas personales multimoneda, *local-first*. El código, los identificadores, los comentarios, los mensajes de error y los commits están en español; mantén ese idioma.

## Piezas del repositorio

- `app/`: aplicación de escritorio. Interfaz Next.js (React + TS, Tailwind) empaquetada con Tauri 2; núcleo en Rust en `app/src-tauri/`.
- `servidor/`: Python + FastAPI gestionado con `uv`. Hoy solo expone `GET /ping`.
- `docs/adr/`: decisiones de arquitectura. Una ADR no se edita cuando cambia la decisión: se escribe otra que la sustituye.
- `docs/persistencia-eventos-y-proyecciones.md`: diseño detallado del libro de eventos y las proyecciones.
- `docs/deuda-tecnica.md`: deuda aceptada (p. ej. avisos conocidos de `npm audit` y `cargo audit`; no ejecutar `npm audit fix --force`).

Regla de diseño principal: el servidor **nunca** recibe cuentas, saldos, movimientos ni categorías (INV-16). La aplicación funciona completa sin red ni servidor; ninguna operación del libro personal puede hacer HTTP.

## Comandos

Aplicación (desde `app/`):

```bash
npm install
npm run tauri dev        # Next en :3000 + ventana Tauri (primera compilación de Rust: varios minutos)
npx tsc --noEmit
npm run lint
npm run build            # exportación estática a out/
```

`npm run dev` solo sirve para maquetar: `invoke` no existe fuera de la ventana de Tauri.

Núcleo de Rust (desde `app/src-tauri/`):

```bash
cargo test --workspace
cargo test -p dominio -p aplicacion          # reglas y casos de uso, sin compilar Tauri ni SQLite
cargo test -p infraestructura                # SQLite
cargo test --test arquitectura               # reglas de dependencia entre capas
cargo test --workspace <filtro>              # una prueba, p. ej. ejemplo_2_2
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo audit
```

Servidor (desde `servidor/`):

```bash
uv sync
uv run uvicorn main:app --reload --port 8000
uv run pytest                                # una prueba: uv run pytest tests/test_main.py::<nombre>
uv run ruff check . && uv run ruff format --check . && uv run mypy   # mypy en modo estricto
```

Verificación completa (desde la raíz): `bash verificar-entrega3.sh` corre fmt, clippy, pruebas, auditorías, tsc, lint y pytest, y deja la salida en `verificacion-entrega3.log`.

## Arquitectura del núcleo (Rust)

`app/src-tauri` es un workspace hexagonal (ADR-005); las dependencias apuntan hacia adentro:

- `crates/dominio`: reglas de negocio puras (monedas, montos, cuentas, movimientos, categorías de fábrica, validaciones con el campo que falla).
- `crates/aplicacion`: casos de uso (`CasosDeUso`), puertos (`puertos.rs`), modelos de lectura (`vistas.rs`) y los adaptadores de prueba en memoria (`memoria.rs`). Depende solo de `dominio`.
- `crates/infraestructura`: adaptadores reales: SQLite con `rusqlite` (`db.rs`, `repositorios.rs`), reloj, generador UUID, id de dispositivo.
- crate `app` (`src-tauri/src/`): comandos de Tauri y raíz de composición (`lib.rs` abre la base y cablea los casos de uso). Es el único que conoce a los otros tres; el `Arc<Mutex<CasosDeUso>>` vive solo aquí.

`tests/arquitectura.rs` hace cumplir con pruebas (leyendo `Cargo.toml` y `cargo tree`): `dominio`/`aplicacion` sin `rusqlite`, `tauri` ni `reqwest`; `infraestructura` sin `reqwest`; sin `f32`/`f64` para dinero; sin HTTP en los comandos; sin `UPDATE`/`DELETE` sobre la tabla `eventos`. El núcleo es síncrono: nada de `async` en dominio, aplicación ni almacenamiento (ADR-006).

### Libro de eventos y proyecciones

- La fuente de verdad es la tabla `eventos` (inmutable por triggers; `guardar` es idempotente por `id` y devuelve `ResultadoGuardado::YaExistia`). Tipos: `CuentaCreada`, `CategoriaCreada`, `MovimientoRegistrado`, `MovimientoCorregido`, `MonedaCreada`, `TasaRegistrada`, `TasaEliminada` y `MonedaBaseCambiada`, con su carga en JSON.
- `cuentas`, `categorias`, `movimientos`, `monedas_propias`, `tasas` y `moneda_base` son **proyecciones** (caché derivada). Eliminar una tasa marca la fila como `eliminada`; nunca hay `UPDATE`/`DELETE` sobre `eventos`. `BaseDatos::persistente` crea el esquema, aplica migraciones, siembra las categorías de fábrica como eventos y **reconstruye las proyecciones en cada apertura**.
- La decodificación JSON vive en `aplicacion::codec` y la traducción a filas en `aplicacion::proyector`; ambos adaptadores (memoria y SQLite) usan ese mismo código. `aplicacion::contrato::bateria_contrato_repositorio` se ejecuta contra los dos para garantizar que producen lo mismo. Un tipo de evento nuevo o un cambio de proyección debe tocar codec, proyector, `memoria.rs`, `repositorios.rs`/`db.rs` y la batería de contrato.
- Los movimientos nunca se editan ni se borran (INV-04). Una corrección es un `MovimientoCorregido` que referencia al original (`reemplaza_a`), y el original pasa a `estado = 'corregido'`. Solo cuentan los movimientos vigentes.
- El saldo siempre se calcula desde los movimientos (INV-05): `saldo_actual = saldo_inicial + ingresos − egresos`. `saldo_inicial` es inmutable.
- Las categorías de fábrica tienen IDs canónicos del dominio (`gasto:alimentacion`, `ingreso:salario`, …). Las bases antiguas guardaron `cat-fabrica-<tipo>-<slug>`; `aplicacion::categorias` los traduce al proyectar sin reescribir eventos.

### Dinero

- Nunca coma flotante (INV-03): `rust_decimal` en dominio y aplicación. En SQLite, `saldo_inicial`, `saldo_actual` y `movimientos.monto` son INTEGER en **unidades mínimas** de la moneda de la cuenta (`monto × 10^decimales`; centavos para las monedas de fábrica). `tasas.valor` es INTEGER con escala fija 10^8. Las conversiones exactas viven en `aplicacion::unidades` y nunca redondean en silencio. `docs/persistencia-eventos-y-proyecciones.md` todavía describe esas columnas como TEXT.
- El catálogo de monedas es dinámico: las 4 de fábrica (`Moneda::catalogo`) más las propias (`MonedaCreada`, validadas por `dominio::crear_moneda`). `Moneda::por_codigo`, `Cuenta::crear` y `registrar_movimiento` solo conocen las de fábrica; el código de la aplicación usa `Moneda::por_codigo_en`, `Cuenta::crear_con_monedas` y `registrar_movimiento_con_monedas` con el catálogo completo (`proyector::catalogo_completo`). Los decimales de una moneda nunca cambian, porque fijan la escala de los montos guardados.
- Los montos cruzan la frontera Rust ↔ interfaz como **texto ya formateado**. La interfaz no suma ni resta: muestra lo que devuelve el núcleo.

### Comandos de Tauri

- `src/cuentas.rs` y `src/movimientos.rs` solo traducen DTO ↔ casos de uso; no contienen reglas de negocio. Los DTO están en `src/dto.rs` y los errores se devuelven como `ErroresDto` (lista de `{ campo, mensaje }` en español, sin trazas técnicas).
- Los tipos TypeScript equivalentes están en `app/app/utils/tauriCommands.ts`; mantenlos sincronizados con `dto.rs`. Un comando nuevo también debe registrarse en `generate_handler!` en `src/lib.rs`.
- `src/tasas.rs` (Entrega 4: tasas, moneda base, monedas propias, patrimonio) es **provisional**: responde con datos fijos en memoria. Los eventos y proyecciones ya existen y `CasosDeUso::crear_moneda` también, pero los comandos de ese archivo todavía no los usan.
- `src/ping.rs` (Entrega 1) es el único código que usa `reqwest`; la URL del servidor se toma de `CUENTAS_CLARAS_URL` (por defecto `http://localhost:8000`).

## Interfaz (Next.js)

- Lee `app/AGENTS.md`: la versión de Next.js (16) tiene cambios incompatibles con lo que puedas conocer; consulta `app/node_modules/next/dist/docs/` antes de escribir código de Next.
- `output: "export"`: no hay rutas de API, componentes de servidor ni server actions, porque la app instalada no tiene un servidor Node.
- La carpeta `app/app/` es intencional (proyecto `app` + App Router `app`).
- La especificación prohíbe la telemetría (RNF-02): `npx next telemetry disable` una vez por máquina.

## Datos locales

La base es `cuentas_claras.db` en el `app_data_dir` de Tauri (en Linux, `~/.local/share/com.cuentasclaras.desktop/`). Para empezar de cero, cierra la app y borra el `.db` y sus `-wal`/`-shm`.

## Contribución

- Todo entra por pull request con revisión; los commits llevan firma DCO (`git commit -s`).
- Antes del PR deben pasar todas las comprobaciones de la pieza modificada.
