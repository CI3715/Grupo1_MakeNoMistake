# Aplicación — Cuentas Claras

Esta carpeta contiene la aplicación de escritorio de Cuentas Claras. La interfaz está hecha con Next.js (React y TypeScript) y se empaqueta en una ventana nativa con Tauri 2, cuyo núcleo está escrito en Rust. Desde la Entrega 2 la aplicación abre directamente en la pantalla **Mis cuentas**, sin registro ni inicio de sesión: permite crear cuentas (efectivo, banco o billetera digital) con su moneda y saldo inicial, y las guarda solo en el dispositivo, en un archivo SQLite, sin necesidad de internet ni del servidor. El comando de Rust `ping_servidor` de la Entrega 1 sigue existiendo, pero ya no participa en este flujo. Desde la Entrega 3 cada cuenta también registra gastos e ingresos, permite corregirlos mediante ajustes y calcula su saldo automáticamente, todo en el dispositivo y sin conexión.

Los comandos de este documento se ejecutan desde la carpeta `app/`, salvo que se indique otra cosa. El archivo `package.json` vive aquí y no en la raíz del repositorio, así que si npm se queja de que no lo encuentra, casi seguro estás en la carpeta equivocada.

## Prerrequisitos

Se necesitan tres cosas: las bibliotecas del sistema que Tauri usa para dibujar la ventana, Rust para compilar el núcleo y Node.js para la interfaz. Estas instrucciones son para Ubuntu y Debian, que es donde se probaron. Para Windows y macOS hay que seguir la guía oficial de prerrequisitos de Tauri 2 en https://v2.tauri.app/start/prerequisites/.

**1. Bibliotecas del sistema.** Tauri muestra la página web con WebKit y dibuja la ventana con GTK, que en Linux se instalan con `apt`:

```
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**2. Rust, con rustup.** No conviene instalar `rustc` desde `apt`, porque Ubuntu congela una versión antigua. El instalador oficial deja siempre la versión actual y permite actualizarla con un comando:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Elige la instalación por defecto y, al terminar, cierra y vuelve a abrir la terminal para que el sistema encuentre `cargo`.

**3. Node.js, con nvm.** El paquete `nodejs` de `apt` suele ser demasiado antiguo para Next.js, por lo que se usa nvm, un administrador de versiones de Node:

```
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
```

Cierra y abre la terminal otra vez y ejecuta `nvm install --lts`. Si ese comando de instalación falla, hay que copiar el vigente desde el README del repositorio `nvm-sh/nvm`.

Para comprobar que todo quedó instalado:

```
node --version
npm --version
rustc --version
cargo --version
git --version
```

El proyecto se probó con Node 24, npm 11, Rust 1.99 y Git 2.43. Cualquier versión LTS reciente de Node y la versión estable actual de Rust deberían servir.

## Instalación

Desde la raíz del repositorio:

```
git clone https://github.com/CI3715/Grupo1_MakeNoMistake.git
cd Grupo1_MakeNoMistake/app
npm install
```

`npm install` descarga las dependencias de la interfaz a `node_modules/` (la primera vez tarda unos minutos) y respeta las versiones exactas fijadas en `package-lock.json`. Las dependencias de Rust no se instalan a mano: Cargo las descarga solas la primera vez que se compila.

Next.js envía por defecto datos anónimos de uso, y la especificación del proyecto prohíbe la telemetría (RNF-02). Esa preferencia se guarda por máquina y fuera del repositorio, por lo que cada persona debe desactivarla una vez:

```
npx next telemetry disable
```

## Ejecutar en modo desarrollo

Para abrir la aplicación completa:

```
npm run tauri dev
```

Este comando hace dos cosas seguidas. Primero lanza `npm run dev`, que levanta Next.js en `http://localhost:3000`, y cuando esa dirección responde compila el núcleo de Rust y abre una ventana nativa titulada "Cuentas Claras" que carga la página desde allí. Los cambios que se guarden en la interfaz se ven al instante. La primera compilación tarda varios minutos, porque Rust construye todas sus dependencias desde cero (unos 13 minutos en un portátil modesto); las siguientes tardan segundos. Para detener todo, `Ctrl + C` en la terminal.

