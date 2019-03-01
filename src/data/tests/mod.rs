use std::cell::RefCell;
use std::net::ToSocketAddrs;
use std::panic;
use std::rc::Rc;

use crate::data;
use crate::data::{models::QueueSetting, plain::Plain, DatabasePlain, RetryMode, RetryType};

#[test]
fn file_read_empty() {
    let result = panic::catch_unwind(|| {
        data::config::config_loader(None, None);
    });

    assert!(result.is_ok());
}

#[test]
fn file_read_dev() {
    let result = panic::catch_unwind(|| {
        data::config::config_loader(Some("dev"), None);
    });

    assert!(result.is_ok());
}

#[test]
fn file_read_prod() {
    let result = panic::catch_unwind(|| {
        data::config::config_loader(Some("prod"), None);
    });

    assert!(result.is_ok());
}

#[test]
fn address() {
    let config = data::config::config_loader(None, None);
    let address = format!("{}:{}", config.rabbit.host, config.rabbit.port).to_socket_addrs();

    assert!(address.is_ok());
    assert!(address.unwrap().next().is_some());
}

#[test]
fn waits() {
    let config = data::config::config_loader(None, None);
    let data = Rc::new(RefCell::new(DatabasePlain::new({
        Box::new(Plain::new(config.rabbit.queues.clone()))
    })));

    const TEST_WAIT: u64 = 120;

    let mut data = data.borrow_mut();
    for queue in config.rabbit.queues {
        for consumer_index in 0..queue.count {
            assert_eq!(
                data.get_queue_wait(queue.id, consumer_index),
                queue.retry_wait * data::TIME_MS_MULTIPLIER
            );

            data.set_queue_wait(queue.id, TEST_WAIT, consumer_index, RetryMode::Normal);

            assert_eq!(
                data.get_queue_wait(queue.id, consumer_index),
                TEST_WAIT * data::TIME_MS_MULTIPLIER
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

#[test]
fn retry_type() {
    let queues = vec![
        QueueSetting {
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
        QueueSetting {
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
        QueueSetting {
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

    let data = Rc::new(RefCell::new(DatabasePlain::new({
        Box::new(Plain::new(queues.clone()))
    })));

    let mut data = data.borrow_mut();
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
