use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;

use crate::consumer::{Consumer, ConsumerResult, Events};
use crate::data::config::Config;

const LOOP_WAIT: u64 = 500;

#[derive(Debug)]
pub enum ExecutorResult {
    Restart,
    Wait(Error, u64),
    Exit,
    Error(Error),
}

pub struct Executor {
    waiter: Rc<RefCell<Waiter>>,
    consumer: Consumer,
}

impl Executor {
    pub fn new(config: Config) -> Self {
        let waiter = Rc::new(RefCell::new(Waiter::new(
            config.rabbit.reconnections.unwrap_or(0),
            LOOP_WAIT,
            1,
        )));

        let mut consumer = Consumer::new(config);
        consumer.add_events_hook(waiter.clone());

        Executor { waiter, consumer }
    }

    pub fn execute(&mut self) -> ExecutorResult {
        match self.consumer.run() {
            Ok(ConsumerResult::CountChanged) => ExecutorResult::Restart,
            Ok(ConsumerResult::GenericOk) => ExecutorResult::Exit,
            Err(e) => {
                let waiter = self.waiter.borrow();
                if waiter.is_to_close() {
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
    pub fn new(connections: i32, waiting: u64, waiting_times: i32) -> Self {
        Waiter {
            connections,
            waiting,
            waiting_times,
        }
    }

    pub fn is_to_close(&self) -> bool {
        if self.connections > 0 {
            if self.waiting_times < self.connections {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl Events for Waiter {
    fn on_connect(&mut self, _host: &str, _port: i32) {
        self.waiting = LOOP_WAIT;
        self.waiting_times = 1;
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
