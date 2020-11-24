use crate::campaign::{Campaign, CampaignRun};
use crate::notifications::Notifications;
use crate::storage::LocalStorage;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde_json;
use std::fs::File;
use uuid::Uuid;

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

pub fn load_campaign(uuid: &str, storage: LocalStorage) -> Result<CommandResult, Notifications> {
    let campaign = storage.load_campaign(uuid)?;

    let run = CampaignRun::new(campaign, storage);
    run.run();

    Ok(CommandResult::GetCampaign(uuid.to_string()))
}

fn create_uuid() -> String {
    let uuid = Uuid::new_v4();
    let mut buffer = Uuid::encode_buffer();
    let uuid = uuid.to_simple().encode_lower(&mut buffer).to_owned();
    uuid
}

pub fn create_campaign(path: &str, storage: LocalStorage) -> Result<CommandResult, Notifications> {
    let uuid = create_uuid();

    let file = File::open(path)?;
    let campaign: Result<Campaign, Notifications> =
        serde_json::from_reader(file).map_err(|err| Notifications::SerdeError(err.to_string()));

    let mut campaign = campaign?;

    campaign.uuid = Some(uuid.clone());

    storage.save_campaign(campaign)?;

    Ok(CommandResult::CreateCampaign(uuid))
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
pub async fn serve() -> Result<CommandResult, Notifications> {
    let server = HttpServer::new(|| App::new().service(hello)).bind("127.0.0.1:8080");

    match server {
        Ok(r) => r.run().await?,
        Err(e) => panic!("{:?}", e),
    }

    Ok(CommandResult::Serve)
}
