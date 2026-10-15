//! Adaptadores que implementan los puertos (almacenamiento, reloj, identificadores).

use std::time::{SystemTime, UNIX_EPOCH};

use aplicacion::{DispositivoOrigen, GeneradorIdentificadores, Reloj};
use uuid::Uuid;

/// Reloj real: devuelve la hora del sistema en ISO 8601 UTC ("2026-10-12T18:00:00Z").
pub struct RelojSistema;

impl Reloj for RelojSistema {
    fn ahora_utc(&self) -> String {
        let segundos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        formatear_iso_utc(segundos)
    }
}

/// Convierte segundos desde 1970-01-01 (UTC) a "AAAA-MM-DDTHH:MM:SSZ".
fn formatear_iso_utc(segundos: i64) -> String {
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    let (anio, mes, dia) = fecha_civil(dias);
    format!(
        "{anio:04}-{mes:02}-{dia:02}T{:02}:{:02}:{:02}Z",
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

/// Días desde 1970-01-01 a (año, mes, día) del calendario gregoriano.
fn fecha_civil(dias: i64) -> (i64, i64, i64) {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let anio = yoe + era * 400 + i64::from(mes <= 2);
    (anio, mes, dia)
}

/// Generador real de identificadores: UUID v4 como texto.
pub struct GeneradorUuid;

impl GeneradorIdentificadores for GeneradorUuid {
    fn nuevo_id(&mut self) -> String {
        Uuid::new_v4().to_string()
    }
}

/// Dispositivo de origen: UUID generado al crear el adaptador.
/// (Cuando haya persistencia, se leerá de la base en vez de generarse cada vez.)
pub struct DispositivoGenerado {
    id: String,
}

impl DispositivoGenerado {
    pub fn nuevo() -> DispositivoGenerado {
        DispositivoGenerado {
            id: Uuid::new_v4().to_string(),
        }
    }
}

impl Default for DispositivoGenerado {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl DispositivoOrigen for DispositivoGenerado {
    fn id_dispositivo(&self) -> String {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_es_1970() {
        assert_eq!(formatear_iso_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn fecha_conocida() {
        // 2026-10-14T12:30:45Z
        assert_eq!(formatear_iso_utc(1_791_981_045), "2026-10-14T12:30:45Z");
    }

    #[test]
    fn dia_bisiesto() {
        // 2024-02-29T00:00:00Z
        assert_eq!(formatear_iso_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn reloj_tiene_formato_iso() {
        let t = RelojSistema.ahora_utc();
        assert_eq!(t.len(), 20);
        assert!(t.ends_with('Z'));
    }

    #[test]
    fn uuid_tiene_36_caracteres() {
        assert_eq!(GeneradorUuid.nuevo_id().len(), 36);
        assert_eq!(DispositivoGenerado::nuevo().id_dispositivo().len(), 36);
    }
}
