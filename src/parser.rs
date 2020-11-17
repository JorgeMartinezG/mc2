use std::collections::HashMap;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use xml::reader::{EventReader, XmlEvent};

use std::io::Seek;
use std::io::SeekFrom;

use crate::campaign::SearchTag;

use crate::elements::{find_attribute, ElementType, LatLng, NElement, Tag};

pub fn parse(read_path: &str, write_path: &str, search_tags: &HashMap<String, SearchTag>) {
    let file = BufReader::new(File::open(read_path).expect("Could not open xml file"));

    let writer_file = File::create(write_path).unwrap();
    let mut writer = BufWriter::new(writer_file);

    let mut ref_nodes: HashMap<i64, LatLng> = HashMap::new();

    let mut parser = EventReader::new(file);

    let mut feature_count: HashMap<String, i64> = HashMap::new();

    let mut element = NElement::init();

    writer
        .write(r#"{"type": "FeatureCollection","features": ["#.as_bytes())
        .unwrap();

    loop {
        let evt = parser.next().expect("Parsing error!");
        match evt {
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
                "node" => element.set_properties("node", &attributes),
                // If there are tags...include them in the current element.
                "tag" => {
                    let tag = Tag::new(&attributes);
                    element.add_tag(tag);
                }
                "way" => element.set_properties("way", &attributes),
                "nd" => {
                    let id = find_attribute("ref", &attributes)
                        .parse::<i64>()
                        .expect("Error parsing");

                    let node = ref_nodes.get(&id).unwrap().clone();
                    element.add_coords(node);
                }
                _ => println!("{:?}", name),
            },
            XmlEvent::EndElement { name } => {
                match name.local_name.as_str() {
                    "node" | "way" => match element.element_type {
                        Some(ElementType::Node) => {
                            if element.tags.len() == 0 {
                                ref_nodes.insert(
                                    element.props.clone().unwrap().id,
                                    element.coords[0].clone(),
                                );
                            } else {
                                let feature = element
                                    .to_feature(search_tags, &mut feature_count)
                                    .to_string()
                                    + &",".to_string();
                                writer.write(feature.as_bytes()).unwrap();
                            }
                        }
                        Some(ElementType::Way) => {
                            let feature = element
                                .to_feature(search_tags, &mut feature_count)
                                .to_string()
                                + &",".to_string();
                            writer.write(feature.as_bytes()).unwrap();
                        }
                        _ => continue,
                    },

                    _ => continue,
                }
                element = NElement::init();
            }
            XmlEvent::EndDocument => {
                writer.seek(SeekFrom::End(0)).unwrap();
                writer.seek(SeekFrom::Current(-1)).unwrap();
                writer.write(b"]").unwrap();

                let feature_count_str = feature_count
                    .into_iter()
                    .map(|(k, v)| format!(" \"{}\":{}", k, v))
                    .collect::<Vec<String>>()
                    .join(",");

                let features_str = format!(
                    r#","properties": {{ "feature_counts": {{ {}  }} }} }}"#,
                    feature_count_str
                );

                writer.write(features_str.as_bytes()).unwrap();

                break;
            }
            _ => continue,
        }
    }
}
