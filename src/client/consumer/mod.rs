use std::error::Error;

use async_std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::client::executor::events::{Events, EventsHandler};
use crate::config::Config;

#[derive(Debug, PartialEq)]
pub enum ConsumerResult {
    CountChanged,
    GenericOk,
}

pub struct Consumer {
    hooks: Vec<Arc<RwLock<dyn Events>>>,
}

impl Consumer {
    pub fn new(config: Config) -> Self {
        Self { hooks: Vec::new() }
    }

    pub async fn run(&mut self) -> Result<ConsumerResult, Box<dyn Error>> {
        Ok(ConsumerResult::GenericOk)
    }
}

#[async_trait]
impl EventsHandler for Consumer {
    fn add_events_hook<E: Events + 'static>(mut self, hook: Arc<RwLock<E>>) -> Self {
        self.hooks.push(hook);

        self
    }
}
