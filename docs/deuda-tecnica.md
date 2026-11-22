**D-01 — Vulnerabilidades conocidas en dependencias de la interfaz (npm)**

* **Qué y dónde:** npm audit (ejecutado el 22/10/2026) reporta 1 aviso: `braces` (severidad alta). Dependencias en app/package.json y app/package-lock.json.
* **Por qué se aceptó:** La corrección automática (`npm audit fix --force`) degrada `eslint-config-next` a la versión 14.2.35. Esto introduce un *breaking change* que rompe las reglas de calidad de código establecidas.
* **Impacto si no se paga:** Afecta exclusivamente a la cadena de herramientas de desarrollo (dependencia transitiva de `eslint` y `fast-glob`). El riesgo para el usuario que instala la aplicación final compilada es nulo.
* **Cómo resolverla:** Actualizar dependencias cuando exista un parche de `eslint-config-next` que resuelva el conflicto de forma nativa, ejecutar `npx tsc --noEmit`, `npm run lint` y probar la app — estimación: 1 h
* **Condición de disparo:** Antes de publicar el primer Release, o si aparece un aviso de severidad alta que afecte al código distribuido.

**D-02 — Advertencias de mantenimiento y de seguridad de memoria en dependencias de Rust (cargo audit)**

* **Qué y dónde:** `cargo audit` (ejecutado el 11/11/2026) no reporta vulnerabilidades. Quedan 7 advertencias en `app/src-tauri/Cargo.lock`: sin mantenimiento, `proc-macro-error` 1.0.4 (RUSTSEC-2024-0370), `zbus_names` 4.3.5 (RUSTSEC-2026-0371), `zvariant` 5.15.1 (RUSTSEC-2026-0370) y `zvariant_derive` 5.15.1 (RUSTSEC-2026-0372); con problemas de seguridad de memoria (unsound), `glib` 0.18.5 (RUSTSEC-2024-0429), `rustix` 1.1.5 (RUSTSEC-2026-0486) y `tokio` 1.53.2 (RUSTSEC-2026-0386). El aviso de `libsqlite3-sys` (RUSTSEC-2026-0459) que apareció antes se corrigió subiendo `rusqlite` a 0.41.1, que usa `libsqlite3-sys` 0.39.1.
* **Por qué se aceptó:** Ninguno de los siete avisos indica una versión corregida (`cargo audit` no muestra la línea `Solution`). Origen comprobado con `cargo tree`: `zvariant` y `zbus_names` (y la macro `zvariant_derive`, que forma parte de `zvariant`) llegan por `keyring` 4.2.0, dependencia directa de `app` desde la Entrega 5, a través de `zbus-secret-service-keyring-store`, `secret-service` y `zbus`, es decir, por el respaldo de Secret Service que usa `sesion.rs` en Linux. `tokio` 1.53.2 es dependencia directa de `app` y además la usan `reqwest` (0.12.28 y 0.13.5) y `tauri`. El origen de `glib`, `proc-macro-error` y `rustix` queda por confirmar con `cargo tree --target all -i <crate>`.
* **Impacto si no se paga:** Son advertencias, no vulnerabilidades explotables conocidas. Los avisos de `unsound` se refieren a funciones concretas (`AsyncFd::new`, `glib::VariantStrIter`, funciones `xattr` con un búfer de longitud cero). Se comprobó mediante análisis estático que el código propio del proyecto no invoca directamente ninguna de estas funciones o patrones de riesgo.
* **Cómo resolverla:** Cuando `keyring`, Tauri o las librerías que arrastran estos crates publiquen versiones corregidas, actualizarlas en `Cargo.toml`, ejecutar `cargo update`, `cargo test --workspace` y `cargo audit`. Las cuatro advertencias de `zvariant` y `zbus_names` dependen de `keyring`, que es una elección propia del proyecto: si no se corrigen, se puede reevaluar el crate o el respaldo (ver D-10) — estimación: 1.5 h
* **Condición de disparo:** Cuando aparezca una versión corregida, si un aviso pasa a ser una vulnerabilidad, y en cualquier caso antes del primer Release.

**D-09 — Compilación para Android fuera del alcance de las Entregas 1 a 3**

