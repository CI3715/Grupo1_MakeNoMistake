//! Identificadores locales derivados de los eventos del servidor.
//!
//! Las partes de un gasto de grupo llegan a cada dispositivo con el mismo id de
//! evento del servidor, pero cada instalación las registra en su propio momento
//! y con su propio dispositivo. Derivar el id local del id del servidor, con un
//! espacio de nombres fijo, garantiza que dos dispositivos que bajaron la misma
//! parte produzcan el mismo evento y que la fusión lo deje una sola vez.

use uuid::Uuid;

/// Espacio de nombres propio de Cuentas Claras para las partes de gasto de grupo.
/// Es constante: no depende del dispositivo, del momento ni de la cuenta elegida.
const ESPACIO_PARTES_DE_GRUPO: Uuid = Uuid::from_u128(0x9f6b1c4e3d2a5f809a176c4e2d8b0f31);

/// Deriva el identificador del evento local de una parte de gasto de grupo a
/// partir del identificador del evento del servidor (UUID v5).
///
/// La carga del evento conserva `id_evento_servidor`: la idempotencia de la
/// bajada sigue comprobándose por ese campo, así que un libro anterior que
/// guardó la parte con el id del servidor no se duplica al volver a bajarla.
pub fn id_evento_parte_de_grupo(id_evento_servidor: &str) -> String {
    Uuid::new_v5(&ESPACIO_PARTES_DE_GRUPO, id_evento_servidor.as_bytes()).to_string()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_mismo_evento_del_servidor_da_siempre_el_mismo_id() {
        assert_eq!(
            id_evento_parte_de_grupo("e-100"),
            id_evento_parte_de_grupo("e-100")
        );
    }

    #[test]
    fn eventos_distintos_del_servidor_dan_ids_distintos() {
        assert_ne!(
            id_evento_parte_de_grupo("e-100"),
            id_evento_parte_de_grupo("e-101")
        );
    }

    #[test]
    fn es_un_uuid_v5_con_el_espacio_de_nombres_fijo() {
        let id = id_evento_parte_de_grupo("e-100");
        let uuid = Uuid::parse_str(&id).expect("el id derivado es un UUID");
        assert_eq!(uuid.get_version_num(), 5);
        assert_eq!(id, "de79fe2b-77d9-5d2f-bbbb-1fc4207472cd");
    }
}
