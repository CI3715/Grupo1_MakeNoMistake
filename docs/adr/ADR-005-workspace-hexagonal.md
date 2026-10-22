# ADR-005: Workspace de Cargo con arquitectura hexagonal

## Contexto

La Entrega 1 dejó la parte Rust de la aplicación como un único crate, `app`, con el comando `ping_servidor`, y la ADR-004 fijó que la aplicación se construye con Tauri 2 y un núcleo en Rust. La Entrega 2 introduce por primera vez reglas de negocio reales (validación de cuentas, catálogo de monedas, montos con aritmética decimal), persistencia local en SQLite y pruebas que deben ejecutarse con un repositorio en memoria, sin base de datos ni red.

La Guía Técnica pide que el núcleo sea un workspace de Cargo con los crates dominio, aplicacion, infraestructura y app, que el dominio no dependa de la base de datos, la red ni el framework (RNF-13) y que cada puerto tenga una implementación real y una de prueba (RNF-14). Con un solo crate el compilador no puede impedir que el dominio importe rusqlite o reqwest, y la ADR-004 eligió un núcleo en Rust precisamente para que sea el compilador quien haga cumplir la regla de dependencia. El criterio de aceptación 7 de la Entrega 2 añade que ninguna operación sobre cuentas genera tráfico hacia el servidor, lo que exige que los crates del núcleo no dependan de reqwest. En la reunión del lunes 12 de octubre el equipo acordó adoptar la estructura de cuatro crates.

## Decisión

app/src-tauri es un workspace de Cargo con cuatro crates. El crate dominio contiene las reglas de negocio y no depende de ningún otro crate del proyecto. El crate aplicacion contiene los casos de uso y los puertos, y depende solo de dominio. El crate infraestructura contiene los adaptadores que implementan los puertos, y depende de aplicacion y dominio. El crate app es el adaptador conductor, porque expone los comandos de Tauri, y la raíz de composición: es el único que conoce a los otros tres y los conecta al arrancar. Las dependencias apuntan siempre hacia adentro.

El crate app se queda en src-tauri/, junto a tauri.conf.json, porque Tauri busca su configuración junto al Cargo.toml de la aplicación; los otros tres viven en src-tauri/crates/. Los cuatro declaran la misma edición, la misma versión mínima de Rust y la misma licencia (Apache-2.0), en cumplimiento de RNF-22. El comando ping_servidor no cambia de lugar: sigue en app y usa reqwest directamente, de modo que ni dominio, ni aplicacion, ni infraestructura dependen de reqwest.

La regla de dependencia se verifica con pruebas de arquitectura en app/src-tauri/tests/arquitectura.rs. Estas pruebas comprueban que dominio y aplicacion no dependen de rusqlite, tauri ni reqwest, que dominio no depende de aplicacion ni de infraestructura, que aplicacion no depende de infraestructura y que infraestructura no depende de reqwest. Cada regla se comprueba de dos maneras: leyendo las dependencias declaradas en el Cargo.toml del crate (incluidas las de desarrollo y de construcción) y recorriendo todo el árbol de dependencias con cargo tree, para detectar las que llegan de forma indirecta. Otra prueba recorre el código de dominio y falla si aparece f32 o f64, en cumplimiento de INV-03, y tres pruebas más verifican las propias utilidades de detección. Además, cargo fmt, cargo clippy y cargo audit se ejecutan sobre todo el workspace (cargo fmt --all, cargo clippy --workspace --all-targets -- -D warnings y cargo audit).

## Alternativas consideradas

Un solo crate con módulos habría sido más simple y no habría tocado la estructura de Tauri, pero el compilador no impide que el dominio importe rusqlite; la regla solo se podría vigilar con búsquedas de texto, que es justo lo que la Guía quiere evitar con verificaciones automáticas.

Mover también app a crates/app habría exigido cambiar tauri.conf.json y los comandos de arranque, sin ningún beneficio para la regla de dependencia, porque esta depende de lo que declara cada Cargo.toml y no de la carpeta donde vive el crate.

La Guía prevé un crate contrato cuando el servidor está escrito en Rust. Como el servidor es de Python con FastAPI, el contrato es el openapi.yaml y no se crea ese crate. Para las pruebas de dependencias se evaluó cargo-deny, que la Guía menciona, pero se prefirió una prueba de Rust con cargo tree porque no añade ninguna herramienta que instalar y corre con el resto de las pruebas.

## Consecuencias

Entre las positivas, el compilador impide que dominio y aplicacion importen dependencias de infraestructura, cada integrante puede trabajar en su crate sin pisar a los demás, y los puertos pueden tener un adaptador real y uno de prueba, como exige RNF-14. Las pruebas de arquitectura detectan una dependencia prohibida aunque llegue de forma indirecta, y sus mensajes nombran el crate y la dependencia.

Entre las negativas, hay más archivos de configuración que mantener y los comandos de calidad deben ejecutarse sobre todo el workspace. Las pruebas de arquitectura tienen límites conocidos: no entienden los comentarios de bloque, de modo que un f64 dentro de uno daría un falso positivo, y la coma flotante solo se vigila en dominio, aunque aplicacion también maneja dinero. Dos de las pruebas llaman a cargo tree con todas las plataformas, por lo que la primera ejecución puede descargar dependencias y necesita red. El cliente HTTP del ping sigue sin puerto ni implementación de prueba, y se registra como deuda técnica.

Tras la conversión se comprobó en Windows y Linux que el workspace compila, que las pruebas pasan y que la ventana de la aplicación abre. cargo audit mantiene los dos avisos descritos en la ADR-004 (proc-macro-error y glib), que provienen de las bibliotecas gráficas de Tauri y no de código del proyecto.

## Estado

Aceptado

## Fecha

2026-10-13