"""Reglas de registro e inicio de sesión (RF-SOC-01, RF-SOC-02).

Funciones puras: no tocan la base de datos, la red ni el reloj. Cada error
indica el campo que falla y un mensaje en español que nunca repite el valor
enviado (RNF-05).
"""

import re
from dataclasses import dataclass

MIN_NOMBRE_USUARIO = 3
MAX_NOMBRE_USUARIO = 30
MAX_NOMBRE_VISIBLE = 50
MAX_CORREO = 254
MIN_CONTRASENA = 10

_PATRON_NOMBRE_USUARIO = re.compile(r"[a-z0-9._]+")
# Formato básico: algo antes de la arroba, y un dominio con al menos un punto.
_PATRON_CORREO = re.compile(r"[^@\s]+@[^@\s.]+(\.[^@\s.]+)+")

MENSAJE_OBLIGATORIO = "Este campo es obligatorio."
MENSAJE_NOMBRE_USUARIO_LONGITUD = (
    f"El nombre de usuario debe tener de {MIN_NOMBRE_USUARIO} "
    f"a {MAX_NOMBRE_USUARIO} caracteres."
)
MENSAJE_NOMBRE_USUARIO_CARACTERES = (
    "El nombre de usuario solo puede tener letras minúsculas, dígitos, "
    "punto y guion bajo."
)
MENSAJE_NOMBRE_VISIBLE_VACIO = "El nombre visible no puede estar vacío."
MENSAJE_NOMBRE_VISIBLE_LARGO = (
    f"El nombre visible no puede tener más de {MAX_NOMBRE_VISIBLE} caracteres."
)
MENSAJE_CORREO_INVALIDO = "El correo no tiene un formato válido."
MENSAJE_CONTRASENA_CORTA = (
    f"La contraseña debe tener al menos {MIN_CONTRASENA} caracteres."
)


@dataclass(frozen=True)
class ErrorCampo:
    campo: str
    mensaje: str


@dataclass(frozen=True)
class DatosRegistro:
    """Datos de registro ya validados y normalizados."""

    nombre_usuario: str
    nombre_visible: str
    correo: str
    contrasena: str


def normalizar_correo(correo: str) -> str:
    """El correo se guarda y se compara sin espacios y en minúsculas."""
    return correo.strip().lower()


def validar_nombre_usuario(valor: str | None) -> ErrorCampo | None:
    """De 3 a 30 caracteres: minúsculas, dígitos, punto y guion bajo.

    No se normaliza: «AnaPerez» se rechaza en lugar de convertirse.
    """
    if valor is None or valor == "":
        return ErrorCampo("nombre_usuario", MENSAJE_OBLIGATORIO)
    if not MIN_NOMBRE_USUARIO <= len(valor) <= MAX_NOMBRE_USUARIO:
        return ErrorCampo("nombre_usuario", MENSAJE_NOMBRE_USUARIO_LONGITUD)
    if not _PATRON_NOMBRE_USUARIO.fullmatch(valor):
        return ErrorCampo("nombre_usuario", MENSAJE_NOMBRE_USUARIO_CARACTERES)
    return None


def validar_nombre_visible(valor: str | None) -> ErrorCampo | None:
    """Se recorta; no puede quedar vacío ni pasar de 50 caracteres."""
    if valor is None:
        return ErrorCampo("nombre_visible", MENSAJE_OBLIGATORIO)
    recortado = valor.strip()
    if recortado == "":
        return ErrorCampo("nombre_visible", MENSAJE_NOMBRE_VISIBLE_VACIO)
    if len(recortado) > MAX_NOMBRE_VISIBLE:
        return ErrorCampo("nombre_visible", MENSAJE_NOMBRE_VISIBLE_LARGO)
    return None


def validar_correo(valor: str | None) -> ErrorCampo | None:
    if valor is None or valor.strip() == "":
        return ErrorCampo("correo", MENSAJE_OBLIGATORIO)
    correo = normalizar_correo(valor)
    if len(correo) > MAX_CORREO or not _PATRON_CORREO.fullmatch(correo):
        return ErrorCampo("correo", MENSAJE_CORREO_INVALIDO)
    return None


def validar_contrasena(valor: str | None) -> ErrorCampo | None:
    """Al menos 10 caracteres. No se recorta: los espacios cuentan."""
    if valor is None or valor == "":
        return ErrorCampo("contrasena", MENSAJE_OBLIGATORIO)
    if len(valor) < MIN_CONTRASENA:
        return ErrorCampo("contrasena", MENSAJE_CONTRASENA_CORTA)
    return None


def validar_registro(
    nombre_usuario: str | None,
    nombre_visible: str | None,
    correo: str | None,
    contrasena: str | None,
) -> tuple[DatosRegistro | None, list[ErrorCampo]]:
    """Valida todos los campos a la vez y devuelve todos los errores juntos.

    Si no hay errores, devuelve los datos normalizados (nombre visible
    recortado y correo en minúsculas).
    """
    posibles = [
        validar_nombre_usuario(nombre_usuario),
        validar_nombre_visible(nombre_visible),
        validar_correo(correo),
        validar_contrasena(contrasena),
    ]
    errores = [error for error in posibles if error is not None]
    if errores:
        return None, errores
    # Sin errores, ningún campo es None.
    assert nombre_usuario is not None and nombre_visible is not None
    assert correo is not None and contrasena is not None
    datos = DatosRegistro(
        nombre_usuario=nombre_usuario,
        nombre_visible=nombre_visible.strip(),
        correo=normalizar_correo(correo),
        contrasena=contrasena,
    )
    return datos, []


def validar_credenciales(
    correo: str | None, contrasena: str | None
) -> list[ErrorCampo]:
    """Solo comprueba que vengan correo y contraseña.

    No valida el formato: cualquier credencial que no coincida recibe el mismo
    401, para no dar pistas sobre qué cuentas existen.
    """
    errores = []
    if correo is None or correo.strip() == "":
        errores.append(ErrorCampo("correo", MENSAJE_OBLIGATORIO))
    if contrasena is None or contrasena == "":
        errores.append(ErrorCampo("contrasena", MENSAJE_OBLIGATORIO))
    return errores
