use std::collections::HashMap;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use xml::reader::{EventReader, XmlEvent};

use std::io::Seek;
use std::io::SeekFrom;

use crate::campaign::SearchTag;

use crate::elements::{find_attribute, Element, ElementType, LatLng, Tag};

fn serialize_hashmap(hashmap: HashMap<String, i64>) -> String {
    hashmap
        .into_iter()
        .map(|(k, v)| format!(" \"{}\":{}", k, v))
        .collect::<Vec<String>>()
        .join(",")
}

pub fn create_key(key: &String, values: &Vec<String>) -> String {
    match values.len() {
        0 => key.to_string(),
        _ => format!("{}={}", key, values.join(",")),
    }
}

fn init_completeness_counter(
    search_tags: &HashMap<String, SearchTag>,
) -> HashMap<String, HashMap<String, i64>> {
    let completeness_count: HashMap<String, HashMap<String, i64>> = search_tags
        .iter()
        .map(|(k, v)| {
            let mut hm = HashMap::new();
            hm.insert("complete".to_string(), 0);
            hm.insert("incomplete".to_string(), 0);

            (create_key(k, &v.values), hm)
        })
        .collect();

    completeness_count
}

fn init_attributes_count(search_tags: &HashMap<String, SearchTag>) -> HashMap<String, i64> {
    search_tags
        .iter()
        .map(|(_k, v)| match v.secondary {
            None => None,
            Some(ref s) => {
                let init_values = s
                    .iter()
                    .map(|(sk, sv)| (create_key(sk, &sv.values), 0))
                    .collect::<Vec<(String, i64)>>();

                Some(init_values)
            }
        })
        .filter_map(|x| x)
        .flatten()
        .collect::<HashMap<String, i64>>()
}

fn init_contributors_count(
    search_tags: &HashMap<String, SearchTag>,
) -> HashMap<String, HashMap<String, i64>> {
    search_tags
        .iter()
        .map(|(k, v)| {
            let hm: HashMap<String, i64> = HashMap::new();
            (create_key(k, &v.values), hm)
        })
        .collect::<HashMap<String, HashMap<String, i64>>>()
}

fn init_feature_count(search_tags: &HashMap<String, SearchTag>) -> HashMap<String, i64> {
    search_tags
        .iter()
        .map(|(k, v)| (create_key(k, &v.values), 0))
        .collect::<HashMap<String, i64>>()
}

pub fn parse(
    read_path: &str,
    write_path: &str,
    search_tags: &HashMap<String, SearchTag>,
    geometry_types: &Vec<String>,
) {
    let file = BufReader::new(File::open(read_path).expect("Could not open xml file"));

    let writer_file = File::create(write_path).unwrap();
    let mut writer = BufWriter::new(writer_file);

    let mut ref_nodes: HashMap<i64, LatLng> = HashMap::new();

    let mut parser = EventReader::new(file);

    let mut feature_count = init_feature_count(search_tags);

    let mut completeness_count = init_completeness_counter(search_tags);

    let mut element = Element::init();

    let mut contributors = init_contributors_count(search_tags);

    let mut attributes_count = init_attributes_count(search_tags);

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

                    ref_nodes
                        .get(&id)
                        .map(|node| element.add_coords(node.clone()));
                }
                _ => (),
            },
            XmlEvent::EndElement { name } => {
                match name.local_name.as_str() {
                    "node" | "way" => match element.element_type {
                        Some(ElementType::Node) => match element.tags.len() {
                            0 => {
                                ref_nodes.insert(
                                    element.props.clone().unwrap().id,
                                    element.coords[0].clone(),
                                );
                            }
                            _ => {
                                element
                                    .to_feature(
                                        &search_tags,
                                        &mut feature_count,
                                        geometry_types,
                                        &mut attributes_count,
                                        &mut completeness_count,
                                        &mut contributors,
                                    )
                                    .map(|f| {
                                        writer
                                            .write((f.to_string() + &",".to_string()).as_bytes())
                                            .expect("could not save element");
                                    });
                            }
                        },
                        Some(ElementType::Way) => {
                            element
                                .to_feature(
                                    &search_tags,
                                    &mut feature_count,
                                    geometry_types,
                                    &mut attributes_count,
                                    &mut completeness_count,
                                    &mut contributors,
                                )
                                .map(|f| {
                                    writer
                                        .write((f.to_string() + &",".to_string()).as_bytes())
                                        .expect("could not save element");
                                });
                        }
                        _ => continue,
                    },

                    _ => continue,
                }
                element = Element::init();
            }
            XmlEvent::EndDocument => {
                writer.seek(SeekFrom::End(0)).unwrap();
                writer.seek(SeekFrom::Current(-1)).unwrap();
                writer.write(b"]").unwrap();

                let feature_count_str = serialize_hashmap(feature_count);
                let attributes_count_str = serialize_hashmap(attributes_count);

                let completeness_count_str = completeness_count
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {{ {} }}", k, serialize_hashmap(v.clone())))
                    .collect::<Vec<String>>()
                    .join(",");

                let contributors_str = contributors
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {{ {} }}", k, serialize_hashmap(v.clone())))
                    .collect::<Vec<String>>()
                    .join(",");

                let features_str = format!(
                    r#","properties": {{ "feature_counts": {{ {} }} , "contributors": {{ {} }}, "attributes_count": {{ {} }} , "completeness_count": {{ {} }} }} }}"#,
                    feature_count_str,
                    contributors_str,
                    attributes_count_str,
                    completeness_count_str
                );

                writer.write(features_str.as_bytes()).unwrap();

                break;
            }
            _ => continue,
        }
    }
}
