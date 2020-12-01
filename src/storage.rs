use crate::campaign::Campaign;
use crate::commands::CommandResult;
use crate::errors::AppError;
use crate::notifications::Notifications;
use log::{error, info, warn};
use serde_json::{from_str, to_string};
use std::fs::create_dir;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone)]
pub struct LocalStorage {
    pub path: PathBuf,
}

const CAMPAIGN_FILE: &str = "campaign.json";

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

    pub fn load_campaign(&self, uuid: &str) -> Result<Campaign, Notifications> {
        let path = self.path.join(uuid).join(CAMPAIGN_FILE);

        let contents = read_to_string(path)?;

        let campaign: Result<Campaign, Notifications> =
            from_str(&contents).map_err(|err| Notifications::SerdeError(err.to_string()));

        let campaign = campaign?;

        Ok(campaign)
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
                    .map(|entry| entry.path().join("campaign.json"))
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
