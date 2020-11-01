use std::collections::HashMap;

use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
enum Element {
    Initialized,
    Node(Node),
    EndOfDocument,
    Ignored,
}

#[derive(Debug, Clone)]
struct Node {
    id: i64,
    lat: f64,
    lon: f64,
    tags: Vec<Tag>,
}

#[derive(Debug, Clone)]
struct Tag {
    key: String,
    value: String,
}

fn find_attribute<T>(name: &str, attributes: &Vec<OwnedAttribute>) -> Result<T, T::Err>
where
    T: FromStr,
{
    let attr = attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .unwrap();
    let val = attr.value.clone().parse::<T>();

    val
}

fn create_node(attributes: &Vec<OwnedAttribute>) {
    let lat = find_attribute::<f64>("lat", &attributes).expect("Error parsing");
    let lng = find_attribute::<f64>("lon", &attributes).expect("Error parsing");
    let id = find_attribute::<i64>("id", &attributes).expect("Error parsing");

    println!("{:?}, {:?}, {:?}", lat, lng, id);
}

fn create_element(name: &str, attributes: Vec<OwnedAttribute>) {
    match name {
        "node" => create_node(&attributes),
        _ => println!("Element ignored"),
    }
}

fn match_event(event: XmlEvent) {
    match event {
        XmlEvent::StartElement {
            name, attributes, ..
        } => create_element(name.local_name.as_str(), attributes),
        _ => println!("ignore"),
    }
}

pub fn parse(path: &str) {
    let file = File::open(path).expect("Could not open xml file");
    let file = BufReader::new(file);

    let mut parser = EventReader::new(file);
    let mut ref_nodes: HashMap<i64, Node> = HashMap::new();
    let mut nodes: HashMap<i64, Node> = HashMap::new();

    let mut current_element = Element::Initialized;
    loop {
        let evt = parser.next().expect("Parsing error!");
        match evt {
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
                "node" => {
                    let lat = find_attribute::<f64>("lat", &attributes).expect("Error parsing");
                    let lon = find_attribute::<f64>("lon", &attributes).expect("Error parsing");
                    let id = find_attribute::<i64>("id", &attributes).expect("Error parsing");

                    let node = Node {
                        id: id,
                        lat: lat,
                        lon: lon,
                        tags: Vec::new(),
                    };
                    current_element = Element::Node(node);
                }
                // If there are tags...include them in the current element.
                "tag" => {
                    let key = attributes
                        .iter()
                        .find(|i| i.name.local_name == "k")
                        .unwrap()
                        .value
                        .clone();

                    let value = attributes
                        .iter()
                        .find(|i| i.name.local_name == "v")
                        .unwrap()
                        .value
                        .clone();

                    let tag = Tag {
                        key: key,
                        value: value,
                    };

                    match current_element {
                        Element::Node(ref mut n) => n.tags.push(tag),
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
