use geo::algorithm::centroid::Centroid;

use geo_types::{Geometry, GeometryCollection, MultiPoint, Point};
use serde::{Deserialize, Serialize};

use crate::parser::parse;

use crate::overpass::Overpass;
use crate::storage::LocalStorage;

use std::collections::HashMap;

use chrono::prelude::{DateTime, Utc};

use uuid::Uuid;

use log::info;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    name: String,
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Campaign {
    pub name: String,
    pub geometry_types: Vec<String>,
    pub tags: HashMap<String, SearchTag>,
    pub geom: geojson::GeoJson,
    pub uuid: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub user: Option<User>,
}

impl Campaign {
    pub fn set_uuid(self) -> Self {
        let uuid = Uuid::new_v4();
        let mut buffer = Uuid::encode_buffer();
        let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();

        Campaign {
            uuid: Some(uuid),
            ..self
        }
    }

    pub fn is_creator(&self, user: &User) -> bool {
        let campaign_user = self.user.as_ref().unwrap();

        campaign_user.id == user.id && campaign_user.name == user.name
    }

    pub fn set_created_date(self) -> Self {
        let utc: DateTime<Utc> = Utc::now();
        Campaign {
            created_at: Some(utc),
            ..self
        }
    }

    pub fn set_updated_date(self) -> Self {
        let utc: DateTime<Utc> = Utc::now();
        Campaign {
            updated_at: Some(utc),
            ..self
        }
    }

    pub fn set_user(self, user: User) -> Self {
        Campaign {
            user: Some(user),
            ..self
        }
    }

    pub fn centroid_as_geom(self) -> Self {
        let collection: GeometryCollection<f64> = geojson::quick_collection(&self.geom).unwrap();

        let centroids: MultiPoint<f64> = collection
            .iter()
            .map(|f| match f {
                Geometry::Polygon(p) => p.centroid().unwrap(),
                _ => panic!("Geom not supported"),
            })
            .collect();

        let point = Point::from(centroids.centroid().unwrap());
        let geometry = geojson::Geometry::new(geojson::Value::from(&point));

        let geom = geojson::GeoJson::from(geometry);

        Campaign { geom: geom, ..self }
    }
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
        info!("Started campaign run - {}", self.uuid);

        let xml_path = self.overpass();
        let json_path = self.json();

        self.source.fetch_data(&xml_path);

        parse(&xml_path, &json_path, &self.tags, &self.geometry_types);

        info!("Finished campaign run - {}", self.uuid);
    }
}
