#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;

mod consumer;
mod data;
mod executor;
mod logger;
mod utils;

use std::thread;
use std::time::Duration;

use clap::{App, Arg};

use tokio::prelude::*;

use crate::executor::{Executor, ExecutorResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Dario Cancelliere <dario.cancelliere@facile.it>")
        .about("A configurable RabbitMQ consumer made in Rust, useful for a stable and reliable CLI commands processor.")
        .arg(Arg::with_name("env")
            .short("e")
            .long("env")
            .required(false)
            .takes_value(true)
            .help("Environment for configuration file loading"))
        .arg(Arg::with_name("path")
            .short("p")
            .long("path")
            .required(false)
            .takes_value(true)
            .help("Base config file path"))
        .get_matches();

    let mut executor = Executor::new(data::config::config_loader(
        matches.value_of("env"),
        matches.value_of("path"),
    ))
    .await;

    loop {
        match executor.execute().await {
            ExecutorResult::Restart => logger::log("Consumer count changed, restarting..."),
            ExecutorResult::Wait(error, waiting) => {
                logger::log(&format!(
                    "Error ({:?}), waiting {} seconds...",
                    error,
                    waiting / 1000
                ));

                thread::sleep(Duration::from_millis(waiting));
            }
            ExecutorResult::Exit => {
                logger::log("Process finished, exiting...");

                break;
            }
            ExecutorResult::Error(e) => {
                logger::log(&format!("Error ({:?}), exiting...", e));

                break;
            }
        }
    }
}
