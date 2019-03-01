#![type_length_limit = "33554432"]
extern crate env_logger;
extern crate futures;
extern crate lapin_async;
extern crate lapin_futures as lapin;
extern crate serde;
extern crate tokio;
extern crate tokio_current_thread;
extern crate tokio_io;
extern crate tokio_process;
extern crate tokio_signal;
extern crate tokio_timer;
extern crate toml;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate base64;
extern crate chrono;
extern crate clap;

mod consumer;
mod data;
mod logger;

use std::thread;
use std::time::Duration;

use clap::{App, Arg};

use crate::consumer::{Consumer, ConsumerResult};

const LOOP_WAIT: u64 = 1000;

fn main() {
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

    let mut consumer = Consumer::new(data::config::config_loader(
        matches.value_of("env"),
        matches.value_of("path"),
    ));

    loop {
        match consumer.run() {
            Ok(ConsumerResult::CountChanged) => {
                logger::log("Consumer count changed, restarting...")
            }
            Ok(ConsumerResult::GenericOk) => {
                logger::log("Process finished, exiting...");

                break;
            }
            Err(e) => logger::log(&format!("Error ({:?}), restarting...", e)),
        }

        thread::sleep(Duration::from_millis(LOOP_WAIT));
    }
}
