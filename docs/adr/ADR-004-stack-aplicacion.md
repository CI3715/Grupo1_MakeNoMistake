# ADR-004: Stack de la aplicación: Tauri 2 y Next.js con exportación estática

## Contexto

La ADR-001 fijó que la aplicación se hace con Rust y Next.js, pero solo dio como razón que es el stack recomendado por defecto. Esta ADR completa esa decisión con las razones propias del proyecto y con los detalles de implementación de la Entrega 1, que pide elegir entre una aplicación de escritorio (Tauri 2 con Next.js) y una móvil (Flutter o Kotlin), y que la ventana principal abra en local.

La decisión debe cumplir varios requerimientos de la Especificación. La aplicación debe distribuirse para Windows, Linux y Android desde el mismo código fuente (RNF-11), instalarse sin que la persona usuaria tenga que instalar entornos de ejecución ni servicios adicionales (RNF-12), funcionar sin conexión (RNF-03), guardar los datos financieros solo en el dispositivo (RNF-01) y no enviar telemetría (RNF-02). Además, la Guía Técnica pide que la interfaz solo presente y recoja entrada, y que cualquier regla de negocio quede en el núcleo.

## Decisión

La aplicación se construye con Tauri 2, cuyo núcleo está escrito en Rust, y con una interfaz en Next.js. En el momento de esta decisión las versiones son Tauri 2.12 (CLI 2.12.1) y Next.js 16.3.8 con React 19, y el código de la interfaz se escribe en TypeScript, con Tailwind 4 para los estilos y ESLint como linter. Estas son las opciones que ofrece por defecto `create-next-app`, y se aceptaron porque cubren las necesidades de una interfaz que solo presenta datos.

Next.js se configura con exportación estática (`output: "export"` en `next.config.ts`), de modo que `npm run build` produce una carpeta `out/` con HTML, CSS y JavaScript ya terminados, sin servidor. Se agrega también `images: { unoptimized: true }`, porque la optimización automática de imágenes de Next.js necesita un servidor y haría fallar el build. Tauri carga esa carpeta en producción (`frontendDist: "../out"`) y, en desarrollo, la dirección `http://localhost:3000` (`devUrl`), donde Tauri arranca Next.js por su cuenta con `beforeDevCommand: "npm run dev"`. Antes de empaquetar construye la interfaz con `beforeBuildCommand: "npm run build"`.

El proyecto vive en la carpeta `app/` de la raíz del monorepo, al lado de `servidor/`. La parte Rust está en `app/src-tauri/` y el identificador de la aplicación es `com.cuentasclaras.desktop`, ya que Tauri se niega a empaquetar instaladores con el valor por defecto. El gestor de paquetes es npm, que viene con Node y con el que `create-next-app` generó el proyecto, y sus versiones exactas quedan fijadas en `package-lock.json`. La interfaz usa el puerto 3000 y el servidor el 8000, como ya dejó establecido la ADR-002.

La interfaz no llama al servidor directamente. Siguiendo la especificación de la entrega, la pantalla invoca un comando de Rust con `invoke('ping_servidor')` y es ese comando el que hace la petición HTTP, de modo que la comunicación con el servidor queda en el núcleo y la interfaz se limita a mostrar el resultado. Cada componente que hable con Rust es un componente de cliente, y no se usan rutas de API, componentes de servidor ni acciones de servidor, porque la aplicación no tiene servidor propio.

Además, se desactivó la telemetría anónima de Next.js con `npx next telemetry disable`, y se quitó `Cargo.lock` del `.gitignore` de la raíz, porque Tauri produce un ejecutable y `cargo audit` necesita ese archivo para auditar las dependencias de Rust.

## Alternativas consideradas

Flutter y Kotlin eran la otra rama que ofrecía la entrega. Flutter obligaba a reescribir el núcleo en Dart y a perder la posibilidad de compartir tipos con un núcleo en Rust, y no figura entre las tecnologías de la Guía Técnica, por lo que habría exigido una aprobación aparte. Kotlin solo cubre Android y deja fuera Windows y Linux, que RNF-11 exige. Tauri 2 en cambio cubre las tres plataformas con un único código fuente.

Electron está prohibido por la Guía Técnica, y además empaqueta un navegador completo y un entorno de Node dentro de cada instalador, lo que va en contra de una instalación liviana. Vite con React se acepta en la Guía como equivalente de Next.js y habría sido más simple para una aplicación de una sola ventana, pero se prefirió Next.js por ser lo que nombra directamente la especificación de la entrega. La alternativa de usar Tauri solo con TypeScript, con la lógica en el navegador embebido, exige aprobación previa y obliga a verificar la regla de dependencia con herramientas adicionales, mientras que con el núcleo en Rust el compilador la hace cumplir.

Como gestor de paquetes se podrían haber usado pnpm o yarn. Se eligió npm porque no requiere instalar nada más que Node, y para el tamaño de este proyecto las ventajas de velocidad o de espacio de los otros no justifican una herramienta adicional.

## Consecuencias

Entre las consecuencias positivas, la persona usuaria final recibe un instalador autocontenido que no necesita Node ni un servidor, y la misma base de código sirve para Windows, Linux y Android. El núcleo en Rust permite usar desde el primer día herramientas de calidad maduras (`cargo fmt`, `clippy` y `cargo audit`), y la separación entre interfaz y núcleo deja la lógica de negocio fuera de los componentes, como exige la Guía.

Entre las negativas, el entorno de desarrollo es más pesado que el de una aplicación web: hay que instalar Rust, Node y varias bibliotecas del sistema en Linux (como WebKit y GTK). La primera compilación de Rust es lenta, ya que tardó unos 13 minutos en un portátil modesto, aunque las siguientes tardan segundos. La exportación estática descarta funciones de Next.js que dependen de un servidor, y toda comunicación con el servidor del proyecto debe pasar por comandos de Rust.

Quedan además varios puntos abiertos que se anotan como posible deuda técnica. `npm audit` reporta 5 vulnerabilidades de severidad alta que en realidad son una sola cadena: un defecto en el paquete `braces`, del que dependen `micromatch`, `fast-glob`, `@next/eslint-plugin-next` y `eslint-config-next`. Todas pertenecen a la configuración del linter, es decir, a herramientas de desarrollo que no se distribuyen con la aplicación. El arreglo que propone `npm audit fix --force` instalaría una versión de `eslint-config-next` dos generaciones anterior a Next.js 16, por lo que se decidió no aplicarlo. Por su parte, `cargo audit` no encontró vulnerabilidades en las 424 dependencias de Rust, pero sí dos avisos que no bloquean: el paquete `proc-macro-error`, sin mantenimiento, y un aviso de comportamiento no seguro (`unsound`) en `glib` 0.18.5. Ambos provienen de las bibliotecas gráficas de Linux que usa Tauri, así que su corrección depende de que Tauri las actualice. Estos puntos son candidatos a registrarse como deuda técnica. La configuración de Tauri deja la política de seguridad de contenido sin definir (`"csp": null`, el valor por defecto), que conviene endurecer más adelante. La desactivación de la telemetría de Next.js se guarda por máquina y fuera del repositorio, por lo que cada integrante debe hacerla una vez y por eso se documenta en el README de `app/`. Finalmente, aunque Tauri 2 soporta Android, en esta entrega solo se verificó la ventana de escritorio en Linux; la compilación para Windows y Android queda pendiente de comprobar.

## Estado

Aceptado

## Fecha

2026-10-06
