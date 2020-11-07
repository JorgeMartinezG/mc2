use geojson::{GeoJson, Value};
use serde::Deserialize;
use serde_json;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

#[derive(Debug)]
struct Overpass {
    nodes: Vec<Tag>,
    ways: Vec<Tag>,
    polygon_str: String,
}

impl Overpass {
    fn create_filter(element: &str, tags: &Vec<Tag>, poly_str: &String) -> Vec<String> {
        tags.iter()
            .map(|ptag| {
                if ptag.values.len() == 0 {
                    return format!("{}(poly: {})[{}]", element, poly_str, ptag.key);
                }
                let values = ptag.values.join(" | ");
                return format!("{}(poly: {})[{}~{}]", element, poly_str, ptag.key, values);
            })
            .collect::<Vec<String>>()
    }

    fn to_string(&self) {
        let nodes = Overpass::create_filter("nodes", &self.nodes, &self.polygon_str);
        let ways = Overpass::create_filter("ways", &self.ways, &self.polygon_str);

        let query = format!(
            r#"
            ({});
            ({}); >;
            out meta;
        "#,
            nodes.join("\n"),
            ways.join("\n")
        );

        println!("{}", query);
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

    fn new(campaign: &Campaign) -> Overpass {
        let polygon_str = Overpass::geom(&campaign.geom);
        let ref primary_tags = campaign.tags.primary;

        let mut nodes = Vec::new();
        let mut ways = Vec::new();

        campaign
            .geometry_types
            .iter()
            .map(|t| match t.as_str() {
                "point" => nodes.push(primary_tags.to_vec()),
                "polygon" | "line" => ways.push(primary_tags.to_vec()),
                _ => panic!("Geometry type missing"),
            })
            .for_each(drop);

        Overpass {
            nodes: nodes.into_iter().flatten().collect::<Vec<Tag>>(),
            ways: ways.into_iter().flatten().collect::<Vec<Tag>>(),
            polygon_str: polygon_str,
        }
    }
}

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

#[derive(Deserialize, Debug, Clone)]
struct Tag {
    key: String,
    values: Vec<String>,
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
        let overpass = Overpass::new(&data);
        overpass.to_string();
    }
}
