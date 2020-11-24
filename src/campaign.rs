use geojson::GeoJson;
use serde::{Deserialize, Serialize};

use crate::parser::parse;

use crate::overpass::Overpass;
use crate::storage::LocalStorage;

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Campaign {
    pub name: String,
    pub geometry_types: Vec<String>,
    pub tags: HashMap<String, SearchTag>,
    pub geom: GeoJson,
    pub uuid: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchTag {
    pub values: Vec<String>,
    pub secondary: Option<HashMap<String, SearchTag>>,
}

pub struct CampaignRun {
    source: Overpass,
    storage: LocalStorage,
    tags: HashMap<String, SearchTag>,
    geometry_types: Vec<String>,
    uuid: String,
}

impl CampaignRun {
    pub fn new(campaign: Campaign, storage: LocalStorage) -> Self {
        CampaignRun {
            source: Overpass::new(campaign.clone()),
            storage: storage,
            tags: campaign.tags.clone(),
            geometry_types: campaign.geometry_types.clone(),
            uuid: campaign.uuid.unwrap(),
        }
    }

    fn overpass(&self) -> String {
        self.storage
            .path
            .join(self.uuid.clone())
            .join("overpass.xml")
            .display()
            .to_string()
    }

    fn json(&self) -> String {
        self.storage
            .path
            .join(self.uuid.clone())
            .join("output.json")
            .display()
            .to_string()
    }

    pub fn run(&self) {
        let xml_path = self.overpass();
        let json_path = self.json();

        self.source.fetch_data(&xml_path);
        parse(&xml_path, &json_path, &self.tags, &self.geometry_types);
    }
}
