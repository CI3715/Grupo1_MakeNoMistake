"""Esquema de la base de datos del servidor (PostgreSQL).

Todo el DDL vive aquí para que `base_datos.py` se ocupe solo de las
conexiones. Las sentencias son idempotentes: se ejecutan al arrancar y no
hacen nada si las tablas y los índices ya existen.

INV-16: aquí no hay tablas del libro personal (cuentas, movimientos, saldos,
tasas ni categorías). Solo se guarda lo que ven varias personas: amistades,
grupos, miembros, gastos y partes de cada gasto y, desde la Entrega 7, el
registro de cambios de cada persona (su parte de cada gasto), la última
posición de ese registro y las claves de idempotencia (ADR-015).

Los montos son `NUMERIC(14, 2)` (euros con dos decimales exactos) y los
porcentajes `SMALLINT` en centésimas de punto. El dinero nunca se guarda como
coma flotante: PostgreSQL calcula y compara `NUMERIC` con precisión decimal
(RNF-08), y Python usa `Decimal`, no `float`.
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
        monto          NUMERIC(14, 2) NOT NULL CHECK (monto > 0),
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
    # `monto` es el importe exacto de esa persona en euros con dos decimales,
    # ya con el residuo aplicado si le tocó. `lleva_residuo` permite mostrar
    # "incluye 0,01 de redondeo" (INV-07). `porcentaje` solo se llena con la
    # regla `porcentajes`, en centésimas de punto (3333 = 33,33 %).
    """
    CREATE TABLE IF NOT EXISTS partes_gasto (
        gasto_id      UUID NOT NULL REFERENCES gastos (id) ON DELETE CASCADE,
        usuario_id    UUID NOT NULL REFERENCES usuarios (id),
        monto         NUMERIC(14, 2) NOT NULL CHECK (monto >= 0),
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
    # --------------------------------------------------------------- cambios
    # Registro de cambios de cada persona (Entrega 7, ADR-015): una fila por
    # participante de cada gasto, con su parte. Es lo que entrega
    # `GET /cambios`. Solo crece: los disparadores de abajo rechazan UPDATE y
    # DELETE (INV-13). No hay columna `tipo`: hoy todos los eventos son
    # `gasto_de_grupo` y la ruta lo escribe como valor fijo. `grupo_nombre` es
    # una copia del nombre al registrar el gasto, para que el movimiento de la
    # app no cambie si luego se renombra el grupo. INV-16: ni cuenta ni saldo.
    """
    CREATE TABLE IF NOT EXISTS cambios (
        id               UUID PRIMARY KEY,
        usuario_id       UUID NOT NULL REFERENCES usuarios (id),
        posicion         BIGINT NOT NULL CHECK (posicion >= 1),
        gasto_id         UUID NOT NULL REFERENCES gastos (id),
        grupo_id         UUID NOT NULL REFERENCES grupos (id),
        grupo_nombre     VARCHAR(80) NOT NULL,
        concepto         VARCHAR(200) NOT NULL,
        fecha            DATE NOT NULL,
        moneda           VARCHAR(3) NOT NULL,
        monto            NUMERIC(14, 2) NOT NULL CHECK (monto > 0),
        incluye_redondeo BOOLEAN NOT NULL,
        fecha_registro   TIMESTAMPTZ NOT NULL,
        CONSTRAINT cambios_posicion_unica UNIQUE (usuario_id, posicion),
        CONSTRAINT cambios_un_evento_por_gasto UNIQUE (usuario_id, gasto_id),
        CONSTRAINT cambios_moneda_iso CHECK (moneda ~ '^[A-Z]{3}$')
    )
    """,
    """
    CREATE OR REPLACE FUNCTION cambios_solo_crece() RETURNS trigger
    LANGUAGE plpgsql AS $$
    BEGIN
        RAISE EXCEPTION 'El registro de cambios no se modifica ni se borra.'
            USING ERRCODE = 'restrict_violation';
    END
    $$
    """,
    "DROP TRIGGER IF EXISTS cambios_sin_update ON cambios",
    """
    CREATE TRIGGER cambios_sin_update
        BEFORE UPDATE ON cambios
        FOR EACH ROW EXECUTE FUNCTION cambios_solo_crece()
    """,
    "DROP TRIGGER IF EXISTS cambios_sin_delete ON cambios",
    """
    CREATE TRIGGER cambios_sin_delete
        BEFORE DELETE ON cambios
        FOR EACH ROW EXECUTE FUNCTION cambios_solo_crece()
    """,
    # ---------------------------------------------------- posiciones_usuario
    # Última posición usada en el registro de cada persona. El registro de un
    # gasto bloquea estas filas (SELECT ... FOR UPDATE) para que dos gastos a
    # la vez no le den la misma posición a nadie.
    """
    CREATE TABLE IF NOT EXISTS posiciones_usuario (
        usuario_id      UUID PRIMARY KEY REFERENCES usuarios (id),
        ultima_posicion BIGINT NOT NULL DEFAULT 0
                        CHECK (ultima_posicion >= 0)
    )
    """,
    # --------------------------------------------------- claves_idempotencia
    # Una clave consumida por persona (alcance por usuario): qué gasto creó y
    # la huella SHA-256 del cuerpo canónico (RNF-17, ESC-09). Se guarda en la
    # misma transacción que el gasto: si el gasto no queda, la clave tampoco.
    """
    CREATE TABLE IF NOT EXISTS claves_idempotencia (
        usuario_id     UUID NOT NULL REFERENCES usuarios (id),
        clave          VARCHAR(128) NOT NULL,
        huella         CHAR(64) NOT NULL,
        gasto_id       UUID NOT NULL REFERENCES gastos (id),
        fecha_registro TIMESTAMPTZ NOT NULL,
        PRIMARY KEY (usuario_id, clave),
        CONSTRAINT claves_idempotencia_visible
            CHECK (clave ~ '^[!-~]{8,128}$')
    )
    """,
    # --------------------------------------------------------- migración 6.1
    # Las primeras pruebas de la Entrega 6 crearon las columnas como BIGINT en
    # céntimos. El contrato definitivo guarda euros exactos en NUMERIC(14, 2),
    # así que se convierten una sola vez. El bloque comprueba el tipo antes de
    # tocar nada: en una base nueva las tablas ya nacen NUMERIC y no hace nada.
    """
    DO $$
    BEGIN
        IF EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = current_schema()
              AND table_name = 'gastos'
              AND column_name = 'monto'
              AND data_type = 'bigint'
        ) THEN
            ALTER TABLE gastos
                ALTER COLUMN monto TYPE NUMERIC(14, 2)
                USING (monto::numeric / 100);
        END IF;

        IF EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = current_schema()
              AND table_name = 'partes_gasto'
              AND column_name = 'monto'
              AND data_type = 'bigint'
        ) THEN
            ALTER TABLE partes_gasto
                ALTER COLUMN monto TYPE NUMERIC(14, 2)
                USING (monto::numeric / 100);
        END IF;
    END
    $$
    """,
)
