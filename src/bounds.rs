/// Limites geográficos do estado de Alagoas (IBGE / OSM).
///
/// Usado para:
///   - `viewbox` no Nominatim  → prioriza resultados dentro do estado
///   - Validação de coordenadas → rejeita pontos visivelmente fora de AL
///
/// Formato Nominatim viewbox: min_lon,min_lat,max_lon,max_lat
///                            (oeste, sul,    leste,  norte)

pub struct BBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl BBox {
    /// Retorna `true` se `(lat, lng)` está dentro da bbox.
    pub fn contains(&self, lat: f64, lng: f64) -> bool {
        lat >= self.south && lat <= self.north && lng >= self.west && lng <= self.east
    }

    /// Formato `west,south,east,north` aceito pelo Nominatim.
    pub fn as_viewbox(&self) -> String {
        format!("{},{},{},{}", self.west, self.south, self.east, self.north)
    }
}

/// Bbox precisa do estado de Alagoas.
pub const ALAGOAS: BBox = BBox {
    west: -38.24,
    south: -10.50,
    east: -35.09,
    north: -8.80,
};

/// Bbox estendida — cobre municípios na fronteira com PE / SE / BA.
/// Coordenadas nessa faixa são aceitas para entrega; fora dela são rejeitadas.
pub const ALAGOAS_EXTENDED: BBox = BBox {
    west: -39.00,
    south: -11.00,
    east: -34.50,
    north: -7.80,
};

/// Classifica as coordenadas dentro da região operacional.
///
/// Retorna `(within_al, within_extended)`.
pub fn classify(lat: f64, lng: f64) -> (bool, bool) {
    (
        ALAGOAS.contains(lat, lng),
        ALAGOAS_EXTENDED.contains(lat, lng),
    )
}

// ── Testes ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Coordenadas de referência (fonte: Google Maps / IBGE)
    const MACEIO: (f64, f64) = (-9.6658, -35.7350);
    const ARAPIRACA: (f64, f64) = (-9.7522, -36.6613);
    const PENEDO: (f64, f64) = (-10.2900, -36.5900); // extremo sul de AL
    const DELMIRO: (f64, f64) = (-9.3900, -37.9900); // extremo oeste de AL
    const SAO_PAULO: (f64, f64) = (-23.5505, -46.6333);
    const BRASILIA: (f64, f64) = (-15.7801, -47.9292);
    const FORTALEZA: (f64, f64) = (-3.7172, -38.5433); // fora (norte demais)

    // ── Dentro de Alagoas ────────────────────────────────────────────────

    #[test]
    fn maceio_within_alagoas() {
        assert!(ALAGOAS.contains(MACEIO.0, MACEIO.1));
    }

    #[test]
    fn arapiraca_within_alagoas() {
        assert!(ALAGOAS.contains(ARAPIRACA.0, ARAPIRACA.1));
    }

    #[test]
    fn penedo_within_alagoas() {
        assert!(ALAGOAS.contains(PENEDO.0, PENEDO.1));
    }

    #[test]
    fn delmiro_within_alagoas() {
        assert!(ALAGOAS.contains(DELMIRO.0, DELMIRO.1));
    }

    // ── Fora de Alagoas ──────────────────────────────────────────────────

    #[test]
    fn sao_paulo_outside_alagoas() {
        assert!(!ALAGOAS.contains(SAO_PAULO.0, SAO_PAULO.1));
    }

    #[test]
    fn fortaleza_outside_alagoas() {
        // Fortaleza está ao norte do limite norte de AL (-8.80)
        assert!(!ALAGOAS.contains(FORTALEZA.0, FORTALEZA.1));
    }

    // ── Extended ─────────────────────────────────────────────────────────

    #[test]
    fn sao_paulo_outside_extended() {
        assert!(!ALAGOAS_EXTENDED.contains(SAO_PAULO.0, SAO_PAULO.1));
    }

    #[test]
    fn brasilia_outside_extended() {
        assert!(!ALAGOAS_EXTENDED.contains(BRASILIA.0, BRASILIA.1));
    }

    #[test]
    fn fortaleza_outside_extended() {
        assert!(!ALAGOAS_EXTENDED.contains(FORTALEZA.0, FORTALEZA.1));
    }

    // ── classify ─────────────────────────────────────────────────────────

    #[test]
    fn classify_maceio_both_true() {
        let (in_al, in_ext) = classify(MACEIO.0, MACEIO.1);
        assert!(in_al, "Maceió deve estar dentro de AL");
        assert!(in_ext, "Maceió deve estar dentro da extended");
    }

    #[test]
    fn classify_sao_paulo_both_false() {
        let (in_al, in_ext) = classify(SAO_PAULO.0, SAO_PAULO.1);
        assert!(!in_al);
        assert!(!in_ext);
    }

    // ── viewbox ──────────────────────────────────────────────────────────

    #[test]
    fn viewbox_format_nominatim_compatible() {
        // Nominatim espera: west,south,east,north
        let vb = ALAGOAS.as_viewbox();
        assert_eq!(vb, "-38.24,-10.5,-35.09,-8.8");
    }

    // ── Invariantes ──────────────────────────────────────────────────────

    #[test]
    fn alagoas_is_subset_of_extended() {
        // Todo ponto dentro de AL deve estar na extended
        let points = [MACEIO, ARAPIRACA, PENEDO, DELMIRO];
        for (lat, lng) in points {
            let (in_al, in_ext) = classify(lat, lng);
            assert!(
                !in_al || in_ext,
                "({lat}, {lng}) está em AL mas não na extended — invariante violada"
            );
        }
    }
}