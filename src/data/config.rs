use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use toml;

use data::models::QueueSetting;
use logger;

#[derive(Deserialize)]
pub struct Config {
    pub rabbit: RabbitConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize, Clone)]
pub struct RabbitConfig {
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub vhost: String,
    pub queues: Vec<QueueSetting>,
    pub queue_prefix: String,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub enabled: bool,
    pub host: String,
    pub port: Option<i32>,
    pub user: String,
    pub password: String,
    pub db_name: String,
    pub retries: Option<i32>,
}

pub fn config_loader(args: Vec<String>) -> Config {
    let default = "local";
    let environment = match args.get(1) {
        Some(e) => {
            if e == "--env" {
                match args.get(2) {
                    Some(name) => name,
                    None => default,
                }
            } else {
                default
            }
        }
        None => default,
    };

    let environment = format!("config/config_{}.toml", environment);
    let config = {
        match File::open(&Path::new("config/config.toml")) {
            Err(_) => match File::open(&Path::new(&environment)) {
                Err(_) => "config/config_dev.toml",
                Ok(_) => &environment,
            },
            Ok(_) => "config/config.toml",
        }
    };

    let path = Path::new(config);
    let display = path.display();
    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn't open {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let mut configuration = String::new();
    match file.read_to_string(&mut configuration) {
        Err(why) => panic!("Couldn't read {}: {}", display, why.description()),
        Ok(_) => {
            logger::log(format!("{} loaded correctly.", display));

            toml::from_str(&configuration).expect("Couldn't load the configuration file.")
        }
    }
}
