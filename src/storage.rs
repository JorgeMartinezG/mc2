use crate::campaign::Campaign;
use crate::notifications::Notifications;
use serde_json::to_string;
use std::fs::create_dir;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct LocalStorage {
    pub path: PathBuf,
}

impl LocalStorage {
    pub fn new(storage: &PathBuf) -> Self {
        match create_dir(storage) {
            Ok(()) => println!("Storage directory created successfully"),
            Err(_e) => println!("Storage directory already exists"),
        };

        LocalStorage {
            path: storage.to_path_buf(),
        }
    }

    pub fn save_campaign(&self, campaign: Campaign) -> Result<(), Notifications> {
        let uuid = campaign.uuid.clone().unwrap();
        let path = self.path.join(uuid);

        create_dir(path.clone())?;

        let path = path.join("campaign.json");
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