* Qué y dónde: No se genera ni se prueba el APK (`cargo tauri android build`); no existe pipeline release.yml ni en `.github/workflows/`. La aplicación solo se ejecuta y se prueba en entorno de escritorio (Windows/Linux).
* Por qué se aceptó: Las especificaciones de las Entregas 1 a 3 solicitan validar y demostrar la aplicación en escritorio. Configurar el JDK 17, el SDK/NDK de Android y los targets de compilación cruzada en Rust habría consumido tiempo crítico del desarrollo del dominio y la arquitectura del libro de eventos.
* Impacto si no se paga: RNF-11 (soporte multiplataforma en Windows, Linux y Android desde la misma base de código) y ESC-11 no se cumplen. Problemas específicos de móviles (rutas del almacenamiento de datos local, adaptación de pantalla a 360px) aparecerían tarde en el ciclo.
* Cómo resolverla: Instalar la cadena de herramientas de Android, ejecutar `cargo tauri android init`, ajustar `app_data_dir` para plataformas móviles y crear el flujo de integración continua `release.yml` con firma digital del APK — estimación: 6 h
* Condición de disparo: Antes de la entrega que exija instaladores publicados (Release), o antes de la entrega final.

**D-10 — Respaldo en archivo del almacén de secretos de la sesión (Entrega 5)**

* **Qué y dónde:** La sesión (token, vencimiento y nombre visible) se guarda con el crate `keyring` (Secret Service en Linux, Keychain en macOS, Credential Manager en Windows). Si el sistema no lo ofrece, por ejemplo en Linux sin servicio de secretos activo, `app/src-tauri/src/sesion.rs` guarda el mismo JSON en `sesion.json` dentro del directorio de datos de la aplicación, con permisos 0600 en Unix.
* **Por qué se aceptó:** El enunciado de la Entrega 5 admite un archivo local si la plataforma dificulta el almacén del sistema (RF-SOC-02). Cifrar el archivo exigía elegir y mantener un esquema de claves que no cabía en la entrega.
* **Impacto si no se paga:** El archivo `sesion.json` no está cifrado. Quien lea el directorio de datos de la aplicación obtiene el token. El riesgo queda acotado porque el token vence a los 60 minutos y no da acceso a datos financieros, que nunca salen del dispositivo (INV-16).
* **Cómo resolverla:** Cifrar el archivo con una clave derivada del sistema, o negarse a guardar la sesión cuando no exista almacén del sistema (la persona vuelve a iniciar sesión al reabrir), y probar ambos caminos en `sesion/pruebas.rs` — estimación: 3 h
* **Condición de disparo:** Antes de publicar el primer Release, o si la aplicación empieza a guardar en la sesión algo más sensible que un token de 60 minutos.

**D-11 — Las pruebas contra PostgreSQL real no se ejecutan en Windows**

* **Qué y dónde:** `servidor/tests/test_integracion_postgres.py` (8 pruebas) y `servidor/tests/test_repositorio_usuarios.py` (6 pruebas) se omiten cuando no existe `CUENTAS_CLARAS_DB_URL_PRUEBAS`; en `uv run pytest` aparecen como 14 omitidas. El script `servidor/scripts/base_datos_local.sh` que levanta un PostgreSQL local es de bash y está pensado para Linux.
* **Por qué se aceptó:** El resto de las pruebas del servidor usa un almacén en memoria con el mismo contrato que `RepositorioUsuarios`, así que `uv run pytest` funciona en cualquier computador sin instalar nada. Parte del equipo trabaja en Windows y no pudo levantar la base local.
* **Impacto si no se paga:** Si nadie ejecuta esas 14 pruebas, un defecto en `repositorio_usuarios.py` o en el esquema SQL (por ejemplo, el índice único sobre `lower(correo)`) pasa inadvertido hasta que el servidor corre contra PostgreSQL de verdad. Las pruebas de la contraseña (`test_contrasena_no_se_filtra.py`) verifican lo que el servidor entrega al almacén, no lo que PostgreSQL guarda.
* **Cómo resolverla:** Ejecutar esas 14 pruebas en el pipeline de CI con un servicio PostgreSQL, y ofrecer una alternativa para Windows (script en PowerShell o instrucciones con el instalador oficial) — estimación: 3 h
* **Condición de disparo:** Antes de desplegar el servidor, y en cualquier caso antes de la entrega que dependa de PostgreSQL para datos compartidos.

**D-12 — El servidor no limita la longitud máxima de la contraseña**

