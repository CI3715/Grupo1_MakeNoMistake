-- Crea la base que usan las pruebas de integración del servidor.
--
-- Docker la ejecuta sola la primera vez que inicializa el volumen, con el
-- usuario de POSTGRES_USER (el de CUENTAS_CLARAS_DB_USUARIO), que queda como
-- dueño. Las pruebas hacen TRUNCATE sobre sus tablas, por eso van a una base
-- aparte y nunca a la de desarrollo ni a Supabase.
--
-- Si el volumen ya existe, este archivo no se vuelve a ejecutar. Para crearla
-- a mano después de borrarla:
--
--   docker compose exec postgres \
--     psql -U "$CUENTAS_CLARAS_DB_USUARIO" -d postgres \
--     -c "CREATE DATABASE cuentas_claras_pruebas;"

CREATE DATABASE cuentas_claras_pruebas;
