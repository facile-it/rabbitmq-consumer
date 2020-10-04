use std::panic;

use async_std::net::ToSocketAddrs;
use async_std::sync::{Arc, RwLock};

use crate::config::queue::{self, config::QueueConfig, Queue, RetryMode, RetryType};
use crate::config::{file::File, Config};

#[test]
fn file_read_dev() {
    let result = panic::catch_unwind(|| Config::new("dev", "config"));

    assert!(result.is_ok());
}

#[test]
fn file_read_prod() {
    let result = panic::catch_unwind(|| {
        Config::new("prod", "config");
    });

    assert!(result.is_ok());
}

#[tokio::test]
async fn address() {
    let config = Config::new("dev", "config");
    let address = format!("{}:{}", config.rabbit.host, config.rabbit.port)
        .to_socket_addrs()
        .await;

    assert!(address.is_ok());
    assert!(address.unwrap().next().is_some());
}

#[tokio::test]
async fn waits() {
    let config = Config::new("dev", "config");
    let data = Arc::new(RwLock::new(Queue::new(Box::new(File::new(
        config.rabbit.queues.clone(),
    )))));

    const TEST_WAIT: u64 = 120;

    let mut data = data.write().await;
    for queue in config.rabbit.queues {
        for consumer_index in 0..queue.count {
            assert_eq!(
                data.get_queue_wait(queue.id, consumer_index),
                queue.retry_wait * queue::TIME_MS_MULTIPLIER
            );

            data.set_queue_wait(queue.id, TEST_WAIT, consumer_index, RetryMode::Normal);

            assert_eq!(
                data.get_queue_wait(queue.id, consumer_index),
                TEST_WAIT * queue::TIME_MS_MULTIPLIER
            );

            data.set_queue_wait(queue.id, TEST_WAIT, consumer_index, RetryMode::Retry);

            match data.get_retry_type(queue.id) {
                RetryType::Ignored => {
                    assert_eq!(data.get_queue_wait(queue.id, consumer_index), TEST_WAIT);
                }
                RetryType::Incremental => {
                    assert_eq!(data.get_queue_wait(queue.id, consumer_index), TEST_WAIT * 2);
                }
                RetryType::Static => {
                    assert_eq!(data.get_queue_wait(queue.id, consumer_index), TEST_WAIT);
                }
            }

            for wait in vec![
                u64::max_value(),
                ((u64::max_value() as f64 / 1.5) as u64),
                ((u64::max_value() as f64 / 2.0) as u64),
            ] {
                data.set_queue_wait(queue.id, wait, consumer_index, RetryMode::Retry);

                match data.get_retry_type(queue.id) {
                    RetryType::Incremental => {
                        assert_eq!(data.get_queue_wait(queue.id, consumer_index), wait);
                    }
                    _ => {}
                }
            }

            data.set_queue_wait(queue.id, TEST_WAIT, consumer_index, RetryMode::Forced);

            assert_eq!(data.get_queue_wait(queue.id, consumer_index), TEST_WAIT);
        }
    }
}

#[tokio::test]
async fn retry_type() {
    let queues = vec![
        QueueConfig {
            id: 1,
            queue_name: "example".into(),
            consumer_name: "example".into(),
            command: "echo 1".into(),
            command_timeout: None,
            base64: false,
            start_hour: None,
            end_hour: None,
            count: 100,
            retry_wait: 120,
            retry_mode: "static".into(),
            enabled: true,
        },
        QueueConfig {
            id: 2,
            queue_name: "example2".into(),
            consumer_name: "example".into(),
            command: "echo 1".into(),
            command_timeout: None,
            base64: false,
            start_hour: None,
            end_hour: None,
            count: 100,
            retry_wait: 120,
            retry_mode: "ignored".into(),
            enabled: false,
        },
        QueueConfig {
            id: 3,
            queue_name: "example3".into(),
            consumer_name: "example".into(),
            command: "echo 1".into(),
            command_timeout: None,
            base64: true,
            start_hour: None,
            end_hour: None,
            count: 100,
            retry_wait: 120,
            retry_mode: "incremental".into(),
            enabled: true,
        },
    ];

    let data = Arc::new(RwLock::new(Queue::new(Box::new(File::new(queues.clone())))));

    let mut data = data.write().await;
    for queue in queues {
        match data.get_retry_type(queue.id) {
            RetryType::Ignored => {
                assert_eq!(queue.retry_mode, "ignored");
            }
            RetryType::Incremental => {
                assert_eq!(queue.retry_mode, "incremental");
            }
            RetryType::Static => {
                assert_eq!(queue.retry_mode, "static");
            }
        }
    }
}
