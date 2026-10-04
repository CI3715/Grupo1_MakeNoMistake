## Licencia elegida: Apache License 2.0

Copyright (c) 2026 Grupo1_MakeNoMistake

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

---

## Consideraciones sobre la licencia

### 1. Permisos, prohibiciones y tipos de licenciamiento

Al elegir esta licencia, se han tenido en cuenta los siguientes aspectos:

| Tipo | Permite productos cerrados | Requisitos de liberación | Notas |
|---|---|---|---|
| **Permisiva** (MIT, Apache-2.0, BSD) | Sí | Mínimos (mantenimiento de avisos/copyright) | Cualquiera puede construir un producto cerrado encima. |
| **Copyleft débil** (MPL, LGPL) | Limitado | Deben liberarse los cambios sobre los archivos originales | Protege las modificaciones a los archivos licenciados. |
| **Copyleft fuerte** (GPL) | No | Debe liberarse el trabajo derivado completo | Obliga a publicar el código fuente completo de obras derivadas. |
| **Copyleft fuerte con red** (AGPL) | No | Debe liberarse el trabajo derivado completo, incluso al ofrecerlo como servicio | Extiende las obligaciones al caso de distribución por red. |

### 2. Distribución: Instalador vs. Servicio en red

Este sistema tiene dos formas de distribución que son relevantes para la distinción entre GPL y AGPL:

- **Instalador**: Se entrega al usuario para su instalación local. Bajo GPL, esto activa las obligaciones de copyleft para el trabajo derivado completo.
- **Servicio en red (SaaS)**: Se opera y presta como servicio a través de la red. La **AGPL** extiende la obligación de liberar el código fuente a este caso, mientras que la **GPL** no la extiende a la mera interacción remota.

**Nota**: La licencia Apache 2.0 elegida es permisiva, no copyleft. Por ello, no impone las obligaciones de liberación de trabajos derivados completas que imponen GPL/AGPL. Esta elección permite tanto la distribución mediante instalador como la prestación del servicio en red sin generar la obligación automática de liberar el código fuente del trabajo derivado.

### 3. Concesión de patentes

Apache License 2.0 añade una **concesión expresa de patentes** (Patent Grant) a favor de los usuarios y contribuidores. Esto representa una ventaja frente a licencias más simples como MIT, que no incluyen dicha concesión explícita. Esta cláusula ayuda a reducir el riesgo de litigios por patentes relacionados con el uso, modificación o distribución del software.

### 4. Compatibilidad con dependencias

Todas las dependencias de este proyecto deben ser compatibles con la licencia Apache 2.0. Esta compatibilidad se:

- **Verifica automáticamente** como parte del proceso de construcción y publicación.
- **Documenta** en un archivo **THIRD-PARTY-NOTICES** que acompaña a cada versión publicada del proyecto.

Este archivo THIRD-PARTY-NOTICES incluirá los avisos de copyright, licencias y atribuciones correspondientes a todas las dependencias de terceros utilizadas.

### 5. Contribuciones externas

Para las contribuciones externas, este proyecto utiliza **DCO (Developer Certificate of Origin)** en lugar de un CLA (Contributor License Agreement).

- **DCO**: Requiere que cada commit incluya una línea `Signed-off-by` que certifica la autoría y el derecho a contribuir bajo los términos de la licencia del proyecto. Es más ligero y suficiente para un proyecto de este tamaño.
- **CLA**: Implica un acuerdo legal separado entre el contribuidor y el proyecto, lo cual resulta innecesariamente pesado para este caso.

Todos los commits enviados a este repositorio deben estar firmados con `Signed-off-by` conforme a lo establecido en el DCO.

---

## Texto completo de Apache License 2.0

El texto legal completo de la Apache License 2.0 puede consultarse en:
[https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0)
