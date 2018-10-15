use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::ops::Add;
use std::process::Command;
use std::rc::Rc;
use std::str;
use std::time::{Duration, Instant};

use futures::future::{loop_fn, Either, Future, Loop};

use tokio::net::TcpStream;
use tokio_current_thread;
use tokio_process::{CommandExt, OutputAsync};
use tokio_timer::Delay;

use lapin::channel::Channel;
use lapin_async::message::Delivery;

use base64::encode as base64_encode;

use consumer::{Consumer, ConsumerResult, DEFAULT_WAIT_PART};
use data::{models::QueueSetting, DatabasePlain, RetryMode, RetryType};
use logger;

pub struct Message;

impl Message {
    pub fn handle_message(
        data: Rc<RefCell<DatabasePlain>>,
        channel: Channel<TcpStream>,
        queue_setting: QueueSetting,
        consumer_index: i32,
        message: Delivery,
    ) -> impl Future<Item = ConsumerResult, Error = io::Error> + 'static {
        let msg = {
            match str::from_utf8(&message.data) {
                Ok(msg) => {
                    if queue_setting.base64 {
                        base64_encode(msg)
                    } else {
                        msg.to_string().replace("\"", "")
                    }
                }
                Err(_) => "".to_string(),
            }
        };

        let mut command = {
            let mut cmd = data.borrow_mut().get_command(queue_setting.id);
            let mut arguments = cmd.split(' ').collect::<VecDeque<&str>>();
            let mut command = Command::new(arguments.pop_front().unwrap());
            let mut additional_arguments = VecDeque::new();
            if queue_setting.base64 {
                additional_arguments.push_back("--body");
                additional_arguments.push_back(msg.as_str());
            } else {
                additional_arguments.extend(msg.split(' ').collect::<VecDeque<&str>>());
            }

            arguments.append(&mut additional_arguments);
            command.args(arguments);

            command
        };

        logger::log(&format!(
            "[{}] Executing command on consumer #{}: {:?}",
            queue_setting.queue_name, consumer_index, command
        ));

