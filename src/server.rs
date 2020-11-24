use crate::campaign::Campaign;
use crate::commands::CommandResult;
use crate::notifications::Notifications;
use crate::storage::LocalStorage;

use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpServer};

#[derive(Clone)]
struct AppState {
    storage: LocalStorage,
}

#[get("/campaigns")]
async fn list_campaigns(data: web::Data<AppState>) -> Result<String, Notifications> {
    let storage = &data.storage;

    let campaigns = std::fs::read_dir(&storage.path)?
        .map(|c| {
            let file = c.unwrap().path().join("campaign.json");
            let file = std::fs::File::open(file).unwrap();
            let campaign: Campaign = serde_json::from_reader(file).unwrap();

            campaign
        })
        .collect::<Vec<Campaign>>();

    Ok(serde_json::to_string(&campaigns).unwrap())
}

#[actix_web::main]
pub async fn serve(storage: LocalStorage) -> Result<CommandResult, Notifications> {
    let server = HttpServer::new(move || {
        App::new()
            .data(AppState {
                storage: storage.clone(),
            })
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .service(list_campaigns)
    })
    .bind("127.0.0.1:8080");

    match server {
        Ok(r) => r.run().await?,
        Err(e) => panic!("{:?}", e),
    }

    Ok(CommandResult::Serve)
}
