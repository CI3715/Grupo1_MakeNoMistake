use aplicacion::puertos::GeneradorIdentificadores;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct GeneradorUuid;

impl GeneradorUuid {
    pub fn nuevo() -> Self {
        GeneradorUuid
    }
}

impl GeneradorIdentificadores for GeneradorUuid {
    fn nuevo_id(&mut self) -> String {
        Uuid::new_v4().to_string()
    }
}