Si solo se quiere trabajar la interfaz en el navegador, sin Rust, basta con `npm run dev` y abrir `http://localhost:3000`. Hay que tener en cuenta que el servidor del proyecto usa el puerto 8000, justamente para no chocar con el 3000 de Next.js; las instrucciones para levantarlo están en `servidor/README.md`.

## Cuentas y almacenamiento local (Entrega 2)

Las cuentas se guardan únicamente en el dispositivo. Al arrancar, la aplicación abre (o crea) un archivo SQLite llamado `cuentas_claras.db` dentro del directorio de datos de la aplicación (`app_data_dir` de Tauri), cuya carpeta se llama como el identificador de la aplicación, `com.cuentasclaras.desktop`:

| Sistema | Ruta del archivo |
|---|---|
| Linux | `~/.local/share/com.cuentasclaras.desktop/cuentas_claras.db` |
| macOS | `~/Library/Application Support/com.cuentasclaras.desktop/cuentas_claras.db` |
| Windows | `%APPDATA%\com.cuentasclaras.desktop\cuentas_claras.db` |

La ruta de Linux está comprobada; las otras dos siguen la convención de Tauri 2. Al iniciar en modo desarrollo, la terminal imprime la ruta real (`Base de datos local lista en ...`). Para empezar de cero, cierra la aplicación y borra `cuentas_claras.db` y, si existen, `cuentas_claras.db-wal` y `cuentas_claras.db-shm`.

Cómo funciona, en pocas palabras:

- Crear una cuenta valida los datos en el dominio (Rust) y registra un evento inmutable `CuentaCreada` en la tabla `eventos`. La tabla `cuentas` es una proyección derivada de esos eventos. La base rechaza cualquier `UPDATE` o `DELETE` sobre `eventos`.
- El frontend solo se comunica con el núcleo mediante `invoke('listar_monedas')`, `invoke('listar_cuentas')` e `invoke('crear_cuenta')`. Ninguna operación sobre cuentas hace peticiones HTTP, y el servidor no recibe cuentas, saldos ni monedas.

**Prueba de autonomía.** Cierra la aplicación, desactiva la red, apaga el servidor y vuelve a abrirla con `npm run tauri dev`: debe mostrar las mismas cuentas y permitir crear otra.

## Movimientos, saldos y el ejemplo 2.2 (Entrega 3)

Desde la Entrega 3 cada cuenta tiene movimientos: gastos, ingresos y correcciones. Al abrir una cuenta desde **Mis cuentas** se ve su saldo actual, cómo se compone (saldo inicial, total de ingresos, total de gastos y total de ajustes) y la lista de movimientos, del más reciente al más antiguo. Todo se guarda en el mismo archivo SQLite de las cuentas y funciona sin internet y sin el servidor.

Reglas que conviene tener presentes:

- **Un movimiento registrado nunca se modifica ni se elimina** (INV-04). No existe ninguna opción para editar ni borrar, ni en la interfaz ni en el dominio. Si hubo un error, se usa **Corregir**: se indica el monto correcto y el sistema registra un movimiento de **ajuste** por la diferencia, que referencia al original. El original sigue visible y marcado como corregido. Un monto correcto de `0` anula el movimiento por completo. Los ajustes no se corrigen; siempre se corrige el original.
- **El saldo siempre se calcula desde los movimientos** (INV-05): saldo actual = saldo inicial + ingresos − gastos ± ajustes. La interfaz no suma ni resta: muestra los textos que entrega el núcleo.
- **Validaciones** (viven en el dominio y cada error indica el campo que falla): monto cero, negativo o con más decimales de los que admite la moneda de la cuenta (`monto`); categoría vacía, inexistente o del otro tipo (`categoria`); fecha futura (`fecha`). Un gasto puede dejar el saldo en negativo, pero la interfaz avisa antes de confirmar.
- **Categorías**: vienen 7 de gasto (Alimentación, Transporte, Servicios, Salud, Educación, Entretenimiento, Otros gastos) y 4 de ingreso (Salario, Ventas, Regalos, Otros ingresos). Se pueden crear categorías propias; no puede haber dos del mismo tipo con el mismo nombre, sin distinguir mayúsculas.
- **«Hoy»** es la fecha local del dispositivo, no la de UTC.

Comandos de Tauri que usa la interfaz, todos con montos como texto y errores `{ campo, mensaje }` en español:

