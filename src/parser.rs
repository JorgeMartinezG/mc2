use std::collections::HashMap;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

use geojson::{Feature, Geometry, Value};
use serde_json::{to_value, Map};

#[derive(Debug)]
enum Element {
    Initialized,
    Node(Node),
    Way(Way),
}

impl Element {
    fn add_tag(&mut self, tag: Tag) {
        match self {
            Element::Node(ref mut n) => n.tags.push(tag),
            Element::Way(ref mut w) => w.tags.push(tag),
            _ => (),
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    id: i64,
    lat: f64,
    lon: f64,
    tags: Vec<Tag>,
}

impl Node {
    fn to_vec(&self) -> Vec<f64> {
        vec![self.lon, self.lat]
    }
}

#[derive(Debug, Clone)]
struct Way {
    id: i64,
    nodes: Vec<Node>,
    tags: Vec<Tag>,
}

#[derive(Debug, Clone)]
struct Tag {
    key: String,
    value: String,
}

fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .unwrap()
        .value
        .clone()
}

fn create_node(attributes: &Vec<OwnedAttribute>) -> Node {
    let lat = find_attribute("lat", &attributes)
        .parse::<f64>()
        .expect("Error parsing");
    let lon = find_attribute("lon", &attributes)
        .parse::<f64>()
        .expect("Error parsing");
    let id = find_attribute("id", &attributes)
        .parse::<i64>()
        .expect("Error parsing");

    Node {
        id: id,
        lat: lat,
        lon: lon,
        tags: Vec::new(),
    }
}

fn create_way(attributes: &Vec<OwnedAttribute>) -> Way {
    let id = find_attribute("id", &attributes)
        .parse::<i64>()
        .expect("Error parsing");

    Way {
        id: id,
        nodes: Vec::new(),
        tags: Vec::new(),
    }
}

fn create_tag(attributes: &Vec<OwnedAttribute>) -> Tag {
    let key = find_attribute("k", &attributes);
    let value = find_attribute("v", &attributes);

    Tag {
        key: key,
        value: value,
    }
}

pub fn parse(read_path: &str, write_path: &str) {
    let file = BufReader::new(File::open(read_path).expect("Could not open xml file"));

    let writer_file = File::create(write_path).unwrap();
    let mut writer = BufWriter::new(writer_file);

    let mut ref_nodes: HashMap<i64, Node> = HashMap::new();
    let mut elements: HashMap<i64, Element> = HashMap::new();

    let mut current_element = Element::Initialized;
    let mut parser = EventReader::new(file);
    loop {
        let evt = parser.next().expect("Parsing error!");
        match evt {
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
                "node" => {
                    let node = create_node(&attributes);
                    current_element = Element::Node(node);
                }
                // If there are tags...include them in the current element.
                "tag" => {
                    let tag = create_tag(&attributes);
                    current_element.add_tag(tag);
                }
                "way" => {
                    let way = create_way(&attributes);
                    current_element = Element::Way(way);
                }
                "nd" => {
                    let id = find_attribute("ref", &attributes)
                        .parse::<i64>()
                        .expect("Error parsing");
                    let node = ref_nodes.get(&id).unwrap().clone();
                    match current_element {
                        Element::Way(ref mut w) => w.nodes.push(node),
                        _ => continue,
                    }
                }
                _ => println!("{:?}", name),
            },
            XmlEvent::EndElement { name } => {
                match name.local_name.as_str() {
                    "node" | "way" => match current_element {
                        Element::Node(ref n) => {
                            if n.tags.len() == 0 {
                                ref_nodes.insert(n.id, n.clone());
                            } else {
                                elements.insert(n.id, Element::Node(n.clone()));
                            }
                        }
                        Element::Way(ref w) => {
                            elements.insert(w.id, Element::Way(w.clone()));
                        }
                        _ => continue,
                    },

                    _ => continue,
                }
                current_element = Element::Initialized;
            }
            XmlEvent::EndDocument => break,
            _ => continue,
        }
    }

    writer
        .write(r#"{"type": "FeatureCollection","features": ["#.as_bytes())
        .unwrap();
    let mut iterator = elements.into_iter().peekable();
    while let Some((_id, element)) = iterator.next() {
        let geometry = match &element {
            Element::Node(n) => Geometry::new(Value::Point(n.to_vec())),
            Element::Way(w) => {
                let points = w
                    .nodes
                    .iter()
                    .map(|n| n.to_vec())
                    .collect::<Vec<Vec<f64>>>();

                if points[0].first() == points[0].last() {
                    Geometry::new(Value::Polygon(vec![points]))
                } else {
                    Geometry::new(Value::LineString(points))
                }
            }
            _ => panic!("Element not recognized"),
        };

        let mut properties = Map::new();
        match &element {
            Element::Node(n) => n
                .tags
                .iter()
                .map(|t| properties.insert(t.key.clone(), to_value(t.value.clone()).unwrap()))
                .for_each(drop),
            Element::Way(w) => w
                .tags
                .iter()
                .map(|t| properties.insert(t.key.clone(), to_value(t.value.clone()).unwrap()))
                .for_each(drop),
            _ => panic!("Element not recognized"),
        }

        let feature = Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        writer.write(feature.to_string().as_bytes()).unwrap();

        if iterator.peek().is_some() {
            writer.write(b",").unwrap();
        }
    }

    writer.write(b"]}").unwrap();
}
