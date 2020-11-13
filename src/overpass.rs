use geojson::{GeoJson, Value};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::fs::File;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

use crate::campaign::{Campaign, SearchTag};

#[derive(Debug)]
pub struct Overpass {
    nodes: Vec<SearchTag>,
    ways: Vec<SearchTag>,
    polygons: Vec<SearchTag>,
    polygon_str: String,
    url: String,
}

impl Overpass {
    fn create_filter(element: &str, tags: &Vec<SearchTag>, poly_str: &String) -> Vec<String> {
        tags.iter()
            .map(|ptag| {
                if ptag.values.len() == 0 {
                    return format!("{}(poly: '{}')['{}'];", element, poly_str, ptag.key);
                }
                let values = ptag.values.join(" | ");
                return format!(
                    "{}(poly: '{}')['{}'~'{}'];",
                    element, poly_str, ptag.key, values
                );
            })
            .collect::<Vec<String>>()
    }

    fn build_query(&self) -> String {
        let nodes = Overpass::create_filter("node", &self.nodes, &self.polygon_str);
        let ways = Overpass::create_filter("way", &self.ways, &self.polygon_str);
        let polygons = Overpass::create_filter("relation", &self.polygons, &self.polygon_str);
        let query = format!(
            r#"(
            (
              {}
            );
            (
              {}
            );>;
            (
              {}
            );>>;>;
            );out meta;
        "#,
            nodes.join("\n"),
            ways.join("\n"),
            polygons.join("\n")
        );

        query
    }

    fn geom(geom: &GeoJson) -> String {
        let feature_collection = match &geom {
            GeoJson::FeatureCollection(f) => f,
            _ => panic!("Geojson must be FeatureCollection"),
        };

        let ref value = feature_collection.features[0]
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

        items
            .iter()
            .take(size_vec - 1)
            .map(|b| format!("{} {}", b[1], b[0]))
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn new(campaign: Campaign) -> Overpass {
        let polygon_str = Overpass::geom(&campaign.geom);
        let mut nodes = Vec::new();
        let mut ways = Vec::new();
        let mut polygons = Vec::new();

        let ref tags = campaign.tags;
        campaign
            .geometry_types
            .iter()
            .for_each(|t| match t.as_str() {
                "points" => nodes.push(tags.to_vec()),
                "lines" => ways.push(tags.to_vec()),
                "polygons" => polygons.push(tags.to_vec()),
                _ => panic!("Geometry type missing"),
            });

        Overpass {
            nodes: nodes.into_iter().flatten().collect::<Vec<SearchTag>>(),
            ways: ways.into_iter().flatten().collect::<Vec<SearchTag>>(),
            polygons: polygons.into_iter().flatten().collect::<Vec<SearchTag>>(),
            polygon_str: polygon_str,
            url: OVERPASS_URL.to_string(),
        }
    }

    pub fn fetch_data(&self, storage_path: &String) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("HotOSM"));
        let query = self.build_query();
        let mut resp = Client::new()
            .post(&self.url)
            .headers(headers)
            .form(&[("data", &query)])
            .send()
            .expect("Error executing overpass request");

        if resp.status().is_success() {
            let mut buffer = File::create(storage_path).expect("Could not open file");
            resp.copy_to(&mut buffer).expect("Could not copy to file");
        }
    }
}
