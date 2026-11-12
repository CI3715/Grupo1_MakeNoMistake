"""Almacén en memoria para las pruebas de las rutas del servidor.

Imita a ``RepositorioUsuarios``, ``RepositorioGrupos`` y ``RepositorioAmistades``:
recorta los nombres, guarda el correo en minúsculas, valida unicidad y control
de acceso en grupos y amistades. Así las pruebas de las rutas corren de forma
aislada y determinista sin necesidad de PostgreSQL.
"""

from __future__ import annotations

from datetime import date, datetime, timezone
from uuid import UUID, uuid4

from repositorio_amistades import (
    Amistad,
    AmistadNoEncontrada,
    AmistadVista,
    AmistadYaRespondida,
    EstadoAmistad,
    InvitacionPendiente,
    YaSonAmigos,
)
from repositorio_grupos import (
    EstadoMiembro,
    Grupo,
    GrupoVista,
    InvitacionDeGrupoNoPendiente,
    Miembro,
    MiembroVista,
    Rol,
    YaEsMiembro,
)
from repositorio_usuarios import (
    CorreoYaRegistrado,
    NombreUsuarioYaRegistrado,
    Usuario,
)


class RepositorioEnMemoria:
    """Implementación en memoria de los repositorios de usuarios, grupos y amistades."""

    def __init__(self) -> None:
        self.usuarios: dict[UUID, Usuario] = {}
        self.grupos: dict[UUID, Grupo] = {}
        self.miembros: dict[tuple[UUID, UUID], Miembro] = {}
        self.amistades: dict[UUID, Amistad] = {}

    # ------------------------------------------------------------- Usuarios

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
        buscado = nombre_usuario.strip().lower()
        return next(
            (u for u in self.usuarios.values() if u.nombre_usuario.lower() == buscado),
            None,
        )

    def buscar_por_id(self, id_usuario: UUID) -> Usuario | None:
        return self.usuarios.get(id_usuario)

    # --------------------------------------------------------------- Grupos

    def crear_grupo(
        self,
        *,
        creado_por: UUID,
        nombre: str,
        descripcion: str | None,
        moneda: str,
        actividad: str | None,
        fecha_actividad: date | None,
        ahora: datetime,
    ) -> Grupo:
        grupo = Grupo(
            id=uuid4(),
            nombre=nombre.strip(),
            descripcion=descripcion.strip() if descripcion else None,
            moneda=moneda.strip().upper(),
            actividad=actividad.strip() if actividad else None,
            fecha_actividad=fecha_actividad,
            creado_por=creado_por,
            fecha_creacion=ahora,
        )
        self.grupos[grupo.id] = grupo

        # Creador nace como miembro administrador y aceptado (RF-COM-01)
        miembro_admin = Miembro(
            grupo_id=grupo.id,
            usuario_id=creado_por,
            rol=Rol.ADMINISTRADOR,
            estado=EstadoMiembro.ACEPTADO,
            invitado_por=None,
            fecha_invitacion=ahora,
            fecha_respuesta=ahora,
        )
        self.miembros[(grupo.id, creado_por)] = miembro_admin
        return grupo

    def buscar_grupo_por_id(self, id_grupo: UUID) -> Grupo | None:
        return self.grupos.get(id_grupo)

    def buscar_miembro(self, *, id_grupo: UUID, usuario_id: UUID) -> Miembro | None:
        return self.miembros.get((id_grupo, usuario_id))

    def listar_grupos_de(self, usuario_id: UUID) -> list[GrupoVista]:
        resultado: list[GrupoVista] = []
        for (g_id, u_id), miembro in self.miembros.items():
            if u_id == usuario_id:
                grupo = self.grupos.get(g_id)
                if grupo is not None:
                    creador = self.usuarios.get(grupo.creado_por)
                    nombre_creador = creador.nombre_visible if creador else ""
                    resultado.append(
                        GrupoVista(
                            grupo=grupo,
                            mi_rol=miembro.rol,
                            mi_estado=miembro.estado,
                            nombre_creador=nombre_creador,
                        )
                    )
        return resultado

    def listar_miembros_de_grupo(self, id_grupo: UUID) -> list[MiembroVista]:
        resultado: list[MiembroVista] = []
        for (g_id, u_id), miembro in self.miembros.items():
            if g_id == id_grupo:
                usuario = self.usuarios.get(u_id)
                if usuario is not None:
                    resultado.append(
                        MiembroVista(
                            usuario_id=u_id,
                            nombre_usuario=usuario.nombre_usuario,
                            nombre_visible=usuario.nombre_visible,
                            rol=miembro.rol,
                            estado=miembro.estado,
                        )
                    )
        return resultado

    def invitar_a_grupo(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        invitado_por: UUID,
        ahora: datetime,
    ) -> Miembro:
        clave = (id_grupo, usuario_id)
        if clave in self.miembros:
            raise YaEsMiembro()
        miembro = Miembro(
            grupo_id=id_grupo,
            usuario_id=usuario_id,
            rol=Rol.MIEMBRO,
            estado=EstadoMiembro.INVITADO,
            invitado_por=invitado_por,
            fecha_invitacion=ahora,
            fecha_respuesta=None,
        )
        self.miembros[clave] = miembro
        return miembro

    def responder_invitacion_grupo(
        self,
        *,
        id_grupo: UUID,
        usuario_id: UUID,
        aceptar: bool,
        ahora: datetime,
    ) -> Miembro | None:
        clave = (id_grupo, usuario_id)
        miembro = self.miembros.get(clave)
        if miembro is None or miembro.estado != EstadoMiembro.INVITADO:
            raise InvitacionDeGrupoNoPendiente()

        if aceptar:
            actualizado = Miembro(
                grupo_id=id_grupo,
                usuario_id=usuario_id,
                rol=miembro.rol,
                estado=EstadoMiembro.ACEPTADO,
                invitado_por=miembro.invitado_por,
                fecha_invitacion=miembro.fecha_invitacion,
                fecha_respuesta=ahora,
            )
            self.miembros[clave] = actualizado
            return actualizado
        else:
            # Rechazar borra la fila (ADR-011)
            del self.miembros[clave]
            return None

    # ------------------------------------------------------------ Amistades

    def crear_amistad(
        self, *, invitador_id: UUID, invitado_id: UUID, ahora: datetime
    ) -> Amistad:
        activa = self.buscar_activa_entre(invitador_id, invitado_id)
        if activa is not None:
            if activa.estado is EstadoAmistad.ACEPTADA:
                raise YaSonAmigos("Estas dos personas ya son amigas.")
            raise InvitacionPendiente(
                "Ya existe una invitación pendiente entre estas personas."
            )

        amistad = Amistad(
            id=uuid4(),
            invitador_id=invitador_id,
            invitado_id=invitado_id,
            estado=EstadoAmistad.PENDIENTE,
            fecha_invitacion=ahora.replace(microsecond=0),
            fecha_respuesta=None,
            fecha_baja=None,
        )
        self.amistades[amistad.id] = amistad
        return amistad

    def buscar_amistad_por_id(self, id_amistad: UUID) -> Amistad | None:
        return self.amistades.get(id_amistad)

    def buscar_activa_entre(self, uno: UUID, otro: UUID) -> Amistad | None:
        pareja = {uno, otro}
        return next(
            (
                a
                for a in self.amistades.values()
                if {a.invitador_id, a.invitado_id} == pareja
                and a.estado in (EstadoAmistad.PENDIENTE, EstadoAmistad.ACEPTADA)
            ),
            None,
        )

    def listar_amistades_de(self, usuario_id: UUID) -> list[AmistadVista]:
        resultado: list[AmistadVista] = []
        for a in self.amistades.values():
            if a.estado not in (EstadoAmistad.PENDIENTE, EstadoAmistad.ACEPTADA):
                continue
            if usuario_id not in (a.invitador_id, a.invitado_id):
                continue
            otro_id = a.invitado_id if a.invitador_id == usuario_id else a.invitador_id
            otro = self.usuarios.get(otro_id)
            otro_user = otro.nombre_usuario if otro else ""
            otro_vis = otro.nombre_visible if otro else ""
            resultado.append(
                AmistadVista(
                    id=a.id,
                    estado=a.estado,
                    otro_id=otro_id,
                    otro_nombre_usuario=otro_user,
                    otro_nombre_visible=otro_vis,
                    enviada_por_mi=(a.invitador_id == usuario_id),
                    fecha_invitacion=a.fecha_invitacion,
                    fecha_respuesta=a.fecha_respuesta,
                )
            )
        return sorted(
            resultado,
            key=lambda v: (v.estado != EstadoAmistad.ACEPTADA, v.otro_nombre_usuario),
        )

    def responder_amistad(
        self, *, id_amistad: UUID, aceptar: bool, ahora: datetime
    ) -> Amistad:
        amistad = self.buscar_amistad_por_id(id_amistad)
        if amistad is None:
            raise AmistadNoEncontrada("No existe una amistad con ese id.")
        if amistad.estado != EstadoAmistad.PENDIENTE:
            raise AmistadYaRespondida(
                "La invitación ya fue aceptada, rechazada o eliminada."
            )

        nuevo_estado = EstadoAmistad.ACEPTADA if aceptar else EstadoAmistad.RECHAZADA
        actualizada = Amistad(
            id=amistad.id,
            invitador_id=amistad.invitador_id,
            invitado_id=amistad.invitado_id,
            estado=nuevo_estado,
            fecha_invitacion=amistad.fecha_invitacion,
            fecha_respuesta=ahora.replace(microsecond=0),
            fecha_baja=None,
        )
        self.amistades[amistad.id] = actualizada
        return actualizada

    def eliminar_amistad(self, *, id_amistad: UUID, ahora: datetime) -> None:
        amistad = self.buscar_amistad_por_id(id_amistad)
        if amistad is None:
            raise AmistadNoEncontrada("No existe una amistad con ese id.")
        if amistad.estado in (EstadoAmistad.RECHAZADA, EstadoAmistad.ELIMINADA):
            raise AmistadYaRespondida("La amistad ya está cerrada.")

        actualizada = Amistad(
            id=amistad.id,
            invitador_id=amistad.invitador_id,
            invitado_id=amistad.invitado_id,
            estado=EstadoAmistad.ELIMINADA,
            fecha_invitacion=amistad.fecha_invitacion,
            fecha_respuesta=amistad.fecha_respuesta,
            fecha_baja=ahora.replace(microsecond=0),
        )
        self.amistades[amistad.id] = actualizada

    def son_amigos(self, uno_id: UUID, otro_id: UUID) -> bool:
        pareja = {uno_id, otro_id}
        return any(
            a.estado == EstadoAmistad.ACEPTADA
            and {a.invitador_id, a.invitado_id} == pareja
            for a in self.amistades.values()
        )

    def registrar_amistad_aceptada(
        self, uno_id: UUID, otro_id: UUID, ahora: datetime | None = None
    ) -> Amistad:
        """Ayudante de prueba para conectar dos personas como amigas."""
        f_ahora = ahora or datetime.now(timezone.utc).replace(microsecond=0)
        amistad = Amistad(
            id=uuid4(),
            invitador_id=uno_id,
            invitado_id=otro_id,
            estado=EstadoAmistad.ACEPTADA,
            fecha_invitacion=f_ahora,
            fecha_respuesta=f_ahora,
            fecha_baja=None,
        )
        self.amistades[amistad.id] = amistad
        return amistad


