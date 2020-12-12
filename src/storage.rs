use crate::campaign::{Campaign, Status};
use crate::commands::CommandResult;
use crate::errors::AppError;

use log::{error, info, warn};
use serde_json::{from_str, to_string};
use std::fs::create_dir;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;

use geojson::GeoJson;

#[derive(Clone)]
pub struct LocalStorage {
    pub path: PathBuf,
}

const CAMPAIGN_FILE: &str = "campaign.json";
pub const OUTPUT_FILE: &str = "output.json";

impl LocalStorage {
    pub fn new(storage: &PathBuf) -> Self {
        match create_dir(storage) {
            Ok(()) => info!(
                "{}",
                CommandResult::CreateStorage(storage.display().to_string()).message()
            ),
            Err(_e) => warn!("STORAGE {} EXISTS", storage.display().to_string()),
        };

        LocalStorage {
            path: storage.to_path_buf(),
        }
    }

    pub fn delete_campaign(&self, uuid: &str) -> Result<(), AppError> {
        let path = self.path.join(uuid);

        std::fs::remove_dir_all(path)?;

        Ok(())
    }

    pub fn is_campaign_running(&self, uuid: &str) -> bool {
        let campaign = match self.load_campaign(uuid) {
            Ok(c) => c,
            Err(err) => {
                error!("{}", err.to_string());
                return false;
            }
        };
        match campaign.status.unwrap() {
            Status::Finished => false,
            _ => true,
        }
    }

    pub fn update_campaign(
        &self,
        old_campaign: Campaign,
        new_campaign: Campaign,
    ) -> Result<(), AppError> {
        let uuid = old_campaign.uuid.clone().unwrap();
        let path = self.path.join(uuid).join(CAMPAIGN_FILE);

        let mut file = File::create(path)?;

        let new_campaign = Campaign {
            uuid: old_campaign.uuid,
            created_at: old_campaign.created_at,
            user: old_campaign.user,
            ..new_campaign
        };

        let new_campaign = new_campaign.set_updated_date();

        let serialized = to_string(&new_campaign)?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    pub fn load_campaign(&self, uuid: &str) -> Result<Campaign, AppError> {
        let path = self.path.join(uuid).join(CAMPAIGN_FILE);

        let contents = read_to_string(path)?;

        let campaign: Result<Campaign, AppError> =
            from_str(&contents).map_err(|err| AppError::SerdeError(err.to_string()));

        let campaign = campaign?;

        Ok(campaign)
    }

    pub fn load_results(&self, uuid: &str) -> Result<GeoJson, AppError> {
        let path = self.path.join(uuid).join(OUTPUT_FILE);

        let contents = read_to_string(path)?;

        let results: Result<GeoJson, AppError> =
            from_str(&contents).map_err(|err| AppError::SerdeError(err.to_string()));

        let results = results?;

        Ok(results)
    }

    pub fn save_campaign(&self, campaign: Campaign) -> Result<String, AppError> {
        let uuid = campaign.uuid.clone().unwrap();
        let path = self.path.join(uuid.clone());

        create_dir(path.clone())?;

        let path = path.join(CAMPAIGN_FILE);
        let mut file = File::create(path)?;

        let serialized = to_string(&campaign)?;
        file.write_all(serialized.as_bytes())?;

        Ok(uuid)
    }

    pub fn list_campaigns(&self) -> Result<Vec<Campaign>, AppError> {
        let campaigns = std::fs::read_dir(&self.path)?;

        let campaigns = campaigns
            .map(|c| {
                let dir_entry: Result<Campaign, String> = c
                    .map_err(|e| format!("Unknown error {}", e))
                    .map(|entry| entry.path().join(CAMPAIGN_FILE))
                    .and_then(|path| {
                        std::fs::File::open(&path)
                            .map_err(|_err| format!("Could not open file {}", path.display()))
                    })
                    .and_then(|f| {
                        let campaign: Result<Campaign, String> = serde_json::from_reader(f)
                            .map_err(|e| format!("Could not deserialize file {}", e));

                        campaign
                    })
                    .map(|campaign| campaign.centroid_as_geom());

                dir_entry
            })
            .filter(|c| c.is_ok())
            .map(|c| c.unwrap())
            .collect::<Vec<Campaign>>();

        Ok(campaigns)
    }

    pub fn overpass(&self) -> String {
        format!("{}/overpass.xml", self.path.display())
    }

    pub fn json(&self) -> String {
        format!("{}/features.json", self.path.display())
    }
}
