//! Identidad canónica de las categorías de fábrica.
//!
//! El dominio es la fuente de verdad de los identificadores
//! (`gasto:alimentacion`, `ingreso:salario`, ...). SQLite usó históricamente
//! identificadores `cat-fabrica-<tipo>-<slug>`; este módulo los traduce al
//! proyectar para que los eventos ya guardados sigan siendo válidos sin
//! reescribir el libro inmutable.

use dominio::{Categoria, TipoMovimiento, categorias_de_fabrica};

/// Convierte el nombre de una categoría en el fragmento de identificador que
/// usaba la siembra histórica de SQLite.
fn slug(texto: &str) -> String {
    texto
        .to_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
        .replace('ñ', "n")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Identificador histórico (`cat-fabrica-...`) que corresponde a una categoría
/// del catálogo de fábrica.
pub fn id_categoria_legado(categoria: &Categoria) -> String {
    let tipo = match categoria.tipo {
        TipoMovimiento::Gasto => "egreso",
        TipoMovimiento::Ingreso => "ingreso",
    };
    format!("cat-fabrica-{tipo}-{}", slug(&categoria.nombre))
}

/// Traduce un identificador histórico de fábrica a su identificador canónico
/// del dominio. Cualquier otro identificador (por ejemplo, el de una categoría
/// creada por la persona usuaria) se conserva sin cambios.
pub fn id_categoria_canonico(id: &str) -> String {
    if !id.starts_with("cat-fabrica-") {
        return id.to_string();
    }
    categorias_de_fabrica()
        .into_iter()
        .find(|c| id_categoria_legado(c) == id)
        .map(|c| c.id)
        .unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn traduce_los_once_ids_historicos() {
        for categoria in categorias_de_fabrica() {
            let legado = id_categoria_legado(&categoria);
            assert_eq!(
                id_categoria_canonico(&legado),
                categoria.id,
                "el ID histórico {legado} debe traducirse a {}",
                categoria.id
            );
        }
    }

    #[test]
    fn conserva_los_ids_de_dominio_y_los_de_usuario() {
        assert_eq!(
            id_categoria_canonico("gasto:alimentacion"),
            "gasto:alimentacion"
        );
        assert_eq!(id_categoria_canonico("uuid-cualquiera"), "uuid-cualquiera");
        assert_eq!(id_categoria_canonico("cat-1"), "cat-1");
    }

    #[test]
    fn reproduce_los_ids_historicos_conocidos() {
        assert_eq!(
            id_categoria_canonico("cat-fabrica-egreso-alimentacion"),
            "gasto:alimentacion"
        );
        assert_eq!(
            id_categoria_canonico("cat-fabrica-egreso-otros-gastos"),
            "gasto:otros"
        );
        assert_eq!(
            id_categoria_canonico("cat-fabrica-ingreso-otros-ingresos"),
            "ingreso:otros"
        );
    }
}
