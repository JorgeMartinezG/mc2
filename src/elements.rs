use crate::campaign::SearchTag;
use geojson::{Feature, Geometry, Value};
use serde_json::{to_value, Map};
use std::collections::HashMap;
use xml::attribute::OwnedAttribute;

pub fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .unwrap()
        .value
        .clone()
}

fn check_value(tag_value: &String, values: &Vec<String>) -> Option<String> {
    let mut error = None;
    if values.len() != 0 && values.contains(tag_value) == false {
        error = Some(format!("Value mismatch - expected values: {:?}", values));
    }

    error
}

fn validate_tags(
    tags: &Vec<Tag>,
    search_key: &String,
    search_tag: &SearchTag,
) -> (String, Vec<String>) {
    let mut search_errors = Vec::new();

    match tags.iter().find(|t| t.key.as_str() == search_key) {
        Some(tag) => {
            match check_value(&tag.value, &search_tag.values) {
                Some(err) => search_errors.push(err),
                None => (),
            };

            if search_tag.secondary.is_none() {
                return (search_key.to_string(), search_errors);
            }

            search_tag
                .secondary
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|(sk, st)| {
                    match tags.iter().find(|t| t.key.as_str() == sk) {
                        Some(tag) => match check_value(&tag.value, &st.values) {
                            Some(err) => search_errors.push(err),
                            None => (),
                        },
                        None => search_errors.push(format!("Key {} not found", sk)),
                    };
                });
        }
        None => (),
    }

    // Apply secondary check
    (search_key.to_string(), search_errors)
}

fn compute_errors(
    element_tags: &Vec<Tag>,
    search_tags: &HashMap<String, SearchTag>,
) -> HashMap<String, Vec<String>> {
    let errors = search_tags
        .iter()
        .map(|(search_key, search_tag)| validate_tags(&element_tags, &search_key, &search_tag))
        .filter(|x| x.1.len() > 0)
        .collect::<HashMap<String, Vec<String>>>();
    // Check Value

    errors
}

#[derive(Debug)]
pub enum Element {
    Initialized,
    Node(Node),
    Way(Way),
}

impl Element {
    pub fn add_tag(&mut self, tag: Tag) {
        match self {
            Element::Node(ref mut n) => n.tags.push(tag),
            Element::Way(ref mut w) => w.tags.push(tag),
            _ => (),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: i64,
    lat: f64,
    lon: f64,
    pub tags: Vec<Tag>,
}

impl Node {
    fn to_vec(&self) -> Vec<f64> {
        vec![self.lon, self.lat]
    }

    pub fn new(attributes: &Vec<OwnedAttribute>) -> Self {
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

    pub fn to_feature(&self, search_tags: &HashMap<String, SearchTag>) -> Feature {
        let geom = Geometry::new(Value::Point(self.to_vec()));
        let mut properties = Map::new();

        // Compute completeness for primary tag.
        // search_tags.iter().for_each(|st| {
        //     if st.key
        // });
        let errors = compute_errors(&self.tags, search_tags);
        properties.insert("Errors".to_string(), to_value(errors).unwrap());

        self.tags
            .iter()
            .map(|t| properties.insert(t.key.clone(), to_value(t.value.clone()).unwrap()))
            .for_each(drop);

        Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Way {
    pub id: i64,
    pub nodes: Vec<Node>,
    pub tags: Vec<Tag>,
}

impl Way {
    pub fn new(attributes: &Vec<OwnedAttribute>) -> Way {
        let id = find_attribute("id", &attributes)
            .parse::<i64>()
            .expect("Error parsing");

        Way {
            id: id,
            nodes: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn to_feature(&self, search_tags: &HashMap<String, SearchTag>) -> Feature {
        let points = self
            .nodes
            .iter()
            .map(|n| n.to_vec())
            .collect::<Vec<Vec<f64>>>();

        let mut geom = Geometry::new(Value::LineString(points.clone()));

        if &points[0].first() == &points[0].last() {
            geom = Geometry::new(Value::Polygon(vec![points]));
        }
        let mut properties = Map::new();

        let errors = compute_errors(&self.tags, search_tags);
        println!("{:?}", errors);

        self.tags
            .iter()
            .map(|t| properties.insert(t.key.clone(), to_value(t.value.clone()).unwrap()))
            .for_each(drop);

        Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
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
