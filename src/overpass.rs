use geojson::{GeoJson, Value};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::fs::File;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

use crate::campaign::Campaign;

#[derive(Debug)]
pub struct Overpass {
    nodes: Vec<String>,
    ways: Vec<String>,
    relations: Vec<String>,
    polygon_strs: Vec<String>,
    url: String,
}

impl Overpass {
    fn create_filter(element: &str, tag: &(&str, Vec<String>), poly_str: &String) -> String {
        if tag.1.len() == 0 {
            return format!("{}(poly: '{}')['{}'];", element, poly_str, tag.0);
        }
        let values = tag.1.join(" | ");
        return format!(
            "{}(poly: '{}')['{}'~'{}'];",
            element, poly_str, tag.0, values
        );
    }

    fn build_query(&self) -> String {
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
            self.nodes.join("\n"),
            self.ways.join("\n"),
            self.relations.join("\n"),
        );

        query
    }

    fn geom(geom: &GeoJson) -> Vec<String> {
        let feature_collection = match &geom {
            GeoJson::FeatureCollection(f) => f,
            _ => panic!("Geojson must be FeatureCollection"),
        };

        let externals: Vec<String> = feature_collection
            .features
            .iter()
            .map(|f| {
                let ref value = f.geometry.as_ref().expect("Geometry not found").value;

                let polygons_array = match value {
                    Value::Polygon(p) => p,
                    _ => panic!("Polygon type supported only"),
                };

                // Take external polygon.
                let items = &polygons_array[0];
                items
            })
            .map(|items| {
                let size_vec = items.len();
                items
                    .iter()
                    .take(size_vec - 1)
                    .map(|b| format!("{} {}", b[1], b[0]))
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect();

        externals
    }

    pub fn new(campaign: Campaign) -> Overpass {
        let polygon_strs = Overpass::geom(&campaign.geom);
        let mut nodes = Vec::new();
        let mut ways = Vec::new();
        let mut relations = Vec::new();
        let ref tags = campaign.tags;

        campaign
            .geometry_types
            .iter()
            .for_each(|t| match t.as_str() {
                "points" => tags.iter().for_each(|(k, v)| {
                    polygon_strs.iter().for_each(|pstr| {
                        nodes.push(Overpass::create_filter(
                            "node",
                            &(k, v.values.clone()),
                            &pstr,
                        ))
                    })
                }),
                "lines" => tags.iter().for_each(|(k, v)| {
                    polygon_strs.iter().for_each(|pstr| {
                        ways.push(Overpass::create_filter(
                            "way",
                            &(k, v.values.clone()),
                            &pstr,
                        ))
                    })
                }),
                "polygons" => tags.iter().for_each(|(k, v)| {
                    polygon_strs.iter().for_each(|pstr| {
                        relations.push(Overpass::create_filter(
                            "way",
                            &(k, v.values.clone()),
                            &pstr,
                        ))
                    })
                }),

                _ => panic!("Geometry type not recognized"),
            });

        Overpass {
            nodes: nodes,
            ways: ways,
            relations: relations,
            polygon_strs: polygon_strs,
            url: OVERPASS_URL.to_string(),
        }
    }

    pub fn fetch_data(&self, storage_path: &String) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("HotOSM"));
        let query = self.build_query();
        println!("{}", query);

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
