use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
enum Element {
    Node(Node),
    EndOfDocument,
    Ignored,
}

#[derive(Debug)]
struct Node {
    id: i32,
    lat: f64,
    lon: f64,
    tags: Vec<Tag>,
}

#[derive(Debug)]
struct Tag {
    key: String,
    val: String,
}

struct Xml {
    nodes: HashMap<i32, Node>,
}

fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    let attr = attributes.iter().find(|a| a.name.local_name == name);
    match attr {
        Some(a) => a.value.clone(),
        None => "".to_string(),
    }
}

fn create_node(attributes: &Vec<OwnedAttribute>) -> Node {
    let lat = find_attribute("lat", &attributes);
    let lng = find_attribute("lon", &attributes);
    let id = find_attribute("id", &attributes);

    println!("{:?}, {}, {}", lat, lng, id);

    Node {
        id: 1,
        lat: 1.0,
        lon: 2.0,
        tags: Vec::new(),
    }
}

fn parse_element(name: &str, attributes: Vec<OwnedAttribute>) -> Element {
    let elm = match name {
        "nd" => Element::Node(create_node(&attributes)),
        _ => Element::Ignored,
    };

    elm
}

fn match_event(event: XmlEvent) {
    let evt = match event {
        XmlEvent::EndDocument => Element::EndOfDocument,
        XmlEvent::StartElement {
            name, attributes, ..
        } => parse_element(&name.local_name, attributes),
        _ => Element::Ignored,
    };
}

pub fn parse(path: &str) {
    let file = File::open(path).expect("Could not open xml file");
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

    for e in parser {
        let event = e.unwrap();
        match_event(event)
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parse_xml() {
        parse("./examples/overpass.xml");
    }
}
