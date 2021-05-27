pub mod consumer;
mod executor;

use std::thread;
use std::time::Duration;

use log::{error, info};

use crate::client::consumer::ConsumerError;
use crate::client::executor::{Executor, ExecutorError, ExecutorStatus};
use crate::config::Config;

type ClientResult<T> = Result<T, ConsumerError>;

pub struct Client {
    executor: Executor,
}

impl Client {
    pub fn new<S: AsRef<str>>(environment: S, path: S) -> Self {
        Client {
            executor: Executor::new(Config::new(environment, path)),
        }
    }

    pub async fn run(&mut self) -> ClientResult<()> {
        loop {
            match self.executor.execute().await {
                Ok(ExecutorStatus::Restart) => info!("Consumer count changed, restarting..."),
                Ok(ExecutorStatus::Exit) => {
                    info!("Process finished, exiting...");

                    break;
                }
                Ok(ExecutorStatus::Killed) => {
                    info!("Process killed, exiting...");

                    break;
                }
                Err(ExecutorError::Wait(error, waiting)) => {
                    error!("Error ({:?}), waiting {} seconds...", error, waiting / 1000);

                    thread::sleep(Duration::from_millis(waiting));
                }
                Err(ExecutorError::Error(e)) => {
                    error!("Error ({:?}), exiting...", e);

                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
