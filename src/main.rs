mod campaign;
mod commands;
mod elements;
mod errors;
mod notifications;
mod overpass;
mod parser;
mod server;
mod storage;

use campaign::Campaign;
use commands::{create_campaign, load_campaign, CommandResult};
use log::{error, info};
use notifications::Notifications;
use server::serve;

use serde_json;

use std::path::PathBuf;
use storage::LocalStorage;
use structopt::StructOpt;

use parser::parse;

#[derive(Debug, StructOpt)]
#[structopt(name = "mc2", about = "Command line for MapCampaigner v2")]
struct Opts {
    #[structopt(subcommand)]
    /// The command to run
    command: Command,

    // Debug mode, does not delete temp folder.
    #[structopt(short, long)]
    debug: bool,

    /// Storage folder.
    #[structopt(parse(from_os_str))]
    storage: PathBuf,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Run Campaign computarion.
    #[structopt()]
    Run { uuid: String },

    /// Create storage directory.
    #[structopt()]
    CreateStore,

    /// Create a campaign
    #[structopt()]
    CreateCampaign { json_path: String },

    #[structopt()]
    Serve,
}

fn main() {
    env_logger::init();
    let opt = Opts::from_args();

    let storage = LocalStorage::new(&opt.storage);

    let result = match opt.command {
        Command::CreateCampaign { ref json_path } => create_campaign(json_path, storage),
        Command::Run { ref uuid } => load_campaign(uuid, storage),
        Command::Serve => serve(storage),
        _ => Ok(CommandResult::CreateCampaign("aaa".to_string())),
    };

    match result {
        Ok(c) => info!("{}", c.message()),
        Err(e) => error!("{}", e.to_string()),
    }
}

#[cfg(test)]
mod campaign_test {
    use super::*;
    #[test]
    fn test_load_campaign() {
        let campaign_str = r#"
            {
                "name": "Test Campaign",
                "geometry_types": ["points", "polygons"],
                "tags": {
                    "building": {
                        "values": ["yes"],
                        "secondary": {
                            "name": {
                                "values": []
                            },
                            "building": {
                                "values": []
                            }
                        }
                    }
                },
                "geom": {
                    "type": "FeatureCollection",
                    "features": [{
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [
                                    [
                                        -74.80801105499268,
                                        10.99019103370231
                                    ],
                                    [
                                        -74.80352640151978,
                                        10.994614468861599
                                    ],
                                    [
                                        -74.80504989624023,
                                        10.996931479848776
                                    ],
                                    [
                                        -74.81062889099121,
                                        10.9973948798614
                                    ],
                                    [
                                        -74.81320381164551,
                                        10.993982553614636
                                    ],
                                    [
                                        -74.80801105499268,
                                        10.99019103370231
                                    ]
                                ]
                            ]
                        }
                    }]
                }
            }
        "#;

        let campaign: Result<Campaign, Notifications> = serde_json::from_str(campaign_str)
            .map_err(|err| Notifications::SerdeError(err.to_string()));
        //println!("{:?}", campaign);

        let campaign = campaign.unwrap();
        parse(
            "/Users/jorge/code/data/test.xml",
            "res.geojson",
            &campaign.tags,
            &campaign.geometry_types,
        );
    }
}
