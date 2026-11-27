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
    /// Incluye fracción de segundo porque el orden canónico del libro ordena por
    /// momento, dispositivo e identificador (Entrega 8). Con solo segundos, los
    /// eventos creados en el mismo instante se ordenarían por su UUID aleatorio y
    /// podrían aplicarse antes de aquello que referencian (una tasa eliminada
    /// antes de registrarla, por ejemplo).
    fn ahora_utc(&self) -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%S%.9fZ").to_string()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_instante_del_sistema_se_puede_ordenar_y_parsear() {
        let ahora = RelojSistema::nuevo().ahora_utc();
        assert!(chrono::DateTime::parse_from_rfc3339(&ahora).is_ok());
        assert!(
            ahora.contains('.'),
            "debe distinguir creaciones del mismo segundo"
        );
    }
}
