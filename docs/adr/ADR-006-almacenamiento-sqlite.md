# ADR-006: Almacenamiento Local de Eventos con SQLite y rusqlite

## Contexto
La Especificación Técnica de la Entrega 2 requiere que los eventos del dominio (como la creación de cuentas) se guarden únicamente en el almacenamiento local del dispositivo (Local-First). 
Este documento detalla la elección de la base de datos, el conector utilizado en Rust y la estructura del esquema que soporta el libro inmutable de eventos, cumpliendo con los invariantes arquitectónicos (INV-13, INV-16) y las restricciones operativas (RES-05, RNF-12).

## Decisión
- **Motor de Base de Datos:** SQLite. Embebido dentro de la aplicación sin requerir un proceso de servidor independiente.
- **Conector de Rust:** `rusqlite` (versión síncrona), conforme a las directrices de la Guía Técnica que prohíben el uso de código asíncrono (`async`) en las capas de núcleo y almacenamiento local.
- **Ubicación del Archivo:** El archivo de base de datos (`cuentas_claras.db`) se almacena en el directorio de datos de la aplicación (`app_data_dir`) provisto por Tauri, aislando la data por usuario a nivel del sistema operativo.
- **Estructura de la Tabla:** Una única tabla `eventos` con el siguiente esquema estricto (acordado en la planificación de la Entrega 2):
  - `id`: Clave primaria (`TEXT`), almacena el UUID v4.
  - `tipo`: `TEXT` (ej. "CuentaCreada").
  - `momento_registro`: `TEXT` en formato UTC ISO 8601.
  - `dispositivo_origen`: `TEXT` almacenando el identificador del dispositivo creador.
  - `datos`: `TEXT` almacenando el contenido en JSON.
- **Idempotencia (INV-13):** Las escrituras de eventos en SQLite utilizan `INSERT OR IGNORE`. Esto asegura que si se intenta guardar el mismo evento dos veces, la base de datos no arrojará un error ni creará duplicados, manteniendo el mismo estado del sistema.

## Alternativas consideradas
- **Almacenamiento en archivos JSON planos:** Aunque es sencillo, requeriría reescribir todo el archivo en cada nuevo evento, lo que degrada el rendimiento a medida que el libro de eventos crece, y es propenso a corrupción en cortes de energía.
- **SQLx (Asíncrono):** Es una librería popular en Rust con validación en tiempo de compilación, pero obligaría a propagar rutinas `async` por todo el núcleo, violando las reglas establecidas en la Guía Técnica del proyecto.
- **Sled / IndexedDB (Vía Tauri):** Sled es un almacén clave-valor integrado en Rust, pero menos estándar que SQLite. Usar IndexedDB desde la interfaz gráfica acoplaría la persistencia al entorno web, rompiendo la regla de dependencia que sitúa la infraestructura del lado de Rust.

## Consecuencias
- **Positivas:** 
  - Cumple el requerimiento "sin instalación externa" (RNF-12) y "sin proceso servidor" (RES-05).
  - La tabla única de eventos (con datos en JSON) elimina la necesidad de escribir migraciones estructurales complejas cada vez que se añada un nuevo caso de uso o tipo de evento (Compatibilidad hacia adelante).
  - `rusqlite` es maduro, síncrono y muy rápido para lecturas/escrituras locales secuenciales.
- **Negativas:** 
  - El uso de la cláusula `INSERT OR IGNORE` puede enmascarar errores de lógica si un ID repetido es accidental en lugar de un intento legítimo de idempotencia.
  - Las consultas directas de datos específicos (ej. "dame todos los saldos iniciales") son costosas a nivel de base de datos, ya que los detalles residen dentro del texto JSON. Sin embargo, esto es mitigado por la arquitectura: la capa de aplicación proyecta los eventos en memoria para calcular saldos (decisión documentada en ADR-005).

## Estado
Aceptado

## Fecha
2026-10-13