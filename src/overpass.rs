use geojson::{FeatureCollection, GeoJson, Value};
use std::fs;

pub struct Overpass {
    pub overpass_url: String,
}

impl Overpass {
    pub fn build_query() -> String {
        "".into()
    }

    fn get_geometry(path: &str) -> Result<FeatureCollection, Box<dyn std::error::Error>> {
        let geom = fs::read_to_string(path)?.parse::<GeoJson>()?;

        let feature_collection = match geom {
            GeoJson::FeatureCollection(f) => f,
            _ => panic!("Geojson must be FeatureCollection"),
        };

        Ok(feature_collection)
    }

    fn get_polygon(path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let geom = Overpass::get_geometry(path)?;

        let value = geom.features[0]
            .clone()
            .geometry
            .expect("Geometry not found")
            .value;

        let polygons_array = match value {
            Value::Polygon(p) => p,
            _ => panic!("Polygon type supported only"),
        };

        let items = &polygons_array[0];

        let size_vec = items.len();
        let items = items
            .iter()
            .take(size_vec - 1)
            .map(|b| format!("{} {}", b[1], b[0]))
            .collect::<Vec<String>>()
            .join(" ");

        Ok(items)
    }
}

#[cfg(test)]
pub mod overpass_tests {
    use super::*;

    #[test]
    fn test_polygon() {
        let items = Overpass::get_polygon("./examples/geometry.json").unwrap();
        assert_eq!("10.99019103370231 -74.80801105499268 10.9946144688616 -74.80352640151978 10.996931479848776 -74.80504989624023 10.9973948798614 -74.81062889099121 10.993982553614636 -74.81320381164551", items);
    }

    #[test]
    fn test_build_query() {
        assert_eq!(Overpass::build_query(), "");
    }
}
