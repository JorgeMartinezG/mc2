use crate::commands::CommandResult;
use crate::notifications::Notifications;
use crate::storage::LocalStorage;

use actix_web::middleware::Logger;
use actix_web::{
    dev::Payload, error::ErrorUnauthorized, get, web, App, Error, FromRequest, HttpRequest,
    HttpServer,
};
use geojson;

use base64::{decode, encode};
use itsdangerous::{default_builder, Signer};

const SECRET_KEY: &str = "pleasechangeme1234";

#[derive(Clone)]
struct AppState {
    storage: LocalStorage,
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

#[get("/campaigns")]
async fn list_campaigns(user: User, data: web::Data<AppState>) -> Result<String, Notifications> {
    println!("{}", user.token);

    let storage = &data.storage;

    let campaigns = std::fs::read_dir(&storage.path)?
        .map(|c| {
            let file = c.unwrap().path().join("output.json");
            let file = std::fs::File::open(file).unwrap();
            let campaign: geojson::FeatureCollection = serde_json::from_reader(file).unwrap();

            println!("{:?}", campaign.foreign_members);

            campaign
        })
        .collect::<Vec<geojson::FeatureCollection>>();

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
            .service(get_token)
    })
    .bind("127.0.0.1:8080");

    match server {
        Ok(r) => r.run().await?,
        Err(e) => panic!("{:?}", e),
    }

    Ok(CommandResult::Serve)
}
