use crate::campaign::Campaign;
use crate::notifications::Notifications;
use crate::storage::LocalStorage;
use serde_json;
use std::fs::File;
use uuid::Uuid;

fn create_uuid() -> String {
    let uuid = Uuid::new_v4();
    let mut buffer = Uuid::encode_buffer();
    let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();
    uuid
}

pub fn create_campaign(path: &str, storage: &LocalStorage) -> Result<String, Notifications> {
    let uuid = create_uuid();

    let file = File::open(path)?;
    let campaign: Result<Campaign, Notifications> =
        serde_json::from_reader(file).map_err(|err| Notifications::SerdeError(err.to_string()));

    let mut campaign = campaign?;

    campaign.uuid = Some(uuid.clone());

    storage.save_campaign(campaign)?;

    Ok(uuid)
}
