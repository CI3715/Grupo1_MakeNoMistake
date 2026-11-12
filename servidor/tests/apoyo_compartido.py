"""Utilidades compartidas por las pruebas de los repositorios de la Entrega 6."""

from repositorio_usuarios import RepositorioUsuarios, Usuario


def crear_usuario(repositorio: RepositorioUsuarios, nombre: str) -> Usuario:
    """Crea una persona con datos mínimos y nombres visibles legibles."""

    return repositorio.crear(
        nombre_usuario=nombre,
        nombre_visible=nombre.replace(".", " ").title(),
        correo=f"{nombre}@example.com",
        hash_contrasena="hash",
    )
