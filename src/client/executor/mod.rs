pub mod events;
mod waiter;

use async_std::sync::{Arc, RwLock};

use crate::client::consumer::{Consumer, ConsumerError, ConsumerStatus};
use crate::client::executor::events::{Events, EventsHandler};
use crate::client::executor::waiter::Waiter;
use crate::config::Config;

#[derive(Debug)]
pub enum ExecutorStatus {
    Restart,
    Exit,
    Killed,
}

#[derive(Debug)]
pub enum ExecutorError {
    Error(ConsumerError),
    Wait(ConsumerError, u64),
}

type ExecutorResult<T> = Result<T, ExecutorError>;

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

    pub async fn execute(&mut self) -> ExecutorResult<ExecutorStatus> {
        match self.consumer.run().await {
            Ok(ConsumerStatus::CountChanged) => Ok(ExecutorStatus::Restart),
            Ok(ConsumerStatus::ConsumerChanged) => Ok(ExecutorStatus::Restart),
            Ok(ConsumerStatus::GenericOk) => Ok(ExecutorStatus::Exit),
            Ok(ConsumerStatus::Killed) => Ok(ExecutorStatus::Killed),
            Err(e) => {
                let mut waiter = self.waiter.write().await;
                waiter.on_error(&format!("{:?}", e));

                if waiter.is_to_close() {
                    return Err(ExecutorError::Error(e));
                }

                Err(ExecutorError::Wait(e, waiter.waiting))
            }
        }
    }
}
