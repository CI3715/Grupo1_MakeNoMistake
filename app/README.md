# Aplicación — Cuentas Claras

Esta carpeta contiene la aplicación de escritorio de Cuentas Claras. La interfaz está hecha con Next.js (React y TypeScript) y se empaqueta en una ventana nativa con Tauri 2, cuyo núcleo está escrito en Rust. En la Entrega 1 la aplicación abre su ventana principal mostrando la página de inicio de Next.js; la pantalla de verificación de conexión y el comando que llama al servidor (`ping_servidor`) se agregan en otras ramas del equipo.

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

## Construir la interfaz estática

```
npm run build
```

Con la opción `output: "export"` de `next.config.ts`, este comando no produce un programa que necesite un servidor, sino una carpeta `out/` con archivos HTML, CSS y JavaScript ya terminados. Esa carpeta es la que Tauri mete dentro del ejecutable en producción, porque una aplicación instalada no puede depender de que haya un servidor de Node corriendo en el computador de la persona usuaria. Por la misma razón, el proyecto no usa rutas de API, componentes de servidor ni acciones de servidor.

## Estructura de la carpeta

```
app/
├── app/                  # Páginas de la interfaz (App Router de Next.js)
│   ├── layout.tsx        # Estructura común de todas las páginas
│   ├── page.tsx          # Página principal
│   └── globals.css       # Estilos globales (Tailwind)
├── public/               # Imágenes y archivos estáticos
├── out/                  # Resultado de `npm run build` (no se sube a Git)
├── src-tauri/            # Núcleo de Rust y configuración de Tauri
│   ├── tauri.conf.json   # Configuración de la ventana y de la compilación
│   ├── Cargo.toml        # Dependencias de Rust
│   ├── Cargo.lock        # Versiones exactas de Rust (sí se sube a Git)
│   ├── capabilities/     # Permisos que la ventana tiene sobre el sistema
│   ├── icons/            # Íconos de la aplicación
│   └── src/
│       ├── main.rs       # Punto de entrada
│       └── lib.rs        # Aquí viven los comandos de Rust
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
