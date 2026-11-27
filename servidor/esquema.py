"""Esquema de la base de datos del servidor (PostgreSQL).

Todo el DDL vive aquí para que `base_datos.py` se ocupe solo de las
conexiones. Las sentencias son idempotentes: se ejecutan al arrancar y no
hacen nada si las tablas y los índices ya existen. Al final se activa RLS en
todas las tablas de `public` y se retiran los permisos a los roles de la API
de Supabase cuando existen.

INV-16: aquí no hay tablas del libro personal (cuentas, movimientos, saldos,
tasas ni categorías). Solo se guarda lo que ven varias personas: amistades,
grupos, miembros, gastos y partes de cada gasto.

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
    # ---------------------------------------------------------------- cambios
    # Registro de cambios de la Entrega 7. Un evento por participante de un
    # gasto, con posición consecutiva por usuario (cursor). `id` se genera en
    # Python con UUIDv5 de (gasto_id, usuario_id): nunca cambia y permite al
    # libro ignorar el evento si baja dos veces (INV-13, INV-15). El registro
    # solo crece: el disparador de más abajo impide UPDATE y DELETE.
    """
    CREATE TABLE IF NOT EXISTS cambios (
        id               UUID PRIMARY KEY,
        usuario_id       UUID NOT NULL REFERENCES usuarios (id),
        posicion         BIGINT NOT NULL CHECK (posicion > 0),
        grupo_id         UUID NOT NULL REFERENCES grupos (id),
        nombre_grupo     VARCHAR(80) NOT NULL,
        gasto_id         UUID NOT NULL REFERENCES gastos (id),
        concepto         VARCHAR(200) NOT NULL,
        fecha            DATE NOT NULL,
        moneda           VARCHAR(3) NOT NULL,
        monto            NUMERIC(14, 2) NOT NULL CHECK (monto >= 0),
        incluye_redondeo BOOLEAN NOT NULL DEFAULT FALSE,
        creado_en        TIMESTAMPTZ NOT NULL DEFAULT now(),

        CONSTRAINT cambios_usuario_posicion_unico UNIQUE (usuario_id, posicion),
        CONSTRAINT cambios_un_evento_por_parte UNIQUE (gasto_id, usuario_id),
        CONSTRAINT cambios_nombre_grupo_no_vacio
            CHECK (btrim(nombre_grupo) <> ''),
        CONSTRAINT cambios_concepto_no_vacio
            CHECK (btrim(concepto) <> ''),
        CONSTRAINT cambios_moneda_iso CHECK (moneda ~ '^[A-Z]{3}$'),

        -- Solo quien tiene parte en el gasto recibe el evento (INV-17):
        -- un miembro excluido de la división no existe en partes_gasto.
        CONSTRAINT cambios_es_parte
            FOREIGN KEY (gasto_id, usuario_id)
            REFERENCES partes_gasto (gasto_id, usuario_id)
    )
    """,
    # El índice único (usuario_id, posicion) ya cubre la lectura por cursor.
    # Este otro sirve para las comprobaciones de la llave foránea compuesta.
    """
    CREATE INDEX IF NOT EXISTS cambios_por_gasto
        ON cambios (gasto_id)
    """,
    # ------------------------------------------------------ posiciones_usuario
    # Un contador por usuario, nunca global: así un cursor no puede saltarse
    # un evento confirmado después. Se actualiza dentro de la transacción del
    # gasto con ON CONFLICT DO UPDATE, que bloquea la fila del contador.
    """
    CREATE TABLE IF NOT EXISTS posiciones_usuario (
        usuario_id      UUID PRIMARY KEY REFERENCES usuarios (id),
        ultima_posicion BIGINT NOT NULL DEFAULT 0 CHECK (ultima_posicion >= 0)
    )
    """,
    # --------------------------------------------------- claves_idempotencia
    # La clave viaja en el encabezado Idempotency-Key; la huella es el SHA-256
    # del cuerpo canónico (incluida la lista ordenada de participantes). El
    # repositorio que la consulta y guarda se agregará después; aquí queda
    # creada la tabla para que el esquema ya esté completo desde el arranque.
    """
    CREATE TABLE IF NOT EXISTS claves_idempotencia (
        usuario_id    UUID NOT NULL REFERENCES usuarios (id),
        clave         VARCHAR(128) NOT NULL,
        huella_cuerpo VARCHAR(64) NOT NULL,
        gasto_id      UUID NOT NULL REFERENCES gastos (id),
        creado_en     TIMESTAMPTZ NOT NULL DEFAULT now(),

        PRIMARY KEY (usuario_id, clave),
        CONSTRAINT claves_clave_valida
            CHECK (length(btrim(clave)) BETWEEN 8 AND 128),
        CONSTRAINT claves_huella_hex
            CHECK (huella_cuerpo ~ '^[0-9a-f]{64}$')
    )
    """,
    # --------------------------------------------------------- solo se agrega
    """
    CREATE OR REPLACE FUNCTION cambios_solo_crecen()
    RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN
        RAISE EXCEPTION 'el registro de cambios solo acepta inserciones';
    END;
    $$
    """,
    # CREATE TRIGGER no admite IF NOT EXISTS, así que se comprueba en pg_trigger.
    """
    DO $$
    BEGIN
        IF NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'public.cambios'::regclass
              AND tgname = 'cambios_inmutables'
              AND NOT tgisinternal
        ) THEN
            CREATE TRIGGER cambios_inmutables
                BEFORE UPDATE OR DELETE ON public.cambios
                FOR EACH ROW EXECUTE FUNCTION cambios_solo_crecen();
        END IF;
    END
    $$
    """,
    # ------------------------------------------------------------------- RLS
    # El servidor entra con el rol dueño de las tablas, que no está sujeto a
    # las políticas: el RLS protege a los roles de la API de Supabase. El
    # bloque se ejecuta en cada arranque, así que cubre también las tablas
    # nuevas que añadan las entregas siguientes. Revocar a roles inexistentes
    # sería un error, por eso se comprueba antes en `pg_roles`.
    """
    DO $$
    DECLARE
        tabla record;
        roles text;
    BEGIN
        SELECT string_agg(format('%I', rolname), ', ')
          INTO roles
          FROM pg_roles
         WHERE rolname IN ('anon', 'authenticated');

        FOR tabla IN
            SELECT c.relname
              FROM pg_class c
              JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = 'public'
               AND c.relkind IN ('r', 'p')
        LOOP
            EXECUTE format(
                'ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', tabla.relname
            );

            IF roles IS NOT NULL THEN
                EXECUTE format(
                    'REVOKE ALL ON TABLE public.%I FROM %s', tabla.relname, roles
                );
            END IF;
        END LOOP;
    END
    $$
    """,
)
