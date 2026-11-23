"""Compara ``servidor/openapi.yaml`` con el ``/openapi.json`` que genera FastAPI.

Uso, desde la carpeta ``servidor/``::

    uv run --with pyyaml python scripts/comparar_openapi.py

PyYAML no es una dependencia del proyecto (no toca ``pyproject.toml`` ni
``uv.lock``): ``--with`` lo instala solo para esta ejecución.

Qué compara
-----------
* Rutas y métodos: los mismos en los dos documentos.
* ``operationId``, parámetros (dónde, nombre y si son obligatorios) y seguridad.
* Códigos de respuesta, con el límite de D-14: las rutas de amistades y grupos
  arman su respuesta a mano y ``/openapi.json`` solo muestra de ellas el código
  de éxito. Por eso: todo código que declare el código debe estar en el YAML; y si
  la ruta declara errores, los dos conjuntos deben ser idénticos.
* Nunca aparece 422 (los errores de tipo también son 400).
* El esquema ``tokenBearer``.
* Lo que exige la Entrega 7: ``GET /cambios`` y el encabezado ``Idempotency-Key``
  en ``POST /grupos/{id_grupo}/gastos`` con sus códigos 200, 201, 400, 401, 404
  y 409.

Sale con código 1 si hay diferencias. Los avisos no hacen fallar.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

RAIZ = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(RAIZ))

METODOS = ("get", "post", "put", "patch", "delete")
Operaciones = dict[tuple[str, str], dict[str, Any]]

# Lo que la Entrega 7 añade al contrato (RF-COM-10, RNF-17).
CODIGOS_GASTO_E7 = {"200", "201", "400", "401", "404", "409"}
CODIGOS_CAMBIOS_E7 = {"200", "400", "401"}
CAMPOS_RESPUESTA_CAMBIOS = {"eventos", "cursor", "hay_mas"}
RUTA_GASTOS = "/grupos/{id_grupo}/gastos"


def resolver(doc: dict[str, Any], nodo: Any) -> Any:
    """Sigue los ``$ref`` locales (``#/components/...``) hasta el contenido."""
    pasos = 0
    while isinstance(nodo, dict) and "$ref" in nodo:
        ref = str(nodo["$ref"])
        if not ref.startswith("#/"):
            raise ValueError(f"solo se admiten $ref locales: {ref}")
        nodo = doc
        for parte in ref[2:].split("/"):
            nodo = nodo[parte]
        pasos += 1
        if pasos > 20:
            raise ValueError(f"$ref circular: {ref}")
    return nodo


def operaciones(doc: dict[str, Any]) -> Operaciones:
    salida: Operaciones = {}
    for ruta, item in doc.get("paths", {}).items():
        for metodo in METODOS:
            if metodo in item:
                salida[(metodo.upper(), ruta)] = item[metodo]
    return salida


def parametros(
    doc: dict[str, Any], ruta: str, op: dict[str, Any]
) -> set[tuple[str, str, bool]]:
    """Conjunto de (dónde, nombre en minúsculas, obligatorio)."""
    crudos = list(doc["paths"][ruta].get("parameters", [])) + list(
        op.get("parameters", [])
    )
    encontrados = set()
    for crudo in crudos:
        p = resolver(doc, crudo)
        obligatorio = bool(p.get("required", False)) or p["in"] == "path"
        encontrados.add((p["in"], str(p["name"]).lower(), obligatorio))
    return encontrados


def codigos(op: dict[str, Any]) -> set[str]:
    return {str(c) for c in op.get("responses", {})}


def seguridad(op: dict[str, Any]) -> list[str]:
    return sorted(nombre for req in op.get("security", []) for nombre in req)


def esquema_de(doc: dict[str, Any], op: dict[str, Any], codigo: str) -> Any:
    """El esquema JSON de la respuesta ``codigo``, o None si no se declara."""
    respuesta = op.get("responses", {}).get(codigo)
    if respuesta is None:
        return None
    contenido = resolver(doc, respuesta).get("content", {})
    esquema = contenido.get("application/json", {}).get("schema")
    return None if esquema is None else resolver(doc, esquema)


def ref_de(doc: dict[str, Any], op: dict[str, Any], codigo: str) -> str | None:
    """Nombre del esquema (``ErroresPorCampo``...) que usa una respuesta."""
    respuesta = op.get("responses", {}).get(codigo)
    if respuesta is None:
        return None
    respuesta = resolver(doc, respuesta)
    esquema = respuesta.get("content", {}).get("application/json", {}).get("schema")
    if isinstance(esquema, dict) and "$ref" in esquema:
        return str(esquema["$ref"]).rsplit("/", 1)[-1]
    return None


def comparar(
    yaml_doc: dict[str, Any], json_doc: dict[str, Any]
) -> tuple[list[str], list[str]]:
    """Devuelve (errores, avisos)."""
    errores: list[str] = []
    avisos: list[str] = []
    en_yaml = operaciones(yaml_doc)
    en_json = operaciones(json_doc)

    for clave in sorted(set(en_yaml) - set(en_json)):
        errores.append(f"{clave[0]} {clave[1]}: está en el yaml y no en el código")
    for clave in sorted(set(en_json) - set(en_yaml)):
        errores.append(f"{clave[0]} {clave[1]}: está en el código y no en el yaml")

    for clave in sorted(set(en_yaml) & set(en_json)):
        metodo, ruta = clave
        nombre = f"{metodo} {ruta}"
        y, j = en_yaml[clave], en_json[clave]

        if j.get("operationId") != y.get("operationId"):
            errores.append(
                f"{nombre}: operationId distinto "
                f"(yaml {y.get('operationId')!r}, código {j.get('operationId')!r})"
            )

        py = parametros(yaml_doc, ruta, y)
        pj = parametros(json_doc, ruta, j)
        if py != pj:
            errores.append(
                f"{nombre}: parámetros distintos "
                f"(solo yaml {sorted(py - pj)}, solo código {sorted(pj - py)})"
            )

        cy, cj = codigos(y), codigos(j)
        if "422" in cy or "422" in cj:
            errores.append(f"{nombre}: no debe documentar 422")
        if cj - cy:
            errores.append(
                f"{nombre}: el código declara {sorted(cj - cy)} y el yaml no"
            )
        con_errores = any(not c.startswith("2") for c in cj)
        if con_errores and cy != cj:
            errores.append(
                f"{nombre}: la ruta declara errores, los códigos deben coincidir "
                f"(yaml {sorted(cy)}, código {sorted(cj)})"
            )

        sy, sj = seguridad(y), seguridad(j)
        if sy != sj:
            errores.append(f"{nombre}: seguridad distinta (yaml {sy}, código {sj})")

        for codigo, esperado in (("400", "ErroresPorCampo"), ("401", None)):
            usado = ref_de(yaml_doc, y, codigo)
            if esperado and usado not in (None, esperado):
                avisos.append(f"{nombre}: el 400 usa {usado} y no {esperado}")

    # Esquema Bearer.
    for nombre, esquema in (
        json_doc.get("components", {}).get("securitySchemes", {}).items()
    ):
        destino = yaml_doc.get("components", {}).get("securitySchemes", {}).get(nombre)
        if destino is None:
            errores.append(f"securitySchemes.{nombre}: falta en openapi.yaml")
            continue
        for campo in ("type", "scheme"):
            if destino.get(campo) != esquema.get(campo):
                errores.append(f"securitySchemes.{nombre}.{campo}: distinto")

    errores += exigido_entrega_7(yaml_doc, json_doc, en_yaml, en_json)
    return errores, avisos


def exigido_entrega_7(
    yaml_doc: dict[str, Any],
    json_doc: dict[str, Any],
    en_yaml: Operaciones,
    en_json: Operaciones,
) -> list[str]:
    errores: list[str] = []

    cambios = ("GET", "/cambios")
    if cambios not in en_yaml:
        errores.append("GET /cambios: falta en openapi.yaml")
    else:
        y = en_yaml[cambios]
        if ("query", "desde", False) not in parametros(yaml_doc, "/cambios", y):
            errores.append("GET /cambios: falta el parámetro opcional `desde`")
        if not CODIGOS_CAMBIOS_E7 <= codigos(y):
            faltan = sorted(CODIGOS_CAMBIOS_E7 - codigos(y))
            errores.append(f"GET /cambios: el yaml no documenta {faltan}")
        if seguridad(y) != ["tokenBearer"]:
            errores.append("GET /cambios: debe exigir tokenBearer")
        esquema = esquema_de(yaml_doc, y, "200")
        propiedades = set((esquema or {}).get("properties", {}))
        if not CAMPOS_RESPUESTA_CAMBIOS <= propiedades:
            errores.append(
                "GET /cambios: el 200 debe traer "
                f"{sorted(CAMPOS_RESPUESTA_CAMBIOS)}; trae {sorted(propiedades)}"
            )
        if cambios not in en_json:
            errores.append("GET /cambios: falta en el código")
    nombres = yaml_doc.get("components", {}).get("schemas", {})
    if not any("Evento" in nombre for nombre in nombres):
        errores.append("components.schemas: falta el esquema del evento")

    gastos = ("POST", RUTA_GASTOS)
    if gastos not in en_yaml:
        errores.append(f"POST {RUTA_GASTOS}: falta en openapi.yaml")
    else:
        y = en_yaml[gastos]
        clave = ("header", "idempotency-key")
        if not any(p[:2] == clave for p in parametros(yaml_doc, RUTA_GASTOS, y)):
            errores.append("POST gastos: el yaml no documenta Idempotency-Key")
        if gastos in en_json and not any(
            p[:2] == clave for p in parametros(json_doc, RUTA_GASTOS, en_json[gastos])
        ):
            errores.append("POST gastos: el código no lee Idempotency-Key")
        if not CODIGOS_GASTO_E7 <= codigos(y):
            faltan = sorted(CODIGOS_GASTO_E7 - codigos(y))
            errores.append(f"POST gastos: el yaml no documenta {faltan}")
    return errores


def main() -> int:
    import yaml  # type: ignore[import-untyped]

    from main import app  # aquí para poder probar `comparar` sin FastAPI

    yaml_doc = yaml.safe_load((RAIZ / "openapi.yaml").read_text(encoding="utf-8"))
    json_doc = app.openapi()
    errores, avisos = comparar(yaml_doc, json_doc)
    for aviso in avisos:
        print(f"AVISO  {aviso}")
    for error in errores:
        print(f"ERROR  {error}")
    print(
        f"{len(operaciones(yaml_doc))} operaciones en el yaml, "
        f"{len(operaciones(json_doc))} en el código."
    )
    print("HAY DIFERENCIAS." if errores else "openapi.yaml coincide con /openapi.json.")
    return 1 if errores else 0


if __name__ == "__main__":
    raise SystemExit(main())
