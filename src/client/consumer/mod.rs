mod connection;

use std::error::Error;

use async_std::sync::{Arc, RwLock};

use crate::client::consumer::connection::{Connection, ConnectionError};
use crate::client::executor::events::{Events, EventsHandler};
use crate::config::Config;
use crate::logger;

#[derive(Debug, PartialEq)]
pub enum ConsumerResult {
    CountChanged,
    GenericOk,
}

#[derive(Debug)]
pub enum ConsumerError {
    GenericError,
    ConnectionError(ConnectionError),
}

pub struct Consumer {
    connection: Connection,
    hooks: Vec<Arc<RwLock<dyn Events>>>,
    config: Config,
}

impl Consumer {
    pub fn new(config: Config) -> Self {
        logger::log(&format!(
            "Connection to {}:{}",
            config.rabbit.host, config.rabbit.port
        ));

        Self {
            connection: Connection::new(config.rabbit.clone()),
            hooks: Vec::new(),
            config,
        }
    }

    pub async fn run(&mut self) -> Result<ConsumerResult, ConsumerError> {
        match self.connection.get_connection().await {
            Ok(connection) => {
                for hook in &self.hooks {
                    hook.write()
                        .await
                        .on_connect(&self.config.rabbit.host, self.config.rabbit.port);
                }

                Ok(ConsumerResult::GenericOk)
            }
            Err(e) => {
                for hook in &self.hooks {
                    hook.write().await.on_error(&format!("{:?}", e));
                }

                Err(ConsumerError::ConnectionError(e))
            }
        }
    }
}

impl EventsHandler for Consumer {
    fn add_events_hook<E: Events + 'static>(mut self, hook: Arc<RwLock<E>>) -> Self {
        self.hooks.push(hook);

        self
    }
}
