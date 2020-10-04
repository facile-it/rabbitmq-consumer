pub mod events;
mod waiter;

use std::error::Error;
use std::sync::Arc;

use async_std::sync::RwLock;

use crate::client::consumer::{Consumer, ConsumerError, ConsumerResult};
use crate::client::executor::events::EventsHandler;
use crate::client::executor::waiter::Waiter;
use crate::config::Config;

#[derive(Debug)]
pub enum ExecutorResult {
    Restart,
    Wait(ConsumerError, u64),
    Exit,
    Error(ConsumerError),
}

pub struct Executor {
    waiter: Arc<RwLock<Waiter>>,
    consumer: Consumer,
}

impl Executor {
    pub fn new(config: Config) -> Self {
        let waiter = Arc::new(RwLock::new(Waiter::new(
            config.rabbit.reconnections.unwrap_or(0),
        )));

        Executor {
            waiter: waiter.clone(),
            consumer: Consumer::new(config).add_events_hook(waiter),
        }
    }

    pub async fn execute(&mut self) -> ExecutorResult {
        match self.consumer.run().await {
            Ok(ConsumerResult::CountChanged) => ExecutorResult::Restart,
            Ok(ConsumerResult::GenericOk) => ExecutorResult::Exit,
            Err(e) => {
                let waiter = self.waiter.read().await;
                if waiter.is_to_close() {
                    return ExecutorResult::Error(e);
                }

                ExecutorResult::Wait(e, waiter.waiting)
            }
        }
    }
}
