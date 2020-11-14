use geojson::GeoJson;
use serde::Deserialize;
use std::fs::create_dir;
use uuid::Uuid;

use crate::parser::parse;

use crate::overpass::Overpass;
use crate::storage::LocalStorage;

use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct Campaign {
    pub name: String,
    pub geometry_types: Vec<String>,
    pub tags: HashMap<String, SearchTag>,
    pub geom: GeoJson,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchTag {
    pub values: Vec<String>,
    pub secondary: Option<HashMap<String, SearchTag>>,
}

pub struct CampaignRun {
    source: Overpass,
    storage: LocalStorage,
}

impl CampaignRun {
    pub fn new(campaign: Campaign) -> Self {
        let uuid = Uuid::new_v4();
        let mut buffer = Uuid::encode_buffer();
        let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();

        let storage = LocalStorage::new(&uuid);

        CampaignRun {
            source: Overpass::new(campaign.clone()),
            storage: storage,
        }
    }

    pub fn run(&self) {
        if self.storage.path.exists() == false {
            create_dir(&self.storage.path).unwrap();
        }

        let xml_path = self.storage.overpass();
        let json_path = self.storage.json();

        self.source.fetch_data(&xml_path);
        parse(&xml_path, &json_path);
    }
}
