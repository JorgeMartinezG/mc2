use std::collections::HashMap;

use std::fs::File;
use std::io::BufReader;

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

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

pub fn parse(path: &str) {
    let file = File::open(path).expect("Could not open xml file");
    let file = BufReader::new(file);

    let mut parser = EventReader::new(file);
    let mut ref_nodes: HashMap<i64, Node> = HashMap::new();
    let mut nodes: HashMap<i64, Node> = HashMap::new();
    let mut ways: HashMap<i64, Way> = HashMap::new();

    let mut current_element = Element::Initialized;
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
                    "node" => match current_element {
                        Element::Node(ref n) => {
                            if n.tags.len() == 0 {
                                ref_nodes.insert(n.id, n.clone());
                            } else {
                                nodes.insert(n.id, n.clone());
                            }
                        }
                        _ => continue,
                    },
                    "way" => match current_element {
                        Element::Way(ref w) => {
                            ways.insert(w.id, w.clone());
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
    println!("{:?}", nodes);
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parse_xml() {
        parse("./examples/overpass.xml");
    }
}
