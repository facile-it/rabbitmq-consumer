pub mod config;
pub mod model;

use std::collections::HashMap;

use crate::config::queue::config::QueueConfig;
use crate::config::queue::model::QueueModel;

pub const TIME_MS_MULTIPLIER: u64 = 1000;
pub const TIME_S_MULTIPLIER: u64 = 60;

pub const DEFAULT_WAIT: u64 = 120;
pub const DEFAULT_TIMEOUT: u64 = 30;

pub enum RetryType {
    Static,
    Incremental,
    Ignored,
}

pub enum RetryMode {
    Normal,
    Retry,
    Forced,
}

pub struct Queue {
    inner: Box<dyn QueueModel>,
    waits: HashMap<(i32, i32), u64>,
}

impl Queue {
    pub fn new(model: Box<dyn QueueModel>) -> Self {
        Queue {
            inner: model,
            waits: HashMap::new(),
        }
    }

    pub fn get_queues(&mut self) -> Vec<QueueConfig> {
        self.inner.get_queues()
    }

    pub fn get_command(&mut self, id: i32) -> String {
        self.inner.get_command(id)
    }

    pub fn get_command_timeout(&mut self, id: i32) -> u64 {
        self.inner.get_command_timeout(id)
    }

    pub fn is_enabled(&mut self, id: i32) -> bool {
        self.inner.is_enabled(id)
    }

    pub fn is_changed(&mut self, id: i32, current_count: i32) -> bool {
        self.inner.is_changed(id, current_count)
    }

    pub fn get_retry_type(&mut self, id: i32) -> RetryType {
        match self.inner.get_queue(id) {
            Some(queue) => match queue.retry_mode.as_str() {
                "incremental" => RetryType::Incremental,
                "ignored" => RetryType::Ignored,
                _ => RetryType::Static,
            },
            None => RetryType::Static,
        }
    }

    pub fn get_queue_wait(&mut self, id: i32, consumer_index: i32) -> u64 {
        let inner = &mut self.inner;
        *self
            .waits
            .entry((id, consumer_index))
            .or_insert_with(|| match inner.get_queue(id) {
                Some(ref queue) => queue.retry_wait * TIME_MS_MULTIPLIER,
                None => DEFAULT_WAIT * TIME_MS_MULTIPLIER,
            })
    }

    pub fn set_queue_wait(
        &mut self,
        id: i32,
        waiting: u64,
        consumer_index: i32,
        retry_mode: RetryMode,
    ) {
        let retry_type = self.get_retry_type(id);
        self.waits.insert(
            (id, consumer_index),
            match retry_mode {
                RetryMode::Normal => waiting * TIME_MS_MULTIPLIER,
                RetryMode::Retry => match retry_type {
                    RetryType::Incremental => {
                        if waiting >= (u64::max_value() / 2) {
                            waiting
                        } else {
                            waiting * 2
                        }
                    }
                    _ => waiting,
                },
                RetryMode::Forced => waiting,
            },
        );
    }
}
