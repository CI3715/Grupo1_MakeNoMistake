"""Esquema de la base de datos del servidor (PostgreSQL).

Todo el DDL vive aquí para que `base_datos.py` se ocupe solo de las
conexiones. Las sentencias son idempotentes: se ejecutan al arrancar y no
hacen nada si las tablas y los índices ya existen.

INV-16: aquí no hay tablas del libro personal (cuentas, movimientos, saldos,
tasas ni categorías). Solo se guarda lo que ven varias personas: amistades,
grupos, miembros, gastos y partes de cada gasto.

Los montos son `BIGINT` en céntimos y los porcentajes `SMALLINT` en
centésimas de punto: el dinero nunca se guarda como coma flotante (RNF-08).
"""

from __future__ import annotations

SENTENCIAS_ESQUEMA: tuple[str, ...] = (
    # ------------------------------------------------------------- usuarios
    """
    CREATE TABLE IF NOT EXISTS usuarios (
        id              UUID PRIMARY KEY,
        nombre_usuario  VARCHAR(30) NOT NULL,
        nombre_visible  VARCHAR(50) NOT NULL,
        correo          VARCHAR(254) NOT NULL,
        hash_contrasena TEXT NOT NULL,
        fecha_alta      TIMESTAMPTZ NOT NULL,
        CONSTRAINT usuarios_correo_en_minusculas
            CHECK (correo = lower(correo)),
        CONSTRAINT usuarios_nombre_en_minusculas
            CHECK (nombre_usuario = lower(nombre_usuario))
    )
    """,
    """
    CREATE UNIQUE INDEX IF NOT EXISTS usuarios_correo_unico
        ON usuarios (lower(correo))
    """,
    """
    CREATE UNIQUE INDEX IF NOT EXISTS usuarios_nombre_usuario_unico
        ON usuarios (nombre_usuario)
    """,
    # ------------------------------------------------------------ amistades
    # Una fila por intento de amistad. `pendiente` es una invitación; solo
    # pasa a `aceptada` cuando el destinatario responde (INV-10). `eliminada`
    # conserva el histórico: la fila nunca se borra.
    """
    CREATE TABLE IF NOT EXISTS amistades (
        id               UUID PRIMARY KEY,
        invitador_id     UUID NOT NULL REFERENCES usuarios (id),
        invitado_id      UUID NOT NULL REFERENCES usuarios (id),
        estado           VARCHAR(10) NOT NULL,
        fecha_invitacion TIMESTAMPTZ NOT NULL,
        fecha_respuesta  TIMESTAMPTZ,
        fecha_baja       TIMESTAMPTZ,
        CONSTRAINT amistades_estado_valido CHECK (
            estado IN ('pendiente', 'aceptada', 'rechazada', 'eliminada')
        ),
        CONSTRAINT amistades_no_uno_mismo
            CHECK (invitador_id <> invitado_id),
        CONSTRAINT amistades_fechas_coherentes CHECK (
            (estado = 'pendiente'
                AND fecha_respuesta IS NULL AND fecha_baja IS NULL)
            OR (estado = 'aceptada'
                AND fecha_respuesta IS NOT NULL AND fecha_baja IS NULL)
            OR (estado = 'rechazada'
                AND fecha_respuesta IS NOT NULL AND fecha_baja IS NULL)
            OR (estado = 'eliminada'
                AND fecha_respuesta IS NOT NULL AND fecha_baja IS NOT NULL)
        )
    )
    """,
    # Una sola amistad viva por pareja, se escriba en el sentido que se
    # escriba: el índice ordena la pareja con least/greatest. Es parcial para
    # que un rechazo o una baja no impidan un intento nuevo (RF-SOC-05).
    """
    CREATE UNIQUE INDEX IF NOT EXISTS amistades_pareja_activa
        ON amistades (
            least(invitador_id, invitado_id),
            greatest(invitador_id, invitado_id)
        )
        WHERE estado IN ('pendiente', 'aceptada')
    """,
    """
    CREATE INDEX IF NOT EXISTS amistades_por_invitador
        ON amistades (invitador_id)
    """,
    """
    CREATE INDEX IF NOT EXISTS amistades_por_invitado
        ON amistades (invitado_id)
    """,
    # ---------------------------------------------------------------- grupos
    """
    CREATE TABLE IF NOT EXISTS grupos (
        id              UUID PRIMARY KEY,
        nombre          VARCHAR(80) NOT NULL,
        descripcion     VARCHAR(500),
        moneda          VARCHAR(3) NOT NULL,
        actividad       VARCHAR(120),
        fecha_actividad DATE,
        creado_por      UUID NOT NULL REFERENCES usuarios (id),
        fecha_creacion  TIMESTAMPTZ NOT NULL,
        CONSTRAINT grupos_nombre_no_vacio CHECK (btrim(nombre) <> ''),
        CONSTRAINT grupos_moneda_iso CHECK (moneda ~ '^[A-Z]{3}$')
    )
    """,
    # -------------------------------------------------------------- miembros
    # `invitado` no ve los gastos ni los balances: solo `aceptado` es
    # miembro de verdad (RF-COM-02, INV-17). Rechazar una invitación borra la
    # fila, porque la especificación solo admite estos dos estados.
    """
    CREATE TABLE IF NOT EXISTS miembros (
        grupo_id         UUID NOT NULL REFERENCES grupos (id),
        usuario_id       UUID NOT NULL REFERENCES usuarios (id),
        rol              VARCHAR(13) NOT NULL,
        estado           VARCHAR(9) NOT NULL,
        invitado_por     UUID REFERENCES usuarios (id),
        fecha_invitacion TIMESTAMPTZ NOT NULL,
        fecha_respuesta  TIMESTAMPTZ,
        PRIMARY KEY (grupo_id, usuario_id),
        CONSTRAINT miembros_rol_valido
            CHECK (rol IN ('administrador', 'miembro')),
        CONSTRAINT miembros_estado_valido
            CHECK (estado IN ('invitado', 'aceptado')),
        CONSTRAINT miembros_fechas_coherentes CHECK (
            (estado = 'invitado' AND fecha_respuesta IS NULL)
            OR (estado = 'aceptado' AND fecha_respuesta IS NOT NULL)
        )
    )
    """,
    """
    CREATE INDEX IF NOT EXISTS miembros_por_usuario
        ON miembros (usuario_id)
    """,
    # ---------------------------------------------------------------- gastos
    # Las claves foráneas compuestas garantizan en la base que el pagador y
    # quien registra son miembros del grupo. Que estén `aceptado` se
    # comprueba en el caso de uso: una clave foránea no filtra por columna.
    """
    CREATE TABLE IF NOT EXISTS gastos (
        id             UUID PRIMARY KEY,
        grupo_id       UUID NOT NULL REFERENCES grupos (id),
        concepto       VARCHAR(200) NOT NULL,
        monto          BIGINT NOT NULL CHECK (monto > 0),
        fecha          DATE NOT NULL,
        pagador_id     UUID NOT NULL,
        regla          VARCHAR(11) NOT NULL,
        registrado_por UUID NOT NULL,
        fecha_registro TIMESTAMPTZ NOT NULL,
        CONSTRAINT gastos_concepto_no_vacio
            CHECK (btrim(concepto) <> ''),
        CONSTRAINT gastos_regla_valida
            CHECK (regla IN ('iguales', 'porcentajes', 'montos')),
        CONSTRAINT gastos_pagador_miembro
            FOREIGN KEY (grupo_id, pagador_id)
            REFERENCES miembros (grupo_id, usuario_id),
        CONSTRAINT gastos_registrador_miembro
            FOREIGN KEY (grupo_id, registrado_por)
            REFERENCES miembros (grupo_id, usuario_id)
    )
    """,
    """
    CREATE INDEX IF NOT EXISTS gastos_por_grupo
        ON gastos (grupo_id, fecha, id)
    """,
    # ---------------------------------------------------------- partes_gasto
    # `monto` es el importe exacto de esa persona en céntimos, ya con el
    # residuo aplicado si le tocó. `lleva_residuo` permite mostrar "incluye
    # 0,01 de redondeo" (INV-07). `porcentaje` solo se llena con la regla
    # `porcentajes`, en centésimas de punto (3333 = 33,33 %).
    """
    CREATE TABLE IF NOT EXISTS partes_gasto (
        gasto_id      UUID NOT NULL REFERENCES gastos (id) ON DELETE CASCADE,
        usuario_id    UUID NOT NULL REFERENCES usuarios (id),
        monto         BIGINT NOT NULL CHECK (monto >= 0),
        lleva_residuo BOOLEAN NOT NULL DEFAULT FALSE,
        porcentaje    SMALLINT
                      CHECK (porcentaje BETWEEN 0 AND 10000),
        PRIMARY KEY (gasto_id, usuario_id)
    )
    """,
    """
    CREATE INDEX IF NOT EXISTS partes_por_usuario
        ON partes_gasto (usuario_id)
    """,
)
