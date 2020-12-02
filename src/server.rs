use crate::campaign::{Campaign, CampaignRun, User};
use crate::commands::CommandResult;
use crate::errors::AppError;
use crate::notifications::Notifications;
use crate::storage::LocalStorage;

use actix_web::middleware::Logger;
use actix_web::{
    dev::Payload, error::ErrorUnauthorized, get, patch, post, web, App, Error, FromRequest,
    HttpRequest, HttpResponse, HttpServer,
};

use base64::{decode, encode};
use itsdangerous::{default_builder, Signer};

use actix::prelude::{Actor, Addr, Handler, Message, SyncArbiter, SyncContext};

use log::error;
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
        let campaign = self.storage.load_campaign(&uuid).unwrap();
        let run = CampaignRun::new(campaign, self.storage.clone());
        run.run();
    }
}

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<User, Error>>>>;

    fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
        let header = req.headers().get("AUTHORIZATION");

        let token: Result<&str, Error> = header
            .ok_or(ErrorUnauthorized("Token not found"))
            .and_then(|h| {
                h.to_str()
                    .map_err(|_e| ErrorUnauthorized("Invalid Token I"))
            });

        let token = match token {
            Ok(t) => t,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        //.map(|v| v.split(" ").collect::<Vec<&str>>())
        //.map(|vec| vec[0].clone());
        let user_str = decode(token)
            .map_err(|_e| ErrorUnauthorized("Could not decode token I"))
            .and_then(|r| {
                String::from_utf8(r).map_err(|_e| ErrorUnauthorized("Could not decode token II"))
            });

        let user_str = match user_str {
            Ok(t) => t,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let signer = default_builder(SECRET_KEY).build();
        let unsigned = signer
            .unsign(&user_str)
            .map_err(|_e| ErrorUnauthorized("Unsigned token"));

        let user: Result<User, Error> = unsigned.and_then(|unsigned| {
            serde_json::from_str(unsigned).map_err(|_e| ErrorUnauthorized("Invalid user"))
        });

        Box::pin(async move { user })
    }
}

#[post("/token")]
async fn create_token(user: web::Json<User>) -> HttpResponse {
    let signer = default_builder(SECRET_KEY).build();

    let output = serde_json::to_string(&user.into_inner())
        .map(|json| signer.sign(json))
        .map(|json| encode(json))
        .map(|token| {
            let mut json = Map::new();
            let token_value = to_value(&token).unwrap();
            json.insert("token".to_string(), token_value);
            json
        })
        .and_then(|str_data| serde_json::to_string(&str_data));

    match output {
        Ok(str_data) => HttpResponse::Ok()
            .content_type("application/json")
            .body(str_data),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().body("Server error")
        }
    }
}

#[post("/campaign")]
async fn create_campaign(
    user: User,
    campaign: web::Json<Campaign>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let campaign = campaign
        .into_inner()
        .set_created_date()
        .set_uuid()
        .set_user(user);

    let saved = data.storage.save_campaign(campaign);

    let resp = saved
        .map(|uuid| {
            let ref addr = data.addr;
            addr.do_send(McMessage { uuid: uuid.clone() });
            uuid
        })
        .and_then(|uuid| {
            let mut response = Map::new();
            let uuid_value = to_value(&uuid).unwrap();
            response.insert("uuid".to_string(), uuid_value);

            let json_string = serde_json::to_string(&response)?;

            Ok(json_string)
        });

    match resp {
        Ok(j) => HttpResponse::Ok().content_type("application/json").body(j),
        Err(e) => {
            error!("{:?}", e);
            HttpResponse::InternalServerError().body("An error ocurred")
        }
    }
}

#[patch("/campaign/{uuid}")]
async fn update_campaign(
    user: User,
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
    campaign: web::Json<Campaign>,
) -> HttpResponse {
    let storage = &data.storage;

    let status = storage.load_campaign(&uuid).and_then(|old_campaign| {
        if old_campaign.is_creator(&user) == false {
            return Err(AppError::Forbidden("Not allowed".to_string()));
        }

        storage.update_campaign(&uuid, campaign.into_inner())
    });

    match status {
        Ok(_ok) => HttpResponse::Ok().body("Campaign updated successfully"),
        Err(err) => match err {
            AppError::Forbidden(_) => HttpResponse::Forbidden().body("Forbidden"),
            _ => HttpResponse::InternalServerError().body("Error"),
        },
    }
}

#[get("/campaign/{uuid}")]
async fn delete_campaign(
    user: User,
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let storage = &data.storage;

    let ok_to_delete = storage.load_campaign(&uuid).map(|c| c.is_creator(&user));

    let status = match ok_to_delete {
        Ok(same_user) => {
            if same_user == true {
                storage
                    .delete_campaign(&uuid)
                    .map(|_k| "ok")
                    .map_err(|_err| "error")
            } else {
                Err("forbidden")
            }
        }
        Err(_e) => Err("error"),
    };

    match status {
        Ok(_ok) => HttpResponse::Ok().body(""),
        Err(e) => match e {
            "forbidden" => HttpResponse::Forbidden().body("Forbidden"),
            _ => HttpResponse::InternalServerError().body("Error"),
        },
    }
}

#[get("/campaign/{uuid}")]
async fn get_campaign(
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
) -> Result<String, AppError> {
    let storage = &data.storage;

    let path = storage.path.join(uuid).join("campaign.json");
    let contents = std::fs::read_to_string(path)?;

    Ok(contents)
}

#[get("/campaigns")]
async fn list_campaigns(data: web::Data<AppState>) -> HttpResponse {
    let campaigns = &data.storage.list_campaigns();

    match campaigns {
        Ok(c) => HttpResponse::Ok().content_type("application/json").json(c),
        Err(e) => {
            error!("{:?}", e);
            HttpResponse::InternalServerError().body("")
        }
    }
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
                    .service(create_token),
            )
    })
    .bind("127.0.0.1:8080");

    match server {
        Ok(r) => r.run().await?,
        Err(e) => panic!("{:?}", e),
    }

    Ok(CommandResult::Serve)
}