        Self::process_message(
            data.clone(),
            msg.clone(),
            command.output_async(),
            channel.clone(),
            queue_setting.clone(),
            consumer_index,
            message.clone(),
        )
    }

    fn process_message(
        data: Rc<RefCell<DatabasePlain>>,
        msg: String,
        command: OutputAsync,
        channel: Channel<TcpStream>,
        queue_setting: QueueSetting,
        consumer_index: i32,
        message: Delivery,
    ) -> impl Future<Item = ConsumerResult, Error = io::Error> + 'static {
        let timeout = data.borrow_mut().get_command_timeout(queue_setting.id);

        let data = data.clone();
        let msg2 = msg.clone();
        let channel2 = channel.clone();
        let queue_setting2 = queue_setting.clone();
        let delivery_tag = message.delivery_tag.clone();

        command.select2(
            Delay::new(
                Instant::now().add(
                    Duration::from_millis(timeout)
                )
            )
        ).and_then(move |result| {
            let future: Box<Future<Item = _, Error = _>> = match result {
                Either::A((output, _timeout)) => {
                    let retry_type = data.borrow_mut().get_retry_type(queue_setting.id);
                    match retry_type {
                        RetryType::Ignored => {
                            logger::log(
                                &format!(
                                    "[{}] Command {} executed on consumer #{} and result ignored, message removed.",
                                    queue_setting.queue_name,
                                    msg,
                                    consumer_index
                                )
                            );

                            Box::new(
                                channel.basic_ack(
                                    message.delivery_tag,
                                    false
                                )
                            )
                        }
                        _ => {
                            match output.status.code().unwrap_or(2) {
                                0 => {
                                    logger::log(
                                        &format!(
                                            "[{}] Command {} succeeded on consumer #{}, message removed.",
                                            queue_setting.queue_name,
                                            msg,
                                            consumer_index
                                        )
                                    );

                                    Box::new(
                                        channel.basic_ack(
                                            message.delivery_tag,
                                            false
                                        ).and_then(move |_| {
                                            data.borrow_mut().set_queue_wait(
                                                queue_setting.id,
                                                queue_setting.retry_wait,
                                                consumer_index,
                                                RetryMode::Normal
                                            );

                                            Ok(())
                                        })
                                    )
                                }
                                1 => {
                                    logger::log(
                                        &format!(
                                            "[{}] Command {} failed on consumer #{}, message rejected and requeued. Output:\n{:#?}",
                                            queue_setting.queue_name,
                                            msg,
                                            consumer_index,
                                            output
                                        )
                                    );

                                    Box::new(
                                        channel.basic_reject(
                                            message.delivery_tag,
                                            true
                                        ).and_then(move |_| {
                                            let ms = data.borrow_mut().get_queue_wait(
                                                queue_setting.id,
                                                consumer_index
                                            );

                                            logger::log(
                                                &format!(
                                                    "[{}] Waiting {} milliseconds for consumer #{}...",
                                                    queue_setting.queue_name,
                                                    ms,
                                                    consumer_index
                                                )
                                            );

                                            Self::wait_db(
                                                data.clone(),
                                                queue_setting.clone(),
                                                consumer_index
                                            ).then(move |_| {
                                                data.borrow_mut().set_queue_wait(
                                                    queue_setting.id,
                                                    ms,
                                                    consumer_index,
                                                    RetryMode::Retry
                                                );

                                                Ok(())
                                            })
                                        })
                                    )
                                }
                                _ => {
                                    logger::log(
                                        &format!(
                                            "[{}] Command {} failed on consumer #{}, message rejected. Output:\n{:#?}",
                                            queue_setting.queue_name,
                                            msg,
                                            consumer_index,
                                            output
                                        )
                                    );

                                    Box::new(
                                        channel.basic_reject(
                                            message.delivery_tag,
                                            false
                                        )
                                    )
                                }
                            }
                        }
                    }
                }
                Either::B((_timeout_error, _get)) => {
                    logger::log(
                        &format!(
                            "[{}] Timeout occurred executing the command on consumer #{}, message {:#?} rejected and requeued...",
                            queue_setting.queue_name,
                            consumer_index,
                            msg
                        )
                    );

                    Box::new(
                        channel.basic_reject(
                            message.delivery_tag,
                            true
                        )
                    )
                }
            };

            future.then(|_| Ok(ConsumerResult::GenericOk))
        }).then(move |result| {
            match result {
                Ok(res) => Ok(res),
                Err(e) => {
                    logger::log(
                        &format!(
                            "[{}] Error executing the command ({:?}) on consumer #{}, message {:#?} rejected...",
                            queue_setting2.queue_name,
                            e,
                            consumer_index,
                            msg2
                        )
                    );

                    tokio_current_thread::spawn(
                        channel2.basic_reject(
                            delivery_tag,
                            false
                        ).then(|_| Ok(()))
                    );

                    Ok(ConsumerResult::GenericOk)
                }
            }
        })
    }

    fn wait_db(
        data: Rc<RefCell<DatabasePlain>>,
        queue_setting: QueueSetting,
        consumer_index: i32,
    ) -> impl Future<Item = (), Error = io::Error> + 'static {
        loop_fn((), move |_| {
            let wait_ms = if !data.borrow_mut().is_enabled(queue_setting.id) {
                0
            } else {
                let waiting = data
                    .borrow_mut()
                    .get_queue_wait(queue_setting.id, consumer_index)
                    as i64
                    - DEFAULT_WAIT_PART as i64;

                if waiting <= 0 {
                    data.borrow_mut().set_queue_wait(
                        queue_setting.id,
                        queue_setting.retry_wait,
                        consumer_index,
                        RetryMode::Normal,
                    );

                    0
                } else {
                    data.borrow_mut().set_queue_wait(
                        queue_setting.id,
                        waiting as u64,
                        consumer_index,
                        RetryMode::Forced,
                    );

                    DEFAULT_WAIT_PART
                }
            };

            Consumer::wait(wait_ms).and_then(move |_| {
                if wait_ms == 0 {
                    Ok(Loop::Break(()))
                } else {
                    Ok(Loop::Continue(()))
                }
            })
        })
    }
}
