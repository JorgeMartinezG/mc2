use geojson::{GeoJson, Value};
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;

use std::fs::{create_dir, File};
use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

#[derive(Debug)]
struct Overpass {
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

    fn new(campaign: Campaign) -> Overpass {
        let polygon_str = Overpass::geom(&campaign.geom);
        let mut nodes = Vec::new();
        let mut ways = Vec::new();
        let mut polygons = Vec::new();

        let ref tags = campaign.tags;
        campaign
            .geometry_types
            .iter()
            .map(|t| match t.as_str() {
                "points" => nodes.push(tags.to_vec()),
                "lines" => ways.push(tags.to_vec()),
                "polygons" => polygons.push(tags.to_vec()),
                _ => panic!("Geometry type missing"),
            })
            .for_each(drop);

        Overpass {
            nodes: nodes.into_iter().flatten().collect::<Vec<SearchTag>>(),
            ways: ways.into_iter().flatten().collect::<Vec<SearchTag>>(),
            polygons: polygons.into_iter().flatten().collect::<Vec<SearchTag>>(),
            polygon_str: polygon_str,
            url: OVERPASS_URL.to_string(),
        }
    }

    fn fetch_data(&self, storage_path: &PathBuf) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("HotOSM"));
        let query = self.build_query();
        let mut resp = Client::new()
            .post(&self.url)
            .headers(headers)
            .form(&[("data", &query)])
            .send()
            .expect("Error executing overpass request");

        let file_path = format!("{}/overpass.xml", storage_path.display());

        if resp.status().is_success() {
            let mut buffer = File::create(file_path).expect("Could not open file");
            resp.copy_to(&mut buffer).expect("Could not copy to file");
        }
    }
}

#[derive(Deserialize, Debug)]
struct Campaign {
    name: String,
    geometry_types: Vec<String>,
    tags: Vec<SearchTag>,
    geom: GeoJson,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
struct SearchTag {
    key: String,
    values: Vec<String>,
    secondary: Option<Vec<SearchTag>>,
}

fn main() {
    println!("Hello world");
}

struct LocalStorage {
    path: PathBuf,
}

impl LocalStorage {
    fn new(uuid: &str) -> Self {
        LocalStorage {
            path: Path::new(".").join(uuid),
        }
    }
}

struct CampaignRun {
    uuid: String,
    source: Overpass,
    storage: LocalStorage,
}

impl CampaignRun {
    fn create_path(&self) {
        let path = Path::new(".").join(&self.uuid);
        if path.exists() == false {
            create_dir(path).unwrap();
        }
    }

    fn new(campaign: Campaign) -> Self {
        let uuid = Uuid::new_v4();
        let mut buffer = Uuid::encode_buffer();
        let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();

        let storage = LocalStorage::new(&uuid);

        CampaignRun {
            source: Overpass::new(campaign),
            storage: storage,
            uuid: uuid,
        }
    }

    fn run(&self) {
        if self.storage.path.exists() == false {
            create_dir(&self.storage.path).unwrap();
        }
        self.source.fetch_data(&self.storage.path);
    }
}

#[cfg(test)]
mod campaign_test {
    use super::*;
    #[test]
    fn test_load_campaign() {
        let campaign_str = r#"
            {
                "name": "Test Campaign",
                "geometry_types": ["points", "polygons"],
                "tags": [
                    {
                        "key": "buildings",
                        "values": [],
                        "secondary": [{
                            "key": "amenity",
                            "values": ["hospital", "pharmacy"]
                        }]
                    },
                    {
                        "key": "highway",
                        "values": ["roads", "train_stations"],
                        "secondary": null
                    },
                    {
                        "key": "amenity",
                        "values": ["pub"]
                    }
                ],
                "geom": {
                    "type": "FeatureCollection",
                    "features": [{
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [
                                    [
                                        -74.80801105499268,
                                        10.99019103370231
                                    ],
                                    [
                                        -74.80352640151978,
                                        10.994614468861599
                                    ],
                                    [
                                        -74.80504989624023,
                                        10.996931479848776
                                    ],
                                    [
                                        -74.81062889099121,
                                        10.9973948798614
                                    ],
                                    [
                                        -74.81320381164551,
                                        10.993982553614636
                                    ],
                                    [
                                        -74.80801105499268,
                                        10.99019103370231
                                    ]
                                ]
                            ]
                        }
                    }]
                }
            }
        "#;

        let data: Campaign = serde_json::from_str(campaign_str).expect("failed reading file");
        let run = CampaignRun::new(data);
        run.run();
        //run = Cam
    }
}
