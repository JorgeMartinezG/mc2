use crate::campaign::Campaign;
use crate::commands::CommandResult;
use crate::notifications::Notifications;
use log::{info, warn};
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

    pub fn save_campaign(&self, campaign: Campaign) -> Result<(), Notifications> {
        let uuid = campaign.uuid.clone().unwrap();
        let path = self.path.join(uuid);

        create_dir(path.clone())?;

        let path = path.join(CAMPAIGN_FILE);
        let mut file = File::create(path)?;

        let serialized =
            to_string(&campaign).map_err(|err| Notifications::SerdeError(err.to_string()))?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    pub fn overpass(&self) -> String {
        format!("{}/overpass.xml", self.path.display())
    }

    pub fn json(&self) -> String {
        format!("{}/features.json", self.path.display())
    }
}