| Comando | Qué hace |
|---|---|
| `invoke('registrar_movimiento')` | Valida y registra un gasto o un ingreso. |
| `invoke('previsualizar_movimiento')` | Calcula el saldo que quedaría y si es negativo, sin guardar nada. |
| `invoke('corregir_movimiento')` | Registra el ajuste por la diferencia y devuelve el ajuste. |
| `invoke('listar_categorias')` | Categorías de fábrica y propias. |
| `invoke('crear_categoria')` | Crea una categoría propia. |
| `invoke('detalle_cuenta')` | Saldo actual, composición y movimientos de una cuenta. |

### Reproducir el ejemplo 2.2 en la aplicación

Con `npm run tauri dev` (empezando de cero, ver «Cuentas y almacenamiento local»):

1. Crea la cuenta **Billetera dólares** en **USD** con saldo inicial `1000,00`.
2. Ábrela desde **Mis cuentas**: el saldo es `$ 1.000,00` y la lista de movimientos está vacía.
3. Registra un **ingreso** de `800,00` con la categoría **Salario**. El saldo pasa a `$ 1.800,00`.
4. Registra un **gasto** de `500,00` con la categoría **Alimentación** (se quería anotar 50 y se escribió 500). El saldo pasa a `$ 1.300,00`.
5. Sobre ese gasto elige **Corregir** e indica el monto correcto `50,00`. Se registra un ajuste de `+ $ 450,00` y el saldo queda en `$ 1.750,00` (1.000 + 800 − 50).
6. El gasto original sigue en la lista, marcado como corregido, y el ajuste muestra a qué movimiento corrige.

### Reproducir el ejemplo 2.2 con la prueba automatizada

Desde `src-tauri/`:

```
cargo test --workspace ejemplo_2_2
```

Corren dos pruebas, ambas con un repositorio en memoria (sin base de datos ni red): `ejemplo_2_2_billetera_dolares_e_invariantes_inv04_inv05`, en la capa de aplicación, y `el_ejemplo_2_2_se_reproduce_tal_cual`, en los comandos de Tauri. Crean la cuenta, registran los tres movimientos y comprueban cada saldo intermedio: 1.000,00 → 1.800,00 → 1.300,00 → 1.750,00, además de la composición final y de que el original queda marcado como corregido.

### Prueba de autonomía de la Entrega 3

Registra y corrige movimientos, cierra la aplicación, desactiva la red, apaga el servidor y vuelve a abrirla con `npm run tauri dev`: los movimientos, la marca de corregido y los saldos deben ser exactamente los mismos. Ningún movimiento ni saldo se envía al servidor (INV-16).

## Monedas, tasas de cambio y patrimonio (Entrega 4)

Desde la Entrega 4 la aplicación permite registrar tasas de cambio, elegir una moneda base y ver el patrimonio total consolidado en esa moneda. Todo se guarda en el mismo archivo `cuentas_claras.db` de las entregas anteriores (ver «Cuentas y almacenamiento local») y funciona sin internet y sin el servidor. Las tasas son personales: no se envían al servidor (INV-16).

La regla de oro es que nunca se suman montos de monedas distintas sin una tasa explícita con fecha (INV-02). Si falta una tasa, el sistema no inventa una conversión ni muestra un total incorrecto: muestra los subtotales por moneda y nombra el par que falta. Una tasa indica cuántas unidades de la moneda de destino equivalen a una de la moneda de origen, con su fecha (no futura) y su fuente; admite hasta 8 decimales y un valor mayor que cero. Pueden convivir varias tasas del mismo par en fechas distintas, y el patrimonio usa la más reciente con fecha igual o anterior a la fecha de consulta (si hay dos el mismo día, gana la registrada más recientemente). Si solo existe la tasa en sentido contrario se usa su inversa (1 ÷ valor), y nunca se calculan tasas cruzadas a través de una tercera moneda. Cambiar la moneda base (USD por defecto) no modifica ningún movimiento ni saldo: solo cambia la moneda del total. Además de VES, USD, EUR y USDT, que no se pueden modificar ni eliminar, se pueden crear monedas propias con código de 3 a 5 letras mayúsculas, nombre, símbolo y de 0 a 8 decimales.

