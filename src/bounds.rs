/// Limites geográficos do estado de Alagoas (IBGE / OSM).
///
/// Usado para:
///   - `viewbox` no Nominatim  → prioriza resultados dentro do estado
///   - Validação de coordenadas → rejeita pontos visivelmente fora de AL
///
/// Formato Nominatim viewbox: min_lon,min_lat,max_lon,max_lat
///                            (oeste, sul,    leste,  norte)

pub struct BBox {
    pub west:  f64,
    pub south: f64,
    pub east:  f64,
    pub north: f64,
}

impl BBox {
    /// Retorna `true` se (lat, lng) está dentro da bbox.
    pub fn contains(&self, lat: f64, lng: f64) -> bool {
        lat  >= self.south && lat  <= self.north
            && lng >= self.west  && lng  <= self.east
    }

    /// Formato `west,south,east,north` aceito pelo Nominatim.
    pub fn as_viewbox(&self) -> String {
        format!("{},{},{},{}", self.west, self.south, self.east, self.north)
    }
}

/// Bbox precisa do estado de Alagoas.
pub const ALAGOAS: BBox = BBox {
    west:  -38.24,
    south: -10.50,
    east:  -35.09,
    north:  -8.80,
};

/// Bbox estendida — cobre cidades na fronteira com PE / SE / BA.
/// Usada para validação de entrega: coordenadas nessa faixa ainda são aceitas,
/// mas coordenadas fora dela são rejeitadas como "fora da área".
pub const ALAGOAS_EXTENDED: BBox = BBox {
    west:  -39.00,
    south: -11.00,
    east:  -34.50,
    north:  -7.80,
};

/// Verifica se as coordenadas estão dentro da região operacional.
/// Retorna `(within_al, within_extended)`.
pub fn classify(lat: f64, lng: f64) -> (bool, bool) {
    (
        ALAGOAS.contains(lat, lng),
        ALAGOAS_EXTENDED.contains(lat, lng),
    )
}