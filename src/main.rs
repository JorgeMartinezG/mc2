mod campaign;
mod elements;
mod overpass;
mod parser;
mod storage;

use campaign::{Campaign, CampaignRun};
use parser::parse;
use serde_json;
use std::fs::create_dir;
use std::path::PathBuf;
use structopt::StructOpt;

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
}

fn main() {
    let opt = Opts::from_args();

    match opt.command {
        Command::CreateCampaign { ref json_path } => println!("{:?}", json_path),
        Command::Run { ref uuid } => println!("{:?}", uuid),
        Command::CreateStore => println!("AAA"),
    }

    println!("{:?}", opt);
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
                        "values": [],
                        "secondary": {
                            "name" : {
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

        let campaign: Campaign = serde_json::from_str(campaign_str).expect("failed reading file");
        parse(
            "/Users/jorge/code/data/test.xml",
            "res.geojson",
            &campaign.tags,
            &campaign.geometry_types,
        );
    }
}