Cada operación se guarda como un evento inmutable (`TasaRegistrada`, `TasaEliminada`, `MonedaBaseCambiada` y `MonedaCreada`) en la tabla `eventos`, que sigue admitiendo solo inserciones: eliminar una tasa es un evento nuevo, no un `DELETE`, y no toca ningún movimiento porque los movimientos nunca guardan una conversión. Los montos se guardan como enteros en la unidad menor de su moneda (50,00 USD es 5000) y las tasas como enteros multiplicados por 10⁸, sin coma flotante (INV-03). El texto solo se usa en lo que viaja hacia la pantalla. Las conversiones usan aritmética decimal exacta y el total consolidado se redondea una sola vez, al final, a los decimales de la moneda base.

Comandos de Tauri que usa la interfaz, con montos y tasas como texto, fechas `AAAA-MM-DD` y errores `{ campo, mensaje }` en español:

| Comando | Qué hace |
|---|---|
| `invoke('registrar_tasa')` | Valida y registra una tasa con su fecha y su fuente. |
| `invoke('eliminar_tasa')` | Registra la eliminación de una tasa registrada por error. |
| `invoke('listar_tasas')` | Historial de cada par con valor, fecha y fuente. |
| `invoke('cambiar_moneda_base')` | Cambia la moneda en la que se presenta el total. |
| `invoke('crear_moneda')` | Crea una moneda propia. |
| `invoke('listar_monedas')` | Monedas de fábrica y propias. |
| `invoke('consultar_patrimonio')` | Saldos, subtotales por moneda, total consolidado, tasas usadas y pares que faltan. |

### Reproducir el ejemplo 2.3 en la aplicación

Con `npm run tauri dev`, empezando de cero (ver «Cuentas y almacenamiento local»):

1. Crea tres cuentas: **Efectivo bolívares** en **VES** con `88.700,00`, **Banco en dólares** en **USD** con `200,00` y **Billetera USDT** en **USDT** con `50,00`.
2. En **Tasas de Cambio** registra `1 USD = 887,00 VES` con fecha 20/10/2026 y fuente `BCV`, y `1 USDT = 1.000,00 VES` con fecha 21/10/2026 y fuente `casa de cambio`.
3. Elige **VES** como moneda base y abre **Patrimonio**: el total consolidado es `Bs. 316.100,00` (88.700,00 + 177.400,00 + 50.000,00) y el total declara las dos tasas que usó.
4. Cambia la moneda base a **USD**. El efectivo se convierte con la inversa de USD→VES (88.700 ÷ 887 = 100,00 USD), pero no existe ninguna tasa entre USDT y USD, así que no aparece total: se ven los subtotales por moneda y el aviso «Falta una tasa de USDT a USD con fecha igual o anterior a» la fecha de consulta.
5. Registra `1 USDT = 1,00 USD`. El total pasa a ser `$ 350,00`.
6. Para ver otra vez el caso de la tasa faltante, elimina una tasa necesaria: el sistema deja de consolidar y nombra el par exacto.

### Reproducir el ejemplo 2.3 con la prueba automatizada

Desde `src-tauri/`:

```
cargo test --workspace ejemplo_2_3
```

Corren tres pruebas con un repositorio en memoria (sin base de datos ni red): `test_entrega4_ejemplo_2_3_completo_y_patrimonio`, en la capa de aplicación, que recorre el ejemplo completo (316.100,00 VES, el faltante USDT→USD con base USD y los 350,00 USD tras registrar la tasa); `el_ejemplo_2_3_se_reproduce_de_punta_a_punta`, en los comandos de Tauri; y `consolidar_subtotales_ejemplo_2_3`, en el dominio.

### Prueba de autonomía de la Entrega 4

Registra tasas, cambia la moneda base y consulta el patrimonio; cierra la aplicación, desactiva la red, apaga el servidor y vuelve a abrirla con `npm run tauri dev`: las tasas, la moneda base y el patrimonio deben ser exactamente los mismos. Ninguna tasa ni saldo se envía al servidor.

## Ejecutar las pruebas

Desde `src-tauri/`:

```
cargo test --workspace
```

Esto ejecuta las pruebas del dominio, de la capa de aplicación, de la infraestructura (SQLite) y de los comandos de Tauri, además de `tests/arquitectura.rs`, que comprueba que el dominio y la aplicación no dependen de la red ni de la base de datos y que no se usa coma flotante para dinero (INV-03). La primera ejecución compila Tauri y puede tardar varios minutos.

