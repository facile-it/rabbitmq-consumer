mod database;
mod file;
mod queue;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::Path;

use toml;

use serde::Deserialize;

use crate::config::queue::config::QueueConfig;
use crate::logger;
use crate::utils::{bool_or_string, option_i32_or_string, u16_or_string};

#[derive(Deserialize)]
pub struct Config {
    pub rabbit: RabbitConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize, Clone)]
pub struct RabbitConfig {
    pub host: String,
    #[serde(deserialize_with = "u16_or_string")]
    pub port: u16,
    pub username: String,
    pub password: String,
    pub vhost: String,
    pub queues: Vec<QueueConfig>,
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

impl Config {
    pub fn new<S: AsRef<str>>(environment: S, path: S) -> Self {
        let environment = format!("{}/config_{}.toml", path.as_ref(), environment.as_ref());
        let config = {
            match File::open(&Path::new(&format!("{}/config.toml", path.as_ref()))) {
                Err(_) => match File::open(&Path::new(&environment)) {
                    Err(_) => format!("{}/config_dev.toml", path.as_ref()),
                    Ok(_) => environment,
                },
                Ok(_) => format!("{}/config.toml", path.as_ref()),
            }
        };

        match crystalsoft_utils::read_file_string(&config) {
            Err(why) => panic!("Couldn't read {}: {:#?}", path.as_ref(), why),
            Ok(mut configuration) => {
                logger::log(format!("\"{}\" loaded correctly.", config));

                let variables: HashMap<_, _> = env::vars().collect();
                for (key, value) in variables {
                    configuration =
                        configuration.replace(&format!("\"${}\"", key), &format!("\"{}\"", value));
                }

                toml::from_str(&configuration).expect("Couldn't load the configuration file.")
            }
        }
    }
}