* **Qué y dónde:** `servidor/reglas.py` solo exige un mínimo de 10 caracteres; no hay máximo en el registro ni en el inicio de sesión. `servidor/openapi.yaml` lo refleja: documenta `minLength` sin `maxLength`.
* **Por qué se aceptó:** El enunciado de la Entrega 5 solo fija el mínimo (RF-SOC-01). Se prefirió que el contrato dijera lo mismo que el servidor antes que prometer un límite que nada aplicaba.
* **Impacto si no se paga:** Una contraseña de tamaño exagerado obliga al servidor a procesarla con argon2 antes de rechazarla, lo que facilita agotar CPU o memoria con pocas peticiones.
* **Cómo resolverla:** Fijar un máximo (por ejemplo, 128 caracteres) en `reglas.py` para el registro y el inicio de sesión, agregar sus pruebas y restituir `maxLength` en `openapi.yaml` — estimación: 1 h
* **Condición de disparo:** Antes de desplegar el servidor fuera del entorno local.

**D-13 — Vulnerabilidad transitiva en `opentelemetry-api` en el servidor (`uv audit`)**

- **Qué y dónde:** `uv audit` (ejecutado el 13/11/2026) reporta 1 vulnerabilidad conocida en `servidor/uv.lock`: `opentelemetry-api` 1.45.1 tiene el aviso `GHSA-mf5v-w3w3-f6cg` / `CVE-2026-126360` (*Denial of service through quadratic-time traceparent parsing*), corregido en la versión `1.45.2`.
- **Por qué se aceptó:** Se verificó mediante `uv tree --invert --package opentelemetry-api` que la versión `1.45.1` viene fijada como dependencia transitiva de `fastapi` (`0.142.2`). Intentar actualizar el paquete de forma aislada (`uv lock --upgrade-package opentelemetry-api`) mantiene la versión anclada para no romper las restricciones del framework, y forzar un *override* en el cierre de la Entrega 6 introduce riesgo innecesario de regresión sobre el servidor ya validado.
- **Impacto si no se paga:** Bajo. El vector de ataque requiere procesar cabeceras W3C `traceparent` malformadas de gran longitud para provocar consumo cuadrático de CPU, y el servidor actual no expone ni utiliza trazabilidad distribuida abierta a clientes externos.
- **Cómo resolverla:** Actualizar `fastapi` (y su árbol de dependencias hacia `opentelemetry-api >= 1.45.2`) en cuanto una versión compatible esté disponible, o configurar `[tool.uv] override-dependencies` en `servidor/pyproject.toml`, ejecutando posteriormente `uv audit`, `uv run mypy .` y `uv run pytest` — estimación: 1 h
- **Condición de disparo:** Antes de desplegar el servidor fuera del entorno local, o en cuanto `fastapi` libere compatibilidad nativa con `opentelemetry-api >= 1.45.2`.

**D-14 — Rutas de amistades y grupos sin esquema de respuesta en `/openapi.json`**

- **Qué y dónde:** Los endpoints de amistades y grupos en el servidor FastAPI (`servidor/`) construyen y devuelven objetos `JSONResponse` manualmente sin declarar `response_model` ni el diccionario `responses={...}` en sus decoradores de ruta. Como resultado, el esquema generado automáticamente en `/openapi.json` (y visible en `/docs`) omite la estructura de los cuerpos de respuesta exitosa y los posibles códigos y formatos de error (`400`, `403`, `404`, `409`).
- **Por qué se aceptó:** Se priorizó implementar y validar la lógica de dominio y las pruebas de integración de amistades y grupos dentro del plazo de la Entrega 6, manteniendo la especificación manual `servidor/openapi.yaml` como referencia del contrato.
- **Impacto si no se paga:** La documentación interactiva (`/docs`) y cualquier herramienta de validación o generación de clientes que consuma `/openapi.json` carecen de información sobre los campos devueltos y los errores HTTP, obligando a inspeccionar el código fuente de las rutas o mantener dos fuentes de verdad (`openapi.yaml` vs. código).
- **Cómo resolverla:** Definir modelos Pydantic de salida para las respuestas y errores de amistades y grupos, añadir `response_model=...` y `responses={...}` en cada ruta del servidor y verificar que `/openapi.json` coincida con `servidor/openapi.yaml` ejecutando `uv run mypy .` y `uv run pytest` — estimación: 2 h
- **Condición de disparo:** Antes de integrar nuevas pantallas del cliente con los endpoints de amistades y grupos, o antes de publicar la API para consumo externo.