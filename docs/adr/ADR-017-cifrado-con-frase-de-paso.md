# ADR-017: Derivación de clave con Argon2id y cifrado autenticado AES-256-GCM para copias y lotes de sincronización

## Contexto

En la Entrega 8, el usuario puede exportar su libro personal de eventos a un
archivo de copia de seguridad cifrado con una frase de paso (RF-INT-01,
RF-INT-02) y sincronizar sus eventos entre dispositivos propios a través de un
repositorio Git o de una carpeta compartida (RF-INT-03, RF-INT-07).

Como los archivos de respaldo y los lotes de sincronización pueden almacenarse
en servicios externos de terceros (GitHub, GitLab, Dropbox, Google Drive o
Syncthing), es obligatorio que:

1. Quien tenga acceso al archivo exportado, al repositorio Git o a la carpeta
   compartida no pueda leer ningún dato del libro personal (RF-INT-06, INV-16).
2. La clave simétrica se derive de una frase de paso elegida por la persona (de
   al menos 12 caracteres) mediante una función de derivación lenta resistente a
   ataques de fuerza bruta por GPU/ASIC (**Argon2id**) y una sal aleatoria nueva
   en cada cifrado.
3. El contenido se proteja con un esquema de cifrado autenticado con datos
   asociados (**AEAD**) que garantice tanto confidencialidad como integridad:
   cualquier alteración de un byte (en la cabecera, en el cuerpo o en la
   etiqueta) o una frase de paso incorrecta debe detectarse antes de tocar el
   libro y rechazarse con el mensaje claro `«Frase de paso incorrecta o archivo
   dañado.»` (RF-INT-02, RNF-18).
4. La parte en claro del contenedor (identificador del formato, versión,
   parámetros de derivación, sal y nonce) no revele ningún dato del libro
   (RF-INT-06) ni contenga credenciales o la frase de paso (RF-INT-05).

## Decisión

### 1. Derivación de clave con Argon2id (`v0x13`)

Se elige **Argon2id** (RFC 9106, versión `0x13`, crate `argon2` de RustCrypto)
para derivar una clave de **32 bytes (256 bits)** a partir de la frase de paso y
una sal aleatoria de **16 bytes (128 bits)** generada con el CSPRNG del sistema
operativo (`getrandom`) en cada operación de cifrado:

- **Por qué Argon2id:** combina la resistencia de Argon2i frente a ataques de
  canal lateral por acceso a memoria con la resistencia de Argon2d frente a
  ataques de compromiso tiempo-memoria con GPU y hardware especializado (ASIC).
- **Parámetros por defecto:**
  - **Memoria (`m_cost`):** `19 456 KiB` (19 MiB). Corresponde al perfil
    recomendado por OWASP para Argon2id, imponiendo un costo de memoria alto
    para un atacante que intente probar frases en paralelo sin penalizar el uso
    interactivo en computadoras o teléfonos del usuario.
  - **Iteraciones (`t_cost`):** `2` pasadas sobre la memoria.
  - **Paralelismo (`p_cost`):** `1` carril, garantizando tiempos consistentes en
    dispositivos con distinta cantidad de núcleos.
- **Tamaño de la sal (`16 bytes` / 128 bits):** cumple el mínimo recomendado por
  RFC 9106 y NIST SP 800-132. Como se genera una sal nueva en cada exportación o
  lote, dos cifrados del mismo libro con la misma frase de paso producen claves
  derivadas y textos cifrados completamente distintos.
- **Cotas de validación al descifrar:** antes de ejecutar Argon2id, el lector
  verifica que los parámetros declarados en la cabecera estén dentro de límites
  seguros (`8 KiB <= m_cost <= 262 144 KiB`, `1 <= t_cost <= 32`,
  `1 <= p_cost <= 16`). Una cabecera corrupta o maliciosa con valores fuera de
  rango se rechaza de inmediato sin reservar memoria excesiva.

### 2. Cifrado autenticado con AES-256-GCM

Se elige **AES-256-GCM** (NIST SP 800-38D, crate `aes-gcm` de RustCrypto) como
algoritmo de cifrado autenticado con datos asociados (AEAD):

- **Clave:** 256 bits (los 32 bytes producidos directamente por Argon2id).
- **Nonce:** **12 bytes (96 bits)** aleatorios nuevos en cada cifrado, tamaño
  estándar de GCM que entra directamente al contador `J_0` sin paso adicional de
  GHASH.
- **Etiqueta de autenticación (`tag`):** **16 bytes (128 bits)** al final del
  cuerpo cifrado.
- **Cabecera autenticada como AAD:** los 46 bytes de la cabecera en claro se
  pasan como *Associated Authenticated Data* (`Payload { msg, aad }`) a
  AES-256-GCM. De esta manera, la etiqueta GCM protege criptográficamente tanto
  el texto cifrado como todos los metadatos de la cabecera (magia, versión,
  identificador de algoritmo, parámetros de Argon2id, sal y nonce).