Las pruebas de las reglas de negocio y los casos de uso (invariantes INV-01, INV-02, INV-03, INV-13 e INV-18 y cada validación de crear cuenta) usan un repositorio **en memoria**, sin base de datos ni red, y se pueden correr solas, sin compilar Tauri:

```
cargo test -p dominio -p aplicacion
```

Las pruebas de la Entrega 3 se pueden ejecutar por partes, siempre desde `src-tauri/`:

| Qué comprueban | Comando |
|---|---|
| Los comandos de movimientos, categorías y detalle (incluido el ejemplo 2.2) con el repositorio en memoria | `cargo test movimientos` |
| El dominio: movimientos, ajustes, categorías y cálculo del saldo | `cargo test -p dominio` |
| Los casos de uso `RegistrarMovimiento` y `CorregirMovimiento` y las invariantes INV-04, INV-05 e INV-13, sin base de datos ni red | `cargo test -p aplicacion` |
| Que no haya `f32` ni `f64` en el dinero, ni HTTP en los comandos, ni `UPDATE` o `DELETE` sobre la tabla de eventos | `cargo test --test arquitectura` |

Para la interfaz, desde `app/`:

```
npx tsc --noEmit
npm run lint
npm run build
```

## Configurar la dirección del servidor

Por defecto, la aplicación busca el servidor en `http://localhost:8000`, que es
el puerto del servidor de `servidor/README.md`. Para usar otra dirección, define
la variable de entorno `CUENTAS_CLARAS_URL` al arrancar la aplicación.

En Linux y Git Bash:

    CUENTAS_CLARAS_URL=http://localhost:9999 npm run tauri dev

En PowerShell (Windows):

    $env:CUENTAS_CLARAS_URL="http://localhost:9999"
    npm run tauri dev

La variable solo vale para esa ejecución. Si el servidor está apagado o la
dirección es incorrecta, la pantalla muestra un mensaje de error en vez de
"Conectado". Usa `http` solo en desarrollo: en producción el servidor debe
estar detrás de HTTPS (RNF-06).

## Construir la interfaz estática

```
npm run build
```

Con la opción `output: "export"` de `next.config.ts`, este comando no produce un programa que necesite un servidor, sino una carpeta `out/` con archivos HTML, CSS y JavaScript ya terminados. Esa carpeta es la que Tauri mete dentro del ejecutable en producción, porque una aplicación instalada no puede depender de que haya un servidor de Node corriendo en el computador de la persona usuaria. Por la misma razón, el proyecto no usa rutas de API, componentes de servidor ni acciones de servidor.

## Estructura de la carpeta

```
app/
├── app/                  # Páginas de la interfaz (App Router de Next.js)
│   ├── components/
│   │   ├── MisCuentas.tsx          # Pantalla principal: carga, crea y lista cuentas con invoke
│   │   ├── FormularioCuenta.tsx    # Formulario "Nueva cuenta" con errores por campo
│   │   ├── ListaCuentas.tsx        # Tabla de cuentas con formato local (Bs. 1.250,50)
│   │   ├── SubtotalesPorMoneda.tsx # Subtotal por moneda (nunca se suman monedas distintas)
│   │   ├── EstadoVacio.tsx         # Invitación a crear la primera cuenta
│   │   └── PingScreen.tsx          # Pantalla de la Entrega 1 (ya no se muestra)
│   ├── types/cuentas.ts  # Tipos compartidos con el núcleo de Rust
│   ├── utils/formato.ts  # Formato de montos y normalización del saldo
│   ├── layout.tsx        # Estructura común de todas las páginas
│   ├── page.tsx          # Página principal (abre en Mis cuentas)
│   └── globals.css       # Estilos globales (Tailwind)
├── public/               # Imágenes y archivos estáticos
├── out/                  # Resultado de `npm run build` (no se sube a Git)
├── src-tauri/            # Núcleo de Rust y configuración de Tauri
│   ├── tauri.conf.json   # Configuración de la ventana y de la compilación
│   ├── Cargo.toml        # Dependencias de Rust
│   ├── Cargo.lock        # Versiones exactas de Rust (sí se sube a Git)
│   ├── capabilities/     # Permisos que la ventana tiene sobre el sistema
│   ├── icons/            # Íconos de la aplicación
│   ├── crates/
│   │   ├── dominio/          # Reglas de negocio: Moneda, Monto, Cuenta, validaciones
│   │   ├── aplicacion/       # Casos de uso, puertos y repositorio en memoria (pruebas)
│   │   └── infraestructura/  # SQLite (rusqlite), reloj, identificadores, dispositivo
│   ├── tests/arquitectura.rs # Reglas de dependencias entre capas
│   └── src/
│       ├── main.rs       # Punto de entrada
│       ├── lib.rs        # Arranque: abre la base local y cablea los casos de uso
│       ├── cuentas.rs    # Comandos listar_monedas, listar_cuentas y crear_cuenta
│       ├── movimientos.rs # Comandos de movimientos, categorías y detalle de cuenta (Entrega 3)
│       ├── dto.rs        # Tipos que viajan entre Rust y la interfaz
│       └── ping.rs       # Comando ping_servidor (Entrega 1, fuera del flujo de cuentas)
├── next.config.ts        # Configuración de Next.js (exportación estática)
├── package.json          # Dependencias y scripts de la interfaz
└── package-lock.json     # Versiones exactas de la interfaz
```

