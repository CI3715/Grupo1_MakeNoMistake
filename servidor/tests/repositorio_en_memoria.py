"""Almacén de usuarios en memoria para las pruebas de las rutas.

Imita a ``RepositorioUsuarios`` (PostgreSQL): recorta los nombres, guarda el
correo en minúsculas, no distingue mayúsculas al buscar por correo y lanza las
mismas excepciones de duplicado. Así las pruebas de las rutas corren sin una
base de datos; ``test_repositorio_usuarios.py`` y ``test_integracion_postgres.py``
prueban el repositorio real.
"""

from datetime import datetime, timezone
from uuid import UUID, uuid4

from repositorio_usuarios import (
    CorreoYaRegistrado,
    NombreUsuarioYaRegistrado,
    Usuario,
)


class RepositorioEnMemoria:
    def __init__(self) -> None:
        self.usuarios: dict[UUID, Usuario] = {}

    def crear(
        self,
        *,
        nombre_usuario: str,
        nombre_visible: str,
        correo: str,
        hash_contrasena: str,
    ) -> Usuario:
        usuario = Usuario(
            id=uuid4(),
            nombre_usuario=nombre_usuario.strip(),
            nombre_visible=nombre_visible.strip(),
            correo=correo.strip().lower(),
            hash_contrasena=hash_contrasena,
            fecha_alta=datetime.now(timezone.utc).replace(microsecond=0),
        )
        if any(u.correo == usuario.correo for u in self.usuarios.values()):
            raise CorreoYaRegistrado()
        if any(
            u.nombre_usuario == usuario.nombre_usuario for u in self.usuarios.values()
        ):
            raise NombreUsuarioYaRegistrado()
        self.usuarios[usuario.id] = usuario
        return usuario

    def buscar_por_correo(self, correo: str) -> Usuario | None:
        buscado = correo.strip().lower()
        return next((u for u in self.usuarios.values() if u.correo == buscado), None)

    def buscar_por_nombre_usuario(self, nombre_usuario: str) -> Usuario | None:
        buscado = nombre_usuario.strip()
        return next(
            (u for u in self.usuarios.values() if u.nombre_usuario == buscado), None
        )

    def buscar_por_id(self, id_usuario: UUID) -> Usuario | None:
        return self.usuarios.get(id_usuario)
