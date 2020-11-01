use geojson::{FeatureCollection, GeoJson, Value};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::fs;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

pub struct Overpass<'a> {
    pub url: &'a str,
}

impl Default for Overpass<'_> {
    fn default() -> Self {
        Overpass { url: OVERPASS_URL }
    }
}

impl Overpass<'_> {
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

        let ref value = geom.features[0]
            .geometry
            .as_ref()
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

    fn get_data(&self) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("HotOSM"));

        let client = Client::new();
        let body = r#"(
			way(poly:"10.99019103370231 -74.80801105499268 10.9946144688616 -74.80352640151978 10.996931479848776 -74.80504989624023 10.9973948798614 -74.81062889099121 10.993982553614636 -74.81320381164551")[building];
			);
			out meta;"#;
        let mut resp = client
            .post(self.url)
            .headers(headers)
            .form(&[("data", body)])
            .send()
            .expect("Error executing overpass request");

        if resp.status().is_success() {
            let mut buffer = fs::File::create("foo.txt").expect("Could not open file");
            resp.copy_to(&mut buffer).expect("Could not copy to file");
        }
        println!("{:?}", resp);
    }
}

#[cfg(test)]
pub mod overpass_tests {
    use super::*;

    #[test]
    fn test_overpass_request() {
        Overpass::default().get_data();
    }

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
