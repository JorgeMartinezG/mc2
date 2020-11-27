use crate::campaign::SearchTag;
use crate::parser::create_key;
use geojson::{Feature, Geometry, Value};
use serde_json::{to_value, Map};
use std::collections::HashMap;
use xml::attribute::OwnedAttribute;

use serde::Serialize;

pub fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    let attr = match attributes.iter().find(|a| a.name.local_name == name) {
        Some(v) => v.value.clone(),
        None => "unknown".to_string(),
    };
    attr
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TagError {
    KeyNotFound(String),
    ValueNotFound(String),
}

fn check_value(tag_value: &String, values: &Vec<String>, key: &String) -> Result<String, TagError> {
    let key = key.to_string();
    let values_str = format!("{}={}", key, values.join(","));

    match values.len() {
        0 => Ok(key),
        _ => match values.contains(tag_value) {
            true => Ok(values_str),
            false => Err(TagError::ValueNotFound(values_str)),
        },
    }
}

fn validate_tags(
    tags: &Vec<Tag>,
    search_key: &String,
    search_tag: &SearchTag,
) -> Option<(String, Option<TagErrors>)> {
    tags.iter()
        .find(|t| match search_tag.values.len() {
            0 => t.key.as_str() == search_key,
            _ => t.key.as_str() == search_key && search_tag.values.contains(&t.value),
        })
        .map(|_tag| {
            let tag_errors = search_tag.secondary.as_ref().map(|ref r| {
                let results = r
                    .iter()
                    .map(
                        |(sk, st)| match tags.iter().find(|t| t.key.as_str() == sk) {
                            Some(tag) => check_value(&tag.value, &st.values, &sk),
                            None => Err(TagError::KeyNotFound(sk.clone())),
                        },
                    )
                    .collect::<Vec<Result<String, TagError>>>();

                let oks = results
                    .iter()
                    .filter(|r| r.is_ok())
                    .map(|r| r.clone().unwrap())
                    .collect::<Vec<String>>();

                let errors = results
                    .iter()
                    .filter(|r| r.is_err())
                    .map(|r| r.clone().unwrap_err())
                    .collect::<Vec<TagError>>();

                TagErrors::new(r.len(), oks, errors)
            });

            (create_key(search_key, &search_tag.values), tag_errors)
        })
}

fn compute_errors(
    element_tags: &Vec<Tag>,
    search_tags: &HashMap<String, SearchTag>,
) -> HashMap<String, Option<TagErrors>> {
    let errors = search_tags
        .iter()
        .map(|(search_key, search_tag)| validate_tags(&element_tags, &search_key, &search_tag))
        .filter_map(|x| x)
        .collect::<HashMap<String, Option<TagErrors>>>();
    // Check Value

    errors
}

#[derive(Serialize, Debug)]
struct TagErrors {
    oks: Vec<String>,
    errors: Vec<TagError>,
    completeness: f64,
}

impl TagErrors {
    fn new(len_tags: usize, oks: Vec<String>, errors: Vec<TagError>) -> Self {
        let completeness = 1.0 - (errors.len() as f64 / len_tags as f64);
        TagErrors {
            oks: oks,
            errors: errors,
            completeness: completeness,
        }
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
        self.props = Some(ElementProps { id: id, user: user });
    }

    fn create_point(&self, geometry_types: &Vec<String>) -> Option<Geometry> {
        if geometry_types.contains(&"points".to_string()) == false {
            return None;
        }

        let geom = Geometry::new(Value::Point(self.coords[0].clone()));
        Some(geom)
    }

    fn create_linestring(&self, geometry_types: &Vec<String>) -> Option<Geometry> {
        if geometry_types.contains(&"linestrings".to_string()) == false {
            return None;
        }
        let geom = Geometry::new(Value::LineString(self.coords.clone()));
        Some(geom)
    }

    fn create_polygon(&self, geometry_types: &Vec<String>) -> Option<Geometry> {
        if geometry_types.contains(&"polygons".to_string()) == false {
            return None;
        }
        let geom = Geometry::new(Value::Polygon(vec![self.coords.clone()]));

        Some(geom)
    }

    fn create_geom(&self, geometry_types: &Vec<String>) -> Option<Geometry> {
        match &self.element_type {
            Some(ElementType::Node) => self.create_point(geometry_types),
            Some(ElementType::Way) => {
                if self.coords.len() == 0 {
                    return None;
                }

                match &self.coords.first() == &self.coords.last() {
                    false => self.create_linestring(geometry_types),
                    true => self.create_polygon(geometry_types),
                }
            }
            _ => panic!("unknown element_type"),
        }
    }

    pub fn to_feature(
        &self,
        search_tags: &HashMap<String, SearchTag>,
        feature_count: &mut HashMap<String, i64>,
        geometry_types: &Vec<String>,
        attributes_count: &mut HashMap<String, i64>,
        completeness_count: &mut HashMap<String, HashMap<String, i64>>,
        contributors: &mut HashMap<String, HashMap<String, i64>>,
    ) -> Option<Feature> {
        let errors = compute_errors(&self.tags, search_tags);
        if errors.len() == 0 {
            return None;
        }
        errors.iter().for_each(|(k, v)| {
            // Add user per feature found.
            if let Some(field) = contributors.get_mut(k) {
                let ref user = self.props.as_ref().unwrap().user;
                if let Some(v) = field.get_mut(user) {
                    *v = *v + 1;
                } else {
                    field.insert(user.to_string(), 1);
                }
            }

            if let Some(v) = feature_count.get_mut(k) {
                *v = *v + 1;
            }

            v.as_ref().map(|tag_error| {
                tag_error.oks.iter().for_each(|ok| {
                    if let Some(v) = attributes_count.get_mut(ok) {
                        *v = *v + 1;
                    }
                });

                if let Some(key) = completeness_count.get_mut(k) {
                    let mut field = "complete";

                    if tag_error.completeness < 1.0 {
                        field = "incomplete";
                    };

                    if let Some(v) = key.get_mut(&field.to_string()) {
                        *v = *v + 1;
                    }
                }
            });
        });

        let feature = self.create_geom(geometry_types).map(|geom| {
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

            Feature {
                bbox: None,
                geometry: Some(geom),
                id: None,
                properties: Some(properties),
                foreign_members: None,
            }
        });

        feature
    }
}
