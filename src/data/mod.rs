pub mod config;
pub mod database;
pub mod models;
pub mod plain;
mod schema;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use chrono;

use self::models::QueueSetting;

const TIME_MS_MULTIPLIER: u64 = 1000;
const TIME_S_MULTIPLIER: u64 = 60;

const DEFAULT_WAIT: u64 = 120;
const DEFAULT_TIMEOUT: u64 = 30;

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

pub trait Data {
    fn get_queues(&mut self) -> Vec<QueueSetting>;
    fn get_queue(&mut self, id: i32) -> Option<QueueSetting>;

    fn get_command(&mut self, id: i32) -> String {
        match self.get_queue(id) {
            Some(queue) => queue.command,
            None => String::new(),
        }
    }

    fn get_command_timeout(&mut self, id: i32) -> u64 {
        (match self.get_queue(id) {
            Some(queue) => queue.command_timeout.unwrap_or(DEFAULT_TIMEOUT),
            None => DEFAULT_TIMEOUT,
        } * TIME_S_MULTIPLIER)
            * TIME_MS_MULTIPLIER
    }

    fn is_enabled(&mut self, id: i32) -> bool {
        match self.get_queue(id) {
            Some(queue) => {
                let now = Some(chrono::Local::now().naive_local().time());

                queue.enabled && {
                    if queue.start_hour.is_some() && queue.end_hour.is_some() {
                        queue.start_hour.le(&now) && queue.end_hour.ge(&now)
                    } else {
                        true
                    }
                }
            }
            None => false,
        }
    }

    fn is_changed(&mut self, id: i32, current_count: i32) -> bool {
        match self.get_queue(id) {
            Some(p) => p.count != current_count,
            None => false,
        }
    }
}

pub struct DatabasePlain {
    inner: Box<dyn Data>,
    waits: HashMap<(i32, i32), u64>,
}

impl DatabasePlain {
    pub fn new(data: Box<dyn Data>) -> Self {
        DatabasePlain {
            inner: data,
            waits: HashMap::new(),
        }
    }

    pub fn get_queues(&mut self) -> Vec<QueueSetting> {
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
