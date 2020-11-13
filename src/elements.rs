use geojson::{Feature, Geometry, Value};
use serde_json::{to_value, Map};
use xml::attribute::OwnedAttribute;

pub fn find_attribute(name: &str, attributes: &Vec<OwnedAttribute>) -> String {
    attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .unwrap()
        .value
        .clone()
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

    pub fn to_feature(&self) -> Feature {
        let geom = Geometry::new(Value::Point(self.to_vec()));
        let mut properties = Map::new();

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

    pub fn to_feature(&self) -> Feature {
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
