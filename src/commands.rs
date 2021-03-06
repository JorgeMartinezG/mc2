use crate::campaign::{Campaign, CampaignRun};
use crate::errors::AppError;
use crate::storage::LocalStorage;

use log::{error, info};
use serde_json;
use std::fs::File;
use uuid::Uuid;

use chrono::prelude::{DateTime, Utc};

pub enum CommandResult {
    GetCampaign(String),
    CreateCampaign(String),
    CreateStorage(String),
    Serve,
}

impl CommandResult {
    pub fn message(&self) -> String {
        match self {
            CommandResult::CreateCampaign(uuid) => format!("CAMPAIGN::CREATE::OK::{}", uuid),
            CommandResult::GetCampaign(uuid) => format!("CAMPAIGN::GET::OK::{}", uuid),
            CommandResult::CreateStorage(storage) => format!("STORAGE::CREATE::OK::{}", storage),
            CommandResult::Serve => format!("SERVER::OK"),
        }
    }
}

pub fn load_campaign(uuid: &str, storage: LocalStorage) -> Result<CommandResult, AppError> {
    let campaign = storage
        .load_campaign(uuid)
        .map_err(|err| AppError::IOError(err.to_string()))?;

    let run = CampaignRun::new(campaign, storage);
    run.run();

    Ok(CommandResult::GetCampaign(uuid.to_string()))
}

pub fn create_uuid() -> String {
    let uuid = Uuid::new_v4();
    let mut buffer = Uuid::encode_buffer();
    let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();
    uuid
}

pub fn create_campaign(path: &str, storage: LocalStorage) -> Result<CommandResult, AppError> {
    let uuid = create_uuid();

    let file = File::open(path)?;
    let campaign: Result<Campaign, AppError> =
        serde_json::from_reader(file).map_err(|err| AppError::SerdeError(err.to_string()));

    let mut campaign = campaign?;

    let utc: DateTime<Utc> = Utc::now();
    campaign.uuid = Some(uuid.clone());
    campaign.created_at = Some(utc);

    match storage
        .save_campaign(campaign)
        .map_err(|err| AppError::SerdeError(err.to_string()))
    {
        Ok(v) => info!("Campaign {} created successfully", v),
        Err(err) => error!("{}", err),
    };

    Ok(CommandResult::CreateCampaign(uuid))
}
