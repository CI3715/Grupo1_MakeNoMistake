"""Entrega 8: el servidor no participa en la copia de seguridad ni en la sincronización
del libro personal (RNF-01, INV-16).

La copia y la sincronización van del dispositivo a un archivo o a un destino que elige y
controla la persona (Git o una carpeta). Esta entrega no agrega endpoints, así que:

- el conjunto de rutas es exactamente el de la Entrega 7,
- ninguna ruta ni esquema habla de copias, destinos, lotes, frases de paso o credenciales
  de Git,
- ningún campo del contrato recibe datos del libro personal.

La otra mitad de la verificación (la aplicación no hace HTTP al exportar, importar ni
sincronizar el libro) la cubre ``app/src-tauri/tests/arquitectura.rs``.
"""

from fastapi.routing import APIRoute

from main import app

# Rutas del servidor al cierre de la Entrega 7. Si una entrega futura agrega una ruta,
# debe confirmar que no recibe datos del libro personal y actualizar esta lista.
RUTAS_ENTREGA_7 = {
    "/ping",
    "/usuarios",
    "/sesiones",
    "/usuarios/yo",
    "/grupos",
    "/grupos/{id_grupo}",
    "/grupos/{id_grupo}/miembros",
    "/grupos/{id_grupo}/miembros/yo/respuesta",
    "/grupos/{id_grupo}/gastos",
    "/cambios",
    "/amistades",
    "/amistades/{id_amistad}/respuesta",
    "/amistades/{id_amistad}",
}

# Palabras que delatarían una ruta o un campo para el libro personal, la copia de
# seguridad o la sincronización con un destino propio.
TERMINOS_DEL_LIBRO_PERSONAL = (
    "respaldo",
    "backup",
    "copia",
    "exportar",
    "importar",
    "destino",
    "lote",
    "libro",
    "repositorio",
    "frase",
    "git",
    "saldo_inicial",
)


def _rutas() -> set[str]:
    return {ruta.path for ruta in app.routes if isinstance(ruta, APIRoute)}


def _nombres_del_contrato() -> set[str]:
    """Nombres de rutas, esquemas y propiedades del contrato OpenAPI generado."""
    contrato = app.openapi()
    nombres: set[str] = set(contrato.get("paths", {}))
    esquemas = contrato.get("components", {}).get("schemas", {})
    for nombre, esquema in esquemas.items():
        nombres.add(nombre)
        nombres.update(esquema.get("properties", {}))
    return nombres


def test_la_entrega_8_no_agrega_rutas_al_servidor() -> None:
    assert _rutas() == RUTAS_ENTREGA_7


def test_ninguna_ruta_habla_de_copias_destinos_o_libro_personal() -> None:
    for ruta in _rutas():
        for termino in TERMINOS_DEL_LIBRO_PERSONAL:
            assert termino not in ruta.lower(), (
                f"La ruta {ruta} menciona «{termino}»: el libro personal no pasa por "
                "el servidor (RNF-01, INV-16)"
            )


def test_ningun_esquema_ni_campo_recibe_datos_del_libro_personal() -> None:
    for nombre in _nombres_del_contrato():
        for termino in TERMINOS_DEL_LIBRO_PERSONAL:
            assert termino not in nombre.lower(), (
                f"El contrato menciona «{nombre}» (contiene «{termino}»): el servidor "
                "no debe recibir datos del libro personal (RNF-01, INV-16)"
            )