use std::io::Error;

use crate::consumer::{Consumer, ConsumerResult};
use crate::data::config::Config;

const LOOP_WAIT: u64 = 1000;

#[derive(Debug)]
pub enum ExecutorResult {
    Restart,
    Exit(Option<Error>),
    Wait(Error, u64),
}

pub struct Executor {
    connections: i32,
    waiting: u64,
    waiting_times: i32,
    consumer: Consumer,
}

impl Executor {
    pub fn new(config: Config) -> Self {
        Executor {
            connections: config.rabbit.reconnections.unwrap_or(0),
            waiting: LOOP_WAIT,
            waiting_times: 1,
            consumer: Consumer::new(config),
        }
    }

    pub fn execute(&mut self) -> ExecutorResult {
        match self.consumer.run() {
            Ok(ConsumerResult::CountChanged) => {
                self.waiting = LOOP_WAIT;
                self.waiting_times = 1;

                ExecutorResult::Restart
            }
            Ok(ConsumerResult::GenericOk) => ExecutorResult::Exit(None),
            Err(e) => {
                let waiting = self.waiting;

                if self.connections > 0 {
                    if self.waiting_times < self.connections {
                        self.waiting_times += 1;
                    } else {
                        return ExecutorResult::Exit(Some(e));
                    }
                }

                if self.waiting < (u64::max_value() / 2) {
                    self.waiting *= 2;
                }

                ExecutorResult::Wait(e, waiting)
            }
        }
    }
}
