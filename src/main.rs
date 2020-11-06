use geojson::{GeoJson, Value};
use serde::Deserialize;
use serde_json;

#[derive(Deserialize, Debug)]
struct Campaign {
    name: String,
    geometry_types: Vec<String>,
    tags: Tags,
    geom: GeoJson,
}

#[derive(Deserialize, Debug)]
struct Tags {
    primary: Vec<Tag>,
    secondary: Vec<Tag>,
}

#[derive(Deserialize, Debug)]
struct Tag {
    key: String,
    values: Vec<String>,
}

impl Campaign {
    fn build_overpass_polygon(&self) -> String {
        let feature_collection = match &self.geom {
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

    fn create_filter(&self, element: &str, poly_str: &String) -> Vec<String> {
        self.tags
            .primary
            .iter()
            .map(|ptag| {
                if ptag.values.len() == 0 {
                    return format!("{}(poly: {})[{}]", element, poly_str, ptag.key);
                }
                let values = ptag.values.join(" | ");
                return format!("{}(poly: {})[{}~{}]", element, poly_str, ptag.key, values);
            })
            .collect::<Vec<String>>()
    }

    fn build_overpass_query(&self) {
        let poly_str = self.build_overpass_polygon();

        let query_lines = self
            .geometry_types
            .iter()
            .map(|t| match t.as_str() {
                "point" => self.create_filter("node", &poly_str),
                "polygon" | "line" => self.create_filter("way", &poly_str),
                _ => panic!("Geometry type missing"),
            })
            .flatten()
            .collect::<Vec<String>>();

        println!("{:?}", query_lines);
    }
}

fn main() {
    println!("Hello world");
}

#[cfg(test)]
mod campaign_test {
    use super::*;
    #[test]
    fn test_load_campaign() {
        let campaign_str = r#"
            {
                "name": "Test Campaign",
                "geometry_types": ["point", "polygon"],
                "tags": {
                    "primary": [{
                        "key": "buildings",
                        "values": []
                    }, {
                        "key": "highways",
                        "values": ["roads", "lala"]
                    }],
                    "secondary": [{
                        "key": "amenity",
                        "values": ["hospital", "pharmacy"]
                    }]
                },
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
        data.build_overpass_query();
    }
}