class RepositorioAmistadesEnMemoria:
    """Adaptador de amistades en memoria que implementa AlmacenAmistades."""

    def __init__(self, almacen: RepositorioEnMemoria) -> None:
        self._almacen = almacen

    def crear(
        self, *, invitador_id: UUID, invitado_id: UUID, ahora: datetime
    ) -> Amistad:
        return self._almacen.crear_amistad(
            invitador_id=invitador_id, invitado_id=invitado_id, ahora=ahora
        )

    def buscar_por_id(self, id_amistad: UUID) -> Amistad | None:
        return self._almacen.buscar_amistad_por_id(id_amistad)

    def buscar_activa_entre(self, uno: UUID, otro: UUID) -> Amistad | None:
        return self._almacen.buscar_activa_entre(uno, otro)

    def listar_de(self, usuario_id: UUID) -> list[AmistadVista]:
        return self._almacen.listar_amistades_de(usuario_id)

    def responder(self, *, id_amistad: UUID, aceptar: bool, ahora: datetime) -> Amistad:
        return self._almacen.responder_amistad(
            id_amistad=id_amistad, aceptar=aceptar, ahora=ahora
        )

    def eliminar(self, *, id_amistad: UUID, ahora: datetime) -> None:
        self._almacen.eliminar_amistad(id_amistad=id_amistad, ahora=ahora)

    def son_amigos(self, uno: UUID, otro: UUID) -> bool:
        return self._almacen.son_amigos(uno, otro)
