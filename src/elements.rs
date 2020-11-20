use crate::campaign::SearchTag;
use geojson::{Feature, Geometry, Value};
use serde_json::{to_value, Map};
use std::collections::HashMap;
use xml::attribute::OwnedAttribute;

use serde::Serialize;

pub fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    let attr = match attributes.iter().find(|a| a.name.local_name == name) {
        Some(v) => v.value.clone(),
        None => "".to_string(),
    };
    attr
}

fn check_value(tag_value: &String, values: &Vec<String>) -> Option<String> {
    let mut error = None;
    if values.len() != 0 && values.contains(tag_value) == false {
        error = Some(format!("Value mismatch - expected values: {:?}", values));
    }
    error
}

#[derive(Serialize, Debug)]
struct TagErrors {
    errors: Vec<String>,
    completeness: f64,
}

impl TagErrors {
    fn new(search_tag: &SearchTag, search_errors: Vec<String>) -> Self {
        let len_tags = match search_tag.secondary {
            Some(ref t) => t.len() + 1,
            None => 1,
        };

        let completeness = 1.0 - (search_errors.len() as f64 / len_tags as f64);
        TagErrors {
            errors: search_errors,
            completeness: completeness,
        }
    }
}

fn validate_tags(
    tags: &Vec<Tag>,
    search_key: &String,
    search_tag: &SearchTag,
    feature_count: &mut HashMap<String, i64>,
) -> Option<(String, TagErrors)> {
    let mut search_errors = Vec::new();

    match tags.iter().find(|t| t.key.as_str() == search_key) {
        Some(tag) => {
            match check_value(&tag.value, &search_tag.values) {
                Some(err) => search_errors.push(err),
                None => (),
            };

            if let Some(v) = feature_count.get_mut(&tag.key) {
                *v = *v + 1;
            } else {
                feature_count.insert(tag.key.clone(), 1);
            }

            match search_tag.secondary {
                None => (),
                Some(ref r) => r.iter().for_each(|(sk, st)| {
                    match tags.iter().find(|t| t.key.as_str() == sk) {
                        Some(tag) => match check_value(&tag.value, &st.values) {
                            Some(err) => search_errors.push(err),
                            None => (),
                        },
                        None => search_errors.push(format!("Key {} not found", sk)),
                    };
                }),
            }

            let tag_errors = TagErrors::new(search_tag, search_errors);
            Some((search_key.to_string(), tag_errors))
        }
        None => None,
    }
}

fn compute_errors(
    element_tags: &Vec<Tag>,
    search_tags: &HashMap<String, SearchTag>,
    feature_count: &mut HashMap<String, i64>,
) -> HashMap<String, TagErrors> {
    let errors = search_tags
        .iter()
        .map(|(search_key, search_tag)| {
            validate_tags(&element_tags, &search_key, &search_tag, feature_count)
        })
        .filter_map(|x| x)
        .collect::<HashMap<String, TagErrors>>();
    // Check Value

    errors
}

#[derive(Debug, PartialEq)]
pub enum ElementType {
    Way,
    Node,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementProps {
    pub id: i64,
    pub user: String,
}

pub type LatLng = Vec<f64>;

#[derive(Debug)]
pub struct Element {
    pub element_type: Option<ElementType>,
    pub tags: Vec<Tag>,
    pub coords: Vec<LatLng>,
    pub props: Option<ElementProps>,
}

impl Element {
    pub fn init() -> Self {
        Element {
            element_type: None,
            tags: Vec::new(),
            coords: Vec::new(),
            props: None,
        }
    }

    pub fn add_contributor(&self, contributors: &mut HashMap<String, i64>) {
        let user = self.get_user();
        if let Some(v) = contributors.get_mut(&user) {
            *v = *v + 1;
        } else {
            contributors.insert(user.clone(), 1);
        }
    }

    pub fn get_user(&self) -> String {
        match &self.props {
            Some(p) => p.user.clone(),
            None => panic!("User not found"),
        }
    }

    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    pub fn add_coords(&mut self, coords: LatLng) {
        self.coords.push(coords);
    }

    pub fn set_properties(&mut self, element: &str, attributes: &Vec<OwnedAttribute>) {
        let element_type = match element {
            "node" => ElementType::Node,
            "way" => ElementType::Way,
            _ => panic!("Unrecognized element type"),
        };

        if element_type == ElementType::Node {
            let lat = find_attribute("lat", &attributes)
                .parse::<f64>()
                .expect("Error parsing");
            let lon = find_attribute("lon", &attributes)
                .parse::<f64>()
                .expect("Error parsing");
            let coords = vec![lon, lat];

            self.add_coords(coords);
        }

        self.element_type = Some(element_type);

        let id = find_attribute("id", &attributes)
            .parse::<i64>()
            .expect("Error parsing");

        let user = find_attribute("user", &attributes);

        let props = ElementProps { id: id, user: user };
        self.props = Some(props);
    }

    pub fn to_feature(
        &self,
        search_tags: &HashMap<String, SearchTag>,
        feature_count: &mut HashMap<String, i64>,
        geometry_types: &Vec<String>,
    ) -> Option<Feature> {
        let errors = compute_errors(&self.tags, search_tags, feature_count);
        if errors.len() == 0 {
            return None;
        }

        let geom = match &self.element_type {
            Some(ElementType::Node) => {
                if geometry_types.contains(&"points".to_string()) == false {
                    return None;
                }

                Geometry::new(Value::Point(self.coords[0].clone()))
            }
            Some(ElementType::Way) => {
                if self.coords.len() == 0 {
                    return None;
                }

                let geom = match &self.coords.first() == &self.coords.last() {
                    false => {
                        if geometry_types.contains(&"linestrings".to_string()) == false {
                            return None;
                        }
                        Geometry::new(Value::LineString(self.coords.clone()))
                    }
                    true => {
                        if geometry_types.contains(&"polygons".to_string()) == false {
                            return None;
                        }
                        Geometry::new(Value::Polygon(vec![self.coords.clone()]))
                    }
                };

                geom
            }
            _ => panic!("unknown element_type"),
        };

        let mut properties = Map::new();

        properties.insert("stats".to_string(), to_value(&errors).unwrap());
        properties.insert(
            "id".to_string(),
            to_value(self.props.as_ref().unwrap().id.clone()).unwrap(),
        );
        properties.insert(
            "user".to_string(),
            to_value(self.props.as_ref().unwrap().user.clone()).unwrap(),
        );

        let feat = Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        Some(feat)
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    key: String,
    value: String,
}

impl Tag {
    pub fn new(attributes: &Vec<OwnedAttribute>) -> Tag {
        let key = find_attribute("k", &attributes);
        let value = find_attribute("v", &attributes);

        Tag {
            key: key,
            value: value,
        }
    }
}
