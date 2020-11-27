pub mod consumer;
mod executor;

use std::thread;
use std::time::Duration;

use crate::client::consumer::ConsumerError;
use crate::client::executor::{Executor, ExecutorResult};
use crate::config::Config;

pub enum ClientResult {
    Ok,
    ConsumerError(ConsumerError),
}

pub struct Client {
    executor: Executor,
}

impl Client {
    pub fn new<S: AsRef<str>>(environment: S, path: S) -> Self {
        Client {
            executor: Executor::new(Config::new(environment, path)),
        }
    }

    pub async fn run(&mut self) -> ClientResult {
        loop {
            match self.executor.execute().await {
                ExecutorResult::Restart => info!("Consumer count changed, restarting..."),
                ExecutorResult::Wait(error, waiting) => {
                    info!("Error ({:?}), waiting {} seconds...", error, waiting / 1000);

                    thread::sleep(Duration::from_millis(waiting));
                }
                ExecutorResult::Exit => {
                    info!("Process finished, exiting...");

                    break;
                }
                ExecutorResult::Killed => {
                    info!("Process killed, exiting...");

                    break;
                }
                ExecutorResult::Error(e) => {
                    info!("Error ({:?}), exiting...", e);

                    return ClientResult::ConsumerError(e);
                }
            }
        }

        ClientResult::Ok
    }
}
