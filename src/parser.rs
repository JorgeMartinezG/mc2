use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

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
    // let evt = match event {
    //     XmlEvent::EndDocument => Element::EndOfDocument,
    //     XmlEvent::StartElement {
    //         name, attributes, ..
    //     } => Element::Node(Node{}),
    //     _ => Element::Ignored,
    // };
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