La carpeta interna `app/app/` no es un error: el proyecto se llama `app`, y Next.js llama también `app` a la carpeta donde viven las páginas.

En `src-tauri/tauri.conf.json`, la sección `build` conecta ambos mundos. `devUrl` y `beforeDevCommand` se usan solo en desarrollo: la primera es la dirección donde corre Next.js y la segunda el comando que lo arranca. `frontendDist` y `beforeBuildCommand` se usan solo al empaquetar: la primera es la carpeta `../out` con la interfaz construida, relativa a `src-tauri/`, y la segunda el comando que la genera.

## Comprobaciones antes de abrir un pull request

El proyecto exige formato, linter y auditoría de dependencias, tanto para la interfaz como para el núcleo de Rust. Para la interfaz:

```
npm run lint
npm audit
```

Para el núcleo de Rust, desde `src-tauri/`:

```
cargo fmt --check
cargo clippy -- -D warnings
cargo audit
```

Si `cargo fmt --check` muestra diferencias, `cargo fmt` las corrige solo. `cargo audit` no viene con Rust y se instala una sola vez con `cargo install cargo-audit`. Todos los commits deben llevar la firma exigida por el DCO, es decir, hacerse con `git commit -s`.

## Problemas comunes

**`npm error ENOENT ... package.json`.** Se ejecutó npm fuera de `app/`. Entra a la carpeta con `cd app` y repite el comando.

**La primera ejecución de `npm run tauri dev` parece trabada.** Está compilando Rust. Mientras salgan líneas que dicen `Compiling ...` no hay que cancelarla.

**Falla la compilación mencionando `webkit2gtk`, `gtk` o `ssl`.** Falta alguna de las bibliotecas del sistema del primer prerrequisito. Se repite el `apt install` y se copia el error completo si persiste.

**El puerto 3000 está ocupado.** Hay otro Next.js (u otro programa) corriendo. Cierra ese proceso, porque Tauri espera la interfaz exactamente en `http://localhost:3000`.

**Tauri se niega a empaquetar un instalador.** El identificador de la aplicación en `tauri.conf.json` debe ser propio del proyecto (aquí es `com.cuentasclaras.desktop`) y no el valor por defecto `com.tauri.dev`.

**`npm install` avisa de vulnerabilidades de severidad alta.** Son dependencias de herramientas de desarrollo. No hay que ejecutar `npm audit fix --force`, porque aplica cambios que pueden romper el proyecto; las vulnerabilidades pendientes quedan para revisarse y, si no se resuelven, registrarse como deuda técnica.

**En el navegador (`npm run dev`) aparece "No se pudieron cargar tus cuentas".** Es lo esperado: los comandos `invoke` solo existen dentro de la ventana de Tauri. Usa `npm run tauri dev`.

**La pantalla de ping muestra "No se pudo conectar con el servidor".** El servidor no
está encendido o la dirección es otra. Levántalo según `servidor/README.md` y
comprueba que `http://localhost:8000/ping` responde en el navegador.
