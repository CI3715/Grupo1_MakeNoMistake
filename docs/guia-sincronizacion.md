# Cómo respaldar y sincronizar tu libro con Git o con una carpeta

Esta guía explica cómo guardar una copia de seguridad de Cuentas Claras, cómo restaurarla y cómo mantener el mismo libro en varios dispositivos tuyos. Cubre la Entrega 8 (RF-INT-01 a RF-INT-07).

## Antes de empezar: qué pasa con tus datos

- Todo ocurre entre tu dispositivo y **un archivo o un destino que tú eliges y controlas**: un repositorio Git tuyo o una carpeta tuya. **El servidor de Cuentas Claras no participa** y no recibe nada de tu libro.
- Todo lo que sale de tu dispositivo va **cifrado con tu frase de paso**. Quien tenga acceso a tu repositorio o a tu carpeta no puede leer tus cuentas, movimientos ni saldos.
- Tu frase de paso y las credenciales de Git se guardan en el **almacén seguro de tu sistema operativo**. Nunca van dentro del libro, de la copia de seguridad ni del destino.
- **Si olvidas la frase de paso, no hay forma de recuperar la copia ni los datos sincronizados.** Guárdala en un gestor de contraseñas.

Las dos funciones están en el menú lateral: **Copia de seguridad** y **Sincronización**.

---

## 1. Exportar una copia de seguridad

Una copia de seguridad es un único archivo con todo tu libro. Exportar no cambia nada de lo que ya tienes.

1. Abre **Copia de seguridad** en el menú lateral.
2. En **Exportar copia de seguridad**, elige cómo guardarla:
   - **Cifrar con frase de paso** (recomendado). Escribe la frase y repítela. Debe tener **al menos 12 caracteres**. La aplicación te recuerda que, si la olvidas, no hay forma de recuperar la copia.
   - **Exportar sin cifrar**. Genera un archivo de texto legible, útil para revisarlo o reconstruir tus saldos con otra herramienta. Cualquiera que lo abra puede leer tu libro, por eso debes marcar **«Entiendo que el archivo no estará protegido»** para continuar.
3. Pulsa **Elegir ubicación…** y escoge dónde guardar el archivo. La copia cifrada termina en `.ccbak` y la legible en `.txt`.
4. Pulsa **Exportar copia**. Al terminar verás la confirmación y un recordatorio de que tu libro no se modificó. La frase se borra de la pantalla.

> Una copia cifrada, abierta en un editor de texto, no muestra ningún dato legible. Ni siquiera la parte que no está cifrada revela información de tu libro.

El formato de la copia sin cifrar está documentado en [`formato-exportacion.md`](formato-exportacion.md), de modo que puedas reconstruir tus saldos sin la aplicación.

## 2. Importar una copia de seguridad

Importar sirve para restaurar tu libro, por ejemplo en una instalación nueva.

1. Abre **Copia de seguridad** y ve a **Importar copia de seguridad**.
2. Pulsa **Elegir archivo…** y selecciona tu `.ccbak` (o el `.txt`).
3. Escribe la **frase de paso** con la que se cifró. Déjala vacía si la copia se exportó sin cifrar.
4. Pulsa **Importar copia**.

Al terminar, la aplicación te dice cuántos eventos fueron **nuevos**, cuántos **ya existían** y cuáles fueron **rechazados** (con el motivo). Algunas cosas útiles que debes saber:

- **Importar el mismo archivo dos veces es seguro.** La segunda vez verás, por ejemplo, «0 nuevos, 44 ya existían» y tu libro queda igual.
- **Importar una copia antigua nunca borra lo que ya tienes.** Solo se agrega lo que falta.
- Si el archivo es inválido, la frase es incorrecta o el archivo está dañado, **no se importa nada** y tu libro queda como estaba.
- En una instalación nueva, importar deja el libro completo: las mismas cuentas, movimientos, tasas y saldos.

---

## 3. Sincronizar entre tus dispositivos

La sincronización mantiene el mismo libro en todos tus dispositivos. Cada dispositivo escribe solo en su propia carpeta del destino, en archivos cifrados que nunca se modifican. Por eso no hay conflictos: todos terminan con los mismos eventos y los mismos saldos, sin importar en qué orden sincronices.

Elige **uno** de estos destinos.

### 3.1. Con un repositorio Git privado

