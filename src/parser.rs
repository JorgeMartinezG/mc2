use std::collections::HashMap;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use xml::reader::{EventReader, XmlEvent};

use std::io::Seek;
use std::io::SeekFrom;

use crate::campaign::SearchTag;
use crate::elements::{find_attribute, Element, Node, Tag, Way};

pub fn parse(read_path: &str, write_path: &str, search_tags: &HashMap<String, SearchTag>) {
    let file = BufReader::new(File::open(read_path).expect("Could not open xml file"));

    let writer_file = File::create(write_path).unwrap();
    let mut writer = BufWriter::new(writer_file);

    let mut ref_nodes: HashMap<i64, Node> = HashMap::new();

    let mut current_element = Element::Initialized;
    let mut parser = EventReader::new(file);

    writer
        .write(r#"{"type": "FeatureCollection","features": ["#.as_bytes())
        .unwrap();

    loop {
        let evt = parser.next().expect("Parsing error!");
        match evt {
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
                "node" => {
                    let node = Node::new(&attributes);
                    current_element = Element::Node(node);
                }
                // If there are tags...include them in the current element.
                "tag" => {
                    let tag = Tag::new(&attributes);
                    current_element.add_tag(tag);
                }
                "way" => {
                    let way = Way::new(&attributes);
                    current_element = Element::Way(way);
                }
                "nd" => {
                    let id = find_attribute("ref", &attributes)
                        .parse::<i64>()
                        .expect("Error parsing");

                    let node = ref_nodes.get(&id).unwrap().clone();
                    if let Element::Way(ref mut w) = current_element {
                        w.nodes.push(node);
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
                                let feat = n.to_feature(search_tags).to_string();
                                writer.write(feat.as_bytes()).unwrap();
                            }
                        }
                        Element::Way(ref w) => {
                            let feature = w.to_feature(search_tags).to_string();
                            writer.write(feature.as_bytes()).unwrap();
                        }
                        _ => continue,
                    },

                    _ => continue,
                }
                current_element = Element::Initialized;
                writer.write(b",").unwrap();
            }
            XmlEvent::EndDocument => {
                writer.seek(SeekFrom::End(0)).unwrap();
                writer.seek(SeekFrom::Current(-1)).unwrap();
                writer.write("]}".as_bytes()).unwrap();
                break;
            }
            _ => continue,
        }
    }
}
