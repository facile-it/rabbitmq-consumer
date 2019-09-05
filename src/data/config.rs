use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use toml;

use serde::Deserialize;

use crate::data::models::QueueSetting;
use crate::logger;
use crate::utils::bool_or_string;
use crate::utils::i32_or_string;
use crate::utils::option_i32_or_string;

#[derive(Deserialize)]
pub struct Config {
    pub rabbit: RabbitConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize, Clone)]
pub struct RabbitConfig {
    pub host: String,
    #[serde(deserialize_with = "i32_or_string")]
    pub port: i32,
    pub username: String,
    pub password: String,
    pub vhost: String,
    pub queues: Vec<QueueSetting>,
    pub queue_prefix: String,
    pub reconnections: Option<i32>,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(deserialize_with = "bool_or_string")]
    pub enabled: bool,
    pub host: String,
    #[serde(deserialize_with = "option_i32_or_string", default)]
    pub port: Option<i32>,
    pub user: String,
    pub password: String,
    pub db_name: String,
    #[serde(deserialize_with = "option_i32_or_string", default)]
    pub retries: Option<i32>,
}

pub fn config_loader(environment: Option<&str>, path: Option<&str>) -> Config {
    let default = "local";

    let path = match path {
        Some(path) => format!("{}/", path),
        None => "config/".into(),
    };

    let environment = format!(
        "{}config_{}.toml",
        path,
        match environment {
            Some(env) => env,
            None => default,
        }
    );

    let config = {
        match File::open(&Path::new(&format!("{}config.toml", path))) {
            Err(_) => match File::open(&Path::new(&environment)) {
                Err(_) => format!("{}config_dev.toml", path),
                Ok(_) => environment,
            },
            Ok(_) => format!("{}config.toml", path),
        }
    };

    let path = Path::new(&config);
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

            let variables: HashMap<_, _> = env::vars().collect();
            for (key, value) in variables {
                configuration =
                    configuration.replace(&format!("\"${}\"", key), &format!("\"{}\"", value));
            }

            toml::from_str(&configuration).expect("Couldn't load the configuration file.")
        }
    }
}