Necesitas una cuenta en un servicio de Git (GitHub, GitLab, etc.) y el programa **Git instalado** en cada dispositivo (en Windows, [Git for Windows](https://git-scm.com/download/win)). La aplicación guarda una copia local del repositorio junto a tu libro; solo contiene lotes cifrados.

1. **Crea un repositorio privado y vacío**, por ejemplo `mi-libro`. Debe ser privado: aunque todo va cifrado, no hay razón para exponerlo.
2. **Crea un token de acceso personal** en tu servicio de Git con permiso para **leer y escribir** en ese repositorio. Cópialo: normalmente solo se muestra una vez.
3. En Cuentas Claras abre **Sincronización**.
4. En **Tipo de destino** elige **Repositorio Git** y completa:
   - **Dirección del repositorio**, por ejemplo `https://github.com/tu-usuario/mi-libro.git`.
   - **Usuario** de tu servicio de Git.
   - **Token de acceso**.
   - **Frase de paso** y **Repite la frase de paso** (mínimo 12 caracteres).
5. Pulsa **Guardar destino**. La aplicación sincroniza de inmediato.

Dentro del repositorio verás una carpeta por dispositivo con archivos de lote cifrados. No los abras, no los edites y no los borres.

### 3.2. Con una carpeta compartida

Funciona con cualquier servicio que mantenga una carpeta sincronizada entre tus equipos: **Syncthing**, **Dropbox** o **Google Drive**.

1. Instala y configura el servicio en tus dispositivos, de modo que **la misma carpeta** (por ejemplo `CuentasClaras-Sync`) se sincronice entre ellos.
2. En Cuentas Claras abre **Sincronización**.
3. En **Tipo de destino** elige **Carpeta compartida** y pulsa **Elegir carpeta…** para seleccionarla.
4. Escribe la **frase de paso** y repítela (mínimo 12 caracteres).
5. Pulsa **Guardar destino**. La aplicación sincroniza de inmediato.

Consejos:

- **Deja que el servicio termine** de copiar los archivos antes de sincronizar en el otro dispositivo. Si todavía no llegaron, simplemente aparecerán en la siguiente sincronización.
- Con Google Drive o Dropbox, mantén la carpeta **disponible sin conexión** en el dispositivo, para que los archivos estén realmente en el disco.
- No guardes otros archivos tuyos dentro de esa carpeta ni renombres los que genera la aplicación.

### 3.3. Unir un dispositivo nuevo

1. Instala Cuentas Claras en el dispositivo nuevo.
2. Si usas una carpeta compartida, asegúrate de que el servicio ya la tenga sincronizada en ese equipo.
3. Abre **Sincronización**, elige el **mismo destino** (mismo repositorio o misma carpeta) y escribe **exactamente la misma frase de paso**.
4. Pulsa **Guardar destino**. La primera sincronización trae todo el libro.

> Escribir la frase dos veces al configurar evita que un error de tipeo deje un dispositivo con una frase distinta a la de los demás.

### 3.4. Sincronizar día a día

- La aplicación sincroniza **sola al abrirla**, si hay un destino configurado.
- También puedes pulsar **Sincronizar ahora** cuando quieras.
- Cada vez verás cuántos eventos se **enviaron**, cuántos se **recibieron** y cuáles fueron **rechazados** (con el motivo), además de la fecha de la **última sincronización exitosa**.
- Si no hay conexión o el destino no está accesible, **no se pierde nada**: lo pendiente se envía en la siguiente sincronización.
- Sincronizar de nuevo sin cambios nuevos no modifica nada: verás 0 enviados y 0 recibidos.

### 3.5. Cambiar las credenciales o quitar el destino

- **Actualizar credenciales** (solo Git): escribe el usuario y el token nuevos. Reemplazan a los anteriores en el almacén seguro de tu sistema.
- **Quitar destino**: la aplicación te pide confirmación. Borra de **este dispositivo** la configuración, la frase y las credenciales guardadas. **No borra** lo que ya está en el repositorio o en la carpeta, ni cambia tu libro local.

---

## 4. Qué hacer ante cada error

En ningún caso verás mensajes técnicos. Esto es lo que pasa y qué hacer:

| Qué ves | Qué significa | Qué hacer | Tu libro |
| :--- | :--- | :--- | :--- |
| **«Frase de paso incorrecta o archivo dañado.»** | La frase no coincide con la del archivo, o el archivo se alteró o se dañó. La aplicación no distingue entre ambos casos a propósito. | Revisa la frase (mayúsculas, espacios, teclado). Si estás seguro de ella, prueba con otra copia del archivo. | No cambia. |
| **El archivo es de la versión X del formato y la aplicación soporta la Y.** | La copia la generó una versión distinta de Cuentas Claras. | Actualiza la aplicación a la versión que generó el archivo, o exporta de nuevo desde ella. | No cambia. |
| **No se pudo llegar al repositorio o a la carpeta; se reintentará.** (o **Sin conexión**) | No hay internet, el repositorio no responde, o la carpeta no está disponible (disco desmontado, servicio detenido). | Revisa tu conexión o que la carpeta exista y esté sincronizada. Pulsa **Reintentar** o espera a la próxima sincronización. | No cambia. Lo pendiente se conserva y se envía después. |
| **Las credenciales fueron rechazadas.** | El servicio de Git no aceptó tu usuario o token (vencido, revocado o sin permiso de escritura). | Crea un token nuevo con permiso de lectura y escritura y pulsa **Actualizar credenciales**. | No cambia. |
| **Un evento aparece como rechazado, con su motivo.** | Llegó un evento con el mismo identificador que uno que ya tienes, pero con contenido distinto. Se conserva el tuyo. | Normalmente no necesitas hacer nada: el resto de los eventos se aplicó con normalidad. Si se repite, anota el motivo que aparece en el informe. | El resto se aplica normalmente. |

Dos mensajes más que puedes ver al importar:

- **«Este archivo está cifrado: escribe la frase de paso.»** Escribe la frase en el campo correspondiente.
- **«El archivo no tiene el formato de una copia de seguridad de Cuentas Claras.»** El archivo no es una copia válida o está incompleto. No se importa nada.

---

## 5. Buenas prácticas

- Usa una frase de paso larga y única, y guárdala en un gestor de contraseñas.
- Usa la **misma frase en todos tus dispositivos**. No hay forma de cambiar la frase de datos que ya están sincronizados ni de recuperarla si se olvida.
- Haz una copia de seguridad cifrada de vez en cuando y guárdala en un lugar distinto al del destino de sincronización.
- Si exportas sin cifrar, borra el archivo cuando termines de usarlo.