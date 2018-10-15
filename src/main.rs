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

mod consumer;
mod data;
mod logger;

use std::env;
use std::thread;
use std::time::Duration;

use consumer::{Consumer, ConsumerResult};

const LOOP_WAIT: u64 = 1000;

fn main() {
    let mut consumer = Consumer::new(data::config::config_loader(env::args().collect()));
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