### 3. Contenedor binario de 46 bytes de cabecera sin filtración de datos

Todo archivo de respaldo cifrado y todo lote de sincronización tienen la
siguiente disposición binaria:

| Bytes      | Tamaño   | Campo         | Contenido                                            |
|------------|----------|---------------|------------------------------------------------------|
| `0..4`     | 4 bytes  | Magia         | `b"CCBK"` (identificador fijo del contenedor)        |
| `4`        | 1 byte   | Versión       | `1` (`VERSION_CONTENEDOR`)                           |
| `5`        | 1 byte   | Algoritmo     | `1` (`Argon2id v0x13 + AES-256-GCM`)                 |
| `6..10`    | 4 bytes  | Memoria (KiB) | `m_cost` en `u32` little-endian                      |
| `10..14`   | 4 bytes  | Iteraciones   | `t_cost` en `u32` little-endian                      |
| `14..18`   | 4 bytes  | Paralelismo   | `p_cost` en `u32` little-endian                      |
| `18..34`   | 16 bytes | Sal           | Bytes aleatorios CSPRNG                              |
| `34..46`   | 12 bytes | Nonce         | Bytes aleatorios CSPRNG                              |
| `46..N-16` | Variable | Cuerpo        | Texto del libro cifrado con AES-256-GCM              |
| `N-16..N`  | 16 bytes | Etiqueta GCM  | Tag de autenticación sobre `cabecera` (AAD) y cuerpo |

**Por qué la parte en claro no filtra información del libro (RF-INT-06):**
- Los primeros 18 bytes dependen exclusivamente de constantes del protocolo
  (`CCBK`, versión `1`, algoritmo `1` y parámetros fijos de Argon2id), idénticas
  para cualquier usuario y cualquier libro.
- Los siguientes 28 bytes (sal de 16 bytes y nonce de 12 bytes) son salida pura
  del generador aleatorio del sistema operativo, estadísticamente independiente
  del contenido del libro y de la frase de paso.
- La cabecera no guarda nombres de cuentas, monedas, saldos, fechas de eventos,
  cantidad de eventos ni identificadores de dispositivo.

### 4. Ubicación arquitectónica

- En `crates/aplicacion/src/respaldo.rs` vive el puerto `Cifrador` y el tipo de
  error `ErrorRespaldo`, manteniendo `aplicacion` libre de dependencias
  criptográficas externas (RNF-13, ADR-005).
- En `crates/infraestructura/src/cifrado.rs` viven las funciones `cifrar`,
  `descifrar` y el adaptador `CifradorArgon2` que implementa `Cifrador`.

## Alternativas consideradas

- **XChaCha20-Poly1305:** excelente algoritmo AEAD con nonce extendido de 192
  bits. Se optó por AES-256-GCM porque AES-256 es el estándar industrial más
  extendido, cuenta con aceleración por hardware (`AES-NI` / extensiones ARMv8
  Crypto) en procesadores modernos y, al derivarse una clave única de 256 bits
  por cada archivo mediante una sal aleatoria nueva de 128 bits, nunca se
  reutiliza un par `(clave, nonce)`. El byte `5` (`Algoritmo`) en la cabecera
  permite incorporar XChaCha20-Poly1305 en el futuro sin romper el contenedor.
- **PBKDF2-HMAC-SHA256:** descartado porque solo consume CPU y no memoria, lo
  que permite ataques masivamente paralelos en GPU y ASIC a bajo costo.
- **Cabecera en JSON o texto con metadatos del libro (como fecha o número de
  eventos):** descartada porque revelar cuántos eventos tiene el respaldo o
  cuándo se exportó en la parte en claro filtraría metadatos innecesarios sobre
  la actividad financiera de la persona; esos datos van dentro del texto del
  libro, ya cifrado.

## Consecuencias

### Positivas

- Confidencialidad e integridad autenticadas en una sola pasada: ni una frase
  errónea ni un archivo truncado o modificado en cualquier byte pueden producir
  eventos corruptos en el libro personal.
- Los parámetros de Argon2id viajan autenticados en la cabecera, por lo que la
  aplicación podrá elevar el costo de memoria en versiones futuras sin
  invalidar copias de seguridad anteriores.
- El mismo contenedor cifra tanto archivos de exportación como lotes de
  sincronización por carpeta o Git.

### Negativas / deuda

- Si el usuario olvida su frase de paso, es criptográficamente imposible
  recuperar el respaldo (diseño intencional exigido por la especificación; la
  interfaz lo advierte antes de exportar).
- Argon2id añade una latencia deliberada (~100–400 ms por operación según el
  dispositivo), necesaria para frenar ataques de diccionario.

## Estado

Aceptada.

## Fecha

2026-11-25
