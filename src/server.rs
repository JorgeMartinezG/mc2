use crate::campaign::{Campaign, CampaignRun, Status, User};
use crate::commands::CommandResult;
use crate::errors::AppError;
use crate::storage::{LocalStorage, OUTPUT_FILE};

use actix_web::middleware::{Compress, Logger};
use actix_web::{
    delete, dev::BodyEncoding, dev::Payload, error::ErrorUnauthorized, get, http::ContentEncoding,
    patch, post, web, App, Error, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};

use base64::{decode, encode};
use itsdangerous::{default_builder, Signer};

use actix::prelude::{Actor, Addr, Handler, Message, SyncArbiter, SyncContext};
use geojson::{GeoJson, PolygonType, Value};

use log::error;
use serde_json::{to_value, Map};

use actix_files::NamedFile;

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
        .set_user(user)
        .set_status(Status::Created);

    let ref geom = campaign.geom;
    let feature_collection = match geom {
        geojson::GeoJson::FeatureCollection(f) => f,
        _ => {
            return HttpResponse::BadRequest()
                .content_type("text/plain")
                .body("Geojson must be FeatureCollection")
        }
    };

    let geometries = feature_collection
        .features
        .iter()
        .map(|f| {
            let ref value = f.geometry.as_ref().unwrap().value;

            match value {
                Value::Polygon(p) => true,
                _ => false,
            }
        })
        .filter(|x| x == &false)
        .collect::<Vec<bool>>();

    if geometries.len() > 0 {
        return HttpResponse::BadRequest().body("Polygon geometry supported only");
    }

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

    let status = storage
        .load_campaign(&uuid)
        .map_err(|err| match err {
            AppError::NotFound => {
                HttpResponse::NotFound().body(format!("Campaign {} not found", uuid))
            }
            _ => HttpResponse::InternalServerError().body("Error found loading the campaign"),
        })
        .and_then(|c| match c.is_creator(&user) {
            true => storage
                .update_campaign(c, campaign.into_inner())
                .map_err(|_err| {
                    HttpResponse::InternalServerError().body("Could not update campaign")
                }),
            false => Err(HttpResponse::Forbidden().body("Not Allowed")),
        });

    match status {
        Ok(_ok) => HttpResponse::Ok().body(""),
        Err(e) => e,
    }
}

#[delete("/campaign/{uuid}")]
async fn delete_campaign(
    user: User,
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let storage = &data.storage;

    let status = storage
        .load_campaign(&uuid)
        .map_err(|err| match err {
            AppError::NotFound => {
                HttpResponse::NotFound().body(format!("Campaign {} not found", uuid))
            }
            _ => HttpResponse::InternalServerError().body("An error ocurred loading the campaign"),
        })
        .map(|c| c.is_creator(&user))
        .and_then(|is_user| match is_user {
            true => storage.delete_campaign(&uuid).map_err(|_err| {
                HttpResponse::InternalServerError().body("Could not delete campaign")
            }),
            false => Err(HttpResponse::Forbidden().body("Not Allowed")),
        });

    match status {
        Ok(_ok) => HttpResponse::Ok().body(""),
        Err(e) => e,
    }
}

#[get("/results/{uuid}")]
async fn get_results(
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let storage = &data.storage;

    if storage.is_campaign_running(&uuid) == true {
        return HttpResponse::Conflict().body(format!("Campaign {} is running", uuid));
    }

    let path = storage.path.join(uuid).join(OUTPUT_FILE);
    match NamedFile::open(path).respond_to(&req).await {
        Ok(mut r) => HttpResponse::Ok()
            .encoding(ContentEncoding::Br)
            .streaming(r.take_body()),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[get("/campaign/{uuid}")]
async fn get_campaign(
    web::Path(uuid): web::Path<String>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let storage = &data.storage;

    match storage.load_campaign(&uuid) {
        Ok(campaign) => HttpResponse::Ok()
            .content_type("application/json")
            .json(campaign),
        Err(e) => match e {
            AppError::NotFound => {
                HttpResponse::NotFound().body(format!("Campaign {} not found", uuid))
            }
            _ => HttpResponse::InternalServerError().body(""),
        },
    }
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
pub async fn serve(storage: LocalStorage) -> Result<CommandResult, AppError> {
    let server = HttpServer::new(move || {
        let mc_actor = McActor {
            storage: storage.clone(),
        };
        App::new()
            .data(AppState {
                storage: storage.clone(),
                addr: SyncArbiter::start(1, move || mc_actor.clone()),
            })
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                actix_web::error::InternalError::from_response(
                    "",
                    HttpResponse::BadRequest()
                        .content_type("application/json")
                        .body(format!(r#"{{"error":"{}"}}"#, err)),
                )
                .into()
            }))
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(Compress::default())
            .service(
                web::scope("/api/v1/")
                    .service(create_campaign)
                    .service(get_campaign)
                    .service(get_results)
                    .service(delete_campaign)
                    .service(update_campaign)
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
