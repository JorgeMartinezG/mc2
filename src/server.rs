use crate::campaign::{Campaign, CampaignRun};
use crate::commands::CommandResult;
use crate::errors::AppError;
use crate::notifications::Notifications;
use crate::storage::LocalStorage;

use actix_web::middleware::Logger;
use actix_web::{dev::Payload, get, post, web, App, Error, FromRequest, HttpRequest, HttpServer};

use base64::{decode, encode};
use itsdangerous::{default_builder, Signer};

use actix::prelude::{Actor, Addr, Handler, Message, SyncArbiter, SyncContext};

use log::{error, info};
use serde_json::{to_value, Map};

const SECRET_KEY: &str = "pleasechangeme1234";

#[derive(Clone)]
struct McActor {
    storage: LocalStorage,
}

impl Actor for McActor {
    type Context = SyncContext<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
struct McMessage {
    uuid: String,
}

impl Handler<McMessage> for McActor {
    type Result = ();

    fn handle(&mut self, msg: McMessage, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let uuid = msg.uuid.clone();
        info!("Started campaign run - {}", uuid);
        let campaign = self.storage.load_campaign(&uuid).unwrap();

        let run = CampaignRun::new(campaign, self.storage.clone());
        info!("Finished campaign run - {}", uuid);
        run.run();
    }
}

struct User {
    token: String,
}

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<User, Error>>>>;

    fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
        let auth = req
            .headers()
            .get("AUTHORIZATION")
            .unwrap()
            .to_str()
            .unwrap()
            .split(" ")
            .collect::<Vec<&str>>()[1]
            .clone();

        let auth = decode(auth).unwrap();
        let auth = std::str::from_utf8(&auth).unwrap();

        let signer = default_builder(SECRET_KEY).build();
        let token = signer.unsign(auth).expect("errror").to_string();

        Box::pin(async move {
            let user = User { token: token };
            Ok(user)

            //Err(ErrorUnauthorized("unauthorized"))
        })
    }
}

#[get("/token")]
async fn get_token() -> String {
    let signer = default_builder(SECRET_KEY).build();
    let token = encode(signer.sign("12345"));

    println!("{:?}", &token);

    token
}

#[post("/campaign")]
async fn create_campaign(
    campaign: web::Json<Campaign>,
    data: web::Data<AppState>,
) -> Result<String, AppError> {
    let campaign = campaign.into_inner().set_created_date().set_uuid();

    let uuid = data.storage.save_campaign(campaign)?;

    let mut response = Map::new();
    let uuid_value = to_value(&uuid)?;
    response.insert("uuid".to_string(), uuid_value);

    let json_string = serde_json::to_string(&response)?;

    let ref addr = data.addr;

    addr.do_send(McMessage { uuid: uuid });

    Ok(json_string)
}

#[get("/campaign/{uuid}")]
async fn get_campaign(
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
) -> Result<String, AppError> {
    let storage = &data.storage;

    let path = storage.path.join(uuid).join("campaign.json");
    println!("{:?}", path);
    let contents = std::fs::read_to_string(path)?;

    Ok(contents)
}

#[get("/campaigns")]
async fn list_campaigns(data: web::Data<AppState>) -> Result<String, Notifications> {
    let storage = &data.storage;

    let campaigns = std::fs::read_dir(&storage.path)?
        .map(|c| {
            let file = c.unwrap().path().join("campaign.json");
            let file = std::fs::File::open(file).unwrap();
            let campaign: Campaign = serde_json::from_reader(file).unwrap();

            let campaign = campaign.centroid_as_geom();
            campaign
        })
        .collect::<Vec<Campaign>>();

    Ok(serde_json::to_string(&campaigns).unwrap())
}

#[derive(Clone)]
struct AppState {
    storage: LocalStorage,
    addr: Addr<McActor>,
}

#[actix_web::main]
pub async fn serve(storage: LocalStorage) -> Result<CommandResult, Notifications> {
    let server = HttpServer::new(move || {
        let mc_actor = McActor {
            storage: storage.clone(),
        };
        App::new()
            .data(AppState {
                storage: storage.clone(),
                addr: SyncArbiter::start(1, move || mc_actor.clone()),
            })
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .service(
                web::scope("/api/v1/")
                    .service(create_campaign)
                    .service(get_campaign)
                    .service(list_campaigns)
                    .service(get_token),
            )
    })
    .bind("127.0.0.1:8080");

    match server {
        Ok(r) => r.run().await?,
        Err(e) => panic!("{:?}", e),
    }

    Ok(CommandResult::Serve)
}
