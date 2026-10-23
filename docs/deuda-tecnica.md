**D-01 — Vulnerabilidades conocidas en dependencias de la interfaz (npm)**

* **Qué y dónde:** npm audit (ejecutado el 22/10/2026) reporta 1 aviso: `braces` (severidad alta). Dependencias en app/package.json y app/package-lock.json.
* **Por qué se aceptó:** La corrección automática (`npm audit fix --force`) degrada `eslint-config-next` a la versión 14.2.35. Esto introduce un *breaking change* que rompe las reglas de calidad de código establecidas.
* **Impacto si no se paga:** Afecta exclusivamente a la cadena de herramientas de desarrollo (dependencia transitiva de `eslint` y `fast-glob`). El riesgo para el usuario que instala la aplicación final compilada es nulo.
* **Cómo resolverla:** Actualizar dependencias cuando exista un parche de `eslint-config-next` que resuelva el conflicto de forma nativa, ejecutar `npx tsc --noEmit`, `npm run lint` y probar la app — estimación: 1 h
* **Condición de disparo:** Antes de publicar el primer Release, o si aparece un aviso de severidad alta que afecte al código distribuido.

**D-02 — Vulnerabilidades conocidas en dependencias de Rust (cargo audit)**

* **Qué y dónde:** cargo audit (ejecutado el 22/10/2026) reporta 1 aviso: RUSTSEC-2026-0382 en el crate `rustls-webpki`, junto con advertencias de mantenimiento en `proc-macro-error` y `glib`. Dependencias en app/src-tauri/Cargo.lock.
* **Por qué se aceptó:** El aviso proviene de dependencias transitivas profundas del núcleo de Tauri. Forzar una actualización manual rompe la compatibilidad del framework de escritorio.
* **Impacto si no se paga:** Posible fallo o vulnerabilidad en la validación de certificados TLS al realizar peticiones de red externas desde el backend de Rust.
* **Cómo resolverla:** Actualizar la versión de Tauri o sus dependencias principales en `Cargo.toml` cuando los autores publiquen una corrección oficial, y ejecutar `cargo test --workspace` — estimación: 1.5 h
* **Condición de disparo:** Cuando salga la versión corregida, y en cualquier caso antes del primer Release.

**D-09 — Compilación para Android fuera del alcance de las Entregas 1 a 3**

* Qué y dónde: No se genera ni se prueba el APK (`cargo tauri android build`); no existe pipeline release.yml ni en `.github/workflows/`. La aplicación solo se ejecuta y se prueba en entorno de escritorio (Windows/Linux).
* Por qué se aceptó: Las especificaciones de las Entregas 1 a 3 solicitan validar y demostrar la aplicación en escritorio. Configurar el JDK 17, el SDK/NDK de Android y los targets de compilación cruzada en Rust habría consumido tiempo crítico del desarrollo del dominio y la arquitectura del libro de eventos.
* Impacto si no se paga: RNF-11 (soporte multiplataforma en Windows, Linux y Android desde la misma base de código) y ESC-11 no se cumplen. Problemas específicos de móviles (rutas del almacenamiento de datos local, adaptación de pantalla a 360px) aparecerían tarde en el ciclo.
* Cómo resolverla: Instalar la cadena de herramientas de Android, ejecutar `cargo tauri android init`, ajustar `app_data_dir` para plataformas móviles y crear el flujo de integración continua `release.yml` con firma digital del APK — estimación: 6 h
* Condición de disparo: Antes de la entrega que exija instaladores publicados (Release), o antes de la entrega final.