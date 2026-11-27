# ADR-010: Sesión con token firmado, vigencia configurable y mensaje idéntico ante credenciales incorrectas

## Contexto

La Entrega 5 agrega identidad al servidor: registro (RF-SOC-01), inicio de sesión y perfil propio (RF-SOC-02). Es la base de las funciones sociales que vendrán después, pero el servidor sigue sin recibir ni guardar datos del libro personal (INV-16, RES-02): solo sabe quién es cada persona.

Había que decidir:

1. Cómo demuestra la aplicación, en cada petición, que la persona inició sesión.
2. Cuánto dura una sesión y cómo probar el vencimiento en una demostración sin esperar una hora.
3. Qué responde el servidor cuando las credenciales no coinciden, sin revelar qué correos están registrados (RES-07).
4. Cómo se guarda la contraseña (RNF-05).
5. Dónde viven los secretos: el de firma en el servidor y el token en el dispositivo.

## Decisión

1. **Token firmado (JWT con HS256)**, emitido por `POST /sesiones` y enviado en `Authorization: Bearer <token>`.
   - La carga lleva solo `sub` (id del usuario), `iat` y `exp`. Nada de correo, nombre ni datos financieros.
   - Al verificar se fija el algoritmo (`HS256`), así que se rechazan `alg: none` y tokens firmados con otro algoritmo. `sub` y `exp` son obligatorios.
   - El servidor no guarda sesiones: verifica la firma y el vencimiento, y luego busca al usuario por id. Si el usuario ya no existe, el token deja de valer.
   - Todo fallo (sin encabezado, esquema distinto de Bearer, token alterado, otro secreto, vencido o sin `exp`) responde `401` con `{"mensaje": "Sesión no válida o vencida."}` y `WWW-Authenticate: Bearer`.

2. **Vigencia configurable** con la variable `JWT_MINUTOS`: 60 minutos por defecto y 1 para la demostración del vencimiento. Un valor que no sea un entero positivo impide arrancar. La respuesta incluye `vence_en` (UTC, `AAAA-MM-DDThh:mm:ssZ`) para que la aplicación sepa cuándo pedir de nuevo el inicio de sesión. No hay token de renovación en esta entrega.

3. **Mensaje idéntico ante credenciales incorrectas.** Si el correo no existe o la contraseña no coincide, la respuesta es la misma: `401` con `{"mensaje": "Correo o contraseña incorrectos."}`, mismo cuerpo y mismos encabezados. Además:
   - El correo se compara en minúsculas y sin espacios, igual que se guarda.
   - Si el correo no existe, se verifica la contraseña contra un **hash falso** (argon2id de un valor aleatorio, calculado al arrancar). Así ambos casos calculan un argon2 y tardan lo mismo; el tiempo de respuesta no delata qué correos existen.
   - El `400` solo se usa cuando falta el correo o la contraseña. No se valida el formato del correo al iniciar sesión, para no dar otra pista.

4. **Contraseñas con argon2id** (`argon2-cffi`, parámetros por defecto de la biblioteca), con sal aleatoria por hash. El hash nunca sale del servidor y ninguna respuesta repite la contraseña. Por eso se reemplazó el `422` de FastAPI, que repite el valor enviado (`input`), por un manejador propio que responde `400` con el campo y un mensaje en español.

5. **Almacén de secretos y su respaldo.**
   - **Servidor:** el secreto de firma sale de la variable de entorno `JWT_SECRET`, sin valor por defecto y con al menos 32 caracteres. Si falta, el servidor no arranca: es preferible a firmar con un secreto conocido. En desarrollo se define en la terminal; en un despliegue, en el gestor de secretos de la plataforma. Nunca se sube al repositorio.
   - **Aplicación:** el token se guarda en el almacén de secretos del sistema operativo (Credential Manager en Windows, Keychain en macOS, Secret Service en Linux) mediante el crate `keyring`, solo desde la capa de infraestructura o el crate `app` (la prueba de arquitectura lo prohíbe en `dominio` y `aplicacion`).
   - **Respaldo:** si el almacén del sistema no está disponible (por ejemplo, Linux sin Secret Service), el token se guarda **solo en memoria** mientras la aplicación esté abierta. Al cerrarla hay que iniciar sesión otra vez. Nunca se escribe en la base SQLite del libro ni en un archivo de texto.

## Alternativas consideradas

- **Sesiones guardadas en el servidor (cookie con id de sesión).** Permiten cerrar sesiones al instante, pero exigen una tabla de sesiones y cookies, que encajan mal con un cliente Tauri que llama por HTTP. Se descartó por ahora; se puede agregar una lista de revocación si se necesita.
- **JWT con RS256 (clave pública y privada).** Útil si otros servicios tuvieran que verificar tokens sin poder firmarlos. Hoy solo el servidor verifica, así que HS256 basta y es más simple de configurar.
- **Mensajes distintos para "correo no registrado" y "contraseña incorrecta".** Son más cómodos, pero permiten averiguar qué correos tienen cuenta. Lo prohíbe RES-07.
- **bcrypt o PBKDF2.** Funcionan, pero argon2id es la recomendación actual de OWASP y resiste mejor los ataques con GPU por su uso de memoria.
- **Guardar el token en `localStorage` de la interfaz o en la base SQLite.** Cualquier proceso que lea esos archivos lo obtendría. El almacén del sistema lo cifra por usuario.

## Consecuencias

### Positivas

- El servidor no guarda estado de sesión y cada petición se verifica por sí sola.
- Ninguna respuesta revela si un correo está registrado, ni por el mensaje ni por el tiempo.
- La contraseña no aparece en respuestas, errores ni en la base; solo su hash argon2id.
- La demostración del vencimiento se hace con `JWT_MINUTOS=1`, sin tocar el código.

### Negativas / Deuda a considerar

- Un token robado vale hasta su vencimiento (60 minutos como máximo): no hay revocación ni cierre de sesión en el servidor.
- Cambiar `JWT_SECRET` invalida todas las sesiones abiertas (útil ante una filtración, molesto en una rotación planificada).
- El registro sí revela si un correo o nombre de usuario existe (`409` por campo), porque la especificación lo pide para que la persona sepa qué cambiar. Para mitigarlo haría falta limitar la frecuencia de peticiones, que queda fuera de esta entrega.
- El respaldo en memoria obliga a iniciar sesión en cada apertura en equipos sin almacén de secretos.

## Estado

Aceptada.

## Fecha

2026-11-09
