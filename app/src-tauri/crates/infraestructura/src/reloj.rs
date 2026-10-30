use aplicacion::puertos::Reloj;
use chrono::Utc;

#[derive(Debug, Default)]
pub struct RelojSistema;

impl RelojSistema {
    pub fn nuevo() -> Self {
        RelojSistema
    }
}

impl Reloj for RelojSistema {
    fn ahora_utc(&self) -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}
