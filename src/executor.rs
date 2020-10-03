use std::io::Error;
use std::sync::Arc;

use async_std::sync::RwLock;

use crate::consumer::{Consumer, ConsumerResult, Events};
use crate::data::config::Config;

#[derive(Debug)]
pub enum ExecutorResult {
    Restart,
    Wait(Error, u64),
    Exit,
    Error(Error),
}

pub struct Executor {
    waiter: Arc<RwLock<Waiter>>,
    consumer: Consumer,
}

impl Executor {
    pub async fn new(config: Config) -> Self {
        let waiter = Arc::new(RwLock::new(Waiter::new(
            config.rabbit.reconnections.unwrap_or(0),
        )));

        let mut consumer = Consumer::new(config);
        consumer.add_events_hook(waiter.clone()).await;

        Executor { waiter, consumer }
    }

    pub async fn execute(&mut self) -> ExecutorResult {
        match self.consumer.run() {
            Ok(ConsumerResult::CountChanged) => ExecutorResult::Restart,
            Ok(ConsumerResult::GenericOk) => ExecutorResult::Exit,
            Err(e) => {
                if self.waiter.read().await.is_to_close() {
                    return ExecutorResult::Error(e);
                }

                ExecutorResult::Wait(e, waiter.waiting)
            }
        }
    }
}

struct Waiter {
    connections: i32,
    waiting: u64,
    waiting_times: i32,
}

impl Waiter {
    const LOOP_WAIT: u64 = 500;

    pub fn new(connections: i32) -> Self {
        Waiter {
            connections,
            waiting: Self::LOOP_WAIT,
            waiting_times: 0,
        }
    }

    pub fn is_to_close(&self) -> bool {
        if self.connections > 0 {
            if self.waiting_times < self.connections {
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}

impl Events for Waiter {
    fn on_connect(&mut self, _host: &str, _port: i32) {
        self.waiting = Self::LOOP_WAIT;
        self.waiting_times = 0;
    }

    fn on_error(&mut self, _error: &str) {
        if self.connections > 0 {
            if self.waiting_times < self.connections {
                self.waiting_times += 1;
            }
        }

        if self.waiting < (u64::max_value() / 2) {
            self.waiting *= 2;
        }
    }
}
