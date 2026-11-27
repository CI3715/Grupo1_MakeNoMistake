# Amigos y Grupos (Entrega 6 · interfaz)

Entrada única para `page.tsx`: `PantallaSocial` (mismo patrón que `PantallaSesion`).
Sin sesión invita a iniciar sesión y **no llama al servidor**; con sesión muestra la pantalla.

| Archivo | Qué hace |
|---|---|
| `PantallaSocial` | Decide: cargando, invitación a iniciar sesión o la pantalla |
| `AmigosConectado` | Buscar (nombre exacto), invitar, aceptar, rechazar, eliminar |
| `BuscadorUsuario`, `ListaAmigos`, `InvitacionesAmistad` | Piezas de Amigos (solo presentan) |
| `GruposConectado` | Navegación interna: lista → detalle → nuevo gasto (estado local) |
| `ListaGrupos`, `InvitacionesGrupo`, `FormularioGrupo` | Piezas de la lista de grupos |
| `DetalleGrupoConectado` → `DetalleGrupo`, `MiembrosGrupo`, `GastosGrupo`, `BalancesGrupo` | Detalle del grupo |
| `NuevoGastoConectado` → `FormularioGasto`, `TablaPartes` | Nuevo gasto con vista previa en vivo |
| `ElementosSociales` | Tarjeta, aviso, botón, select, chip, esqueleto de carga |

Reglas: la interfaz solo presenta y recoge entrada; los montos llegan como texto ya calculado
(`previsualizar_division` en Rust y el servidor hacen la aritmética). Los errores de cada campo
salen debajo de él (`useEnvio`); lo que no es de un campo va en un aviso y nunca se muestra texto técnico.

## Datos fijos (solo desarrollo)

Mientras los comandos responden con el simulador de `social_fijo.rs` no hace falta sesión.
Crea `app/.env.local` (ignorado por git) con:

    NEXT_PUBLIC_DATOS_FIJOS_SOCIAL=1

Al conectar los comandos reales se quita esa variable y rige la sesión.
