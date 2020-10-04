use std::collections::VecDeque;
use std::io;
use std::process::Output;
use std::str;

use async_std::sync::{Arc, RwLock};

use tokio::process::Command;

use futures::future::select_all;
use futures::{FutureExt, TryFutureExt};

use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicRejectOptions};
use lapin::{Channel, Error as LapinError};

use base64::encode as base64_encode;

use crate::client::consumer::DEFAULT_WAIT_PART;
use crate::config::queue::config::QueueConfig;
use crate::config::queue::{Queue, RetryMode, RetryType};
use crate::logger;
use crate::utils;

#[derive(Debug)]
pub enum MessageError {
    LapinError(LapinError),
}

pub enum MessageResult {
    GenericOk,
}

pub enum CommandResult {
    Timeout,
    Output(io::Result<Output>),
}

pub struct Message {
    queue: Arc<RwLock<Queue>>,
}

const ACKNOWLEDGEMENT: i32 = 0;
const NEGATIVE_ACKNOWLEDGEMENT_AND_RE_QUEUE: i32 = 1;
const NEGATIVE_ACKNOWLEDGEMENT: i32 = 2;

impl Message {
    pub fn new(queue: Arc<RwLock<Queue>>) -> Self {
        Self { queue }
    }

    pub async fn handle_message(
        &self,
        index: i32,
        queue_config: &QueueConfig,
        channel: &Channel,
        delivery: Delivery,
    ) -> Result<MessageResult, MessageError> {
        let msg = {
            match str::from_utf8(&delivery.data) {
                Ok(msg) => {
                    if queue_config.base64 {
                        base64_encode(msg)
                    } else {
                        msg.to_string().replace("\"", "")
                    }
                }
                Err(_) => "".into(),
            }
        };

        let command = {
            let cmd = self.queue.write().await.get_command(queue_config.id);
            let mut arguments = cmd.split(' ').collect::<VecDeque<&str>>();
            let mut command = Command::new(arguments.pop_front().unwrap());
            let mut additional_arguments = VecDeque::new();
            if queue_config.base64 {
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
            queue_config.queue_name, index, command
        ));

        self.process_message(index, queue_config, msg, command, channel, delivery)
            .await
    }

    async fn process_message(
        &self,
        index: i32,
        queue_config: &QueueConfig,
        msg: String,
        mut command: Command,
        channel: &Channel,
        delivery: Delivery,
    ) -> Result<MessageResult, MessageError> {
        let timeout = utils::wait(
            self.queue
                .write()
                .await
                .get_command_timeout(queue_config.id),
        )
        .map(|_| CommandResult::Timeout);
        let output = command.output().map(|output| CommandResult::Output(output));

        let (res, _, _) = select_all(vec![timeout.boxed(), output.boxed()]).await;
        match res {
            CommandResult::Output(output) => {
                match output {
                    Ok(output) => match self.queue.write().await.get_retry_type(queue_config.id) {
                        RetryType::Ignored => {
                            logger::log(
                                &format!(
                                    "[{}] Command {} executed on consumer #{} and result ignored, message removed.",
                                    queue_config.queue_name,
                                    msg,
                                    index
                                )
                            );

                            channel
                                .basic_ack(
                                    delivery.delivery_tag,
                                    BasicAckOptions { multiple: false },
                                )
                                .map_err(|e| MessageError::LapinError(e))
                                .await?;
                        }
                        _ => match output.status.code().unwrap_or(NEGATIVE_ACKNOWLEDGEMENT) {
                            ACKNOWLEDGEMENT => {
                                logger::log(&format!(
                                    "[{}] Command {} succeeded on consumer #{}, message removed.",
                                    queue_config.queue_name, msg, index
                                ));

                                channel
                                    .basic_ack(
                                        delivery.delivery_tag,
                                        BasicAckOptions { multiple: false },
                                    )
                                    .map_err(|e| MessageError::LapinError(e))
                                    .await?;

                                self.queue.write().await.set_queue_wait(
                                    queue_config.id,
                                    queue_config.retry_wait,
                                    index,
                                    RetryMode::Normal,
                                );
                            }
                            NEGATIVE_ACKNOWLEDGEMENT_AND_RE_QUEUE => {
                                logger::log(
                                    &format!(
                                        "[{}] Command {} failed on consumer #{}, message rejected and requeued. Output:\n{:#?}",
                                        queue_config.queue_name,
                                        msg,
                                        index,
                                        output
                                    )
                                );

                                channel
                                    .basic_reject(
                                        delivery.delivery_tag,
                                        BasicRejectOptions { requeue: true },
                                    )
                                    .map_err(|e| MessageError::LapinError(e))
                                    .await?;

                                let ms = self
                                    .queue
                                    .write()
                                    .await
                                    .get_queue_wait(queue_config.id, index);

                                logger::log(&format!(
                                    "[{}] Waiting {} milliseconds for consumer #{}...",
                                    queue_config.queue_name, ms, index
                                ));

                                self.wait_db(index, queue_config).await;

                                self.queue.write().await.set_queue_wait(
                                    queue_config.id,
                                    ms,
                                    index,
                                    RetryMode::Retry,
                                );
                            }
                            _ => {
                                logger::log(
                                    &format!(
                                        "[{}] Command {} failed on consumer #{}, message rejected. Output:\n{:#?}",
                                        queue_config.queue_name,
                                        msg,
                                        index,
                                        output
                                    )
                                );

                                channel
                                    .basic_reject(
                                        delivery.delivery_tag,
                                        BasicRejectOptions { requeue: false },
                                    )
                                    .map_err(|e| MessageError::LapinError(e))
                                    .await?;
                            }
                        },
                    },
                    Err(e) => {
                        logger::log(&format!(
                            "[{}] Error {:?} executing the command ({:?}) on consumer #{}, message {:#?} rejected...",
                            e,
                            queue_config.queue_name,
                            msg,
                            index,
                            msg
                        ));

                        channel
                            .basic_reject(
                                delivery.delivery_tag,
                                BasicRejectOptions { requeue: false },
                            )
                            .map_err(|e| MessageError::LapinError(e))
                            .await?;
                    }
                }

                Ok(MessageResult::GenericOk)
            }
            CommandResult::Timeout => {
                logger::log(
                    &format!(
                        "[{}] Timeout occurred executing the command on consumer #{}, message {:#?} rejected and requeued...",
                        queue_config.queue_name,
                        index,
                        msg
                    )
                );

                channel
                    .basic_reject(delivery.delivery_tag, BasicRejectOptions { requeue: true })
                    .map_err(|e| MessageError::LapinError(e))
                    .await?;

                Ok(MessageResult::GenericOk)
            }
        }
    }

    async fn wait_db(&self, index: i32, queue_config: &QueueConfig) {
        while let Some(_) = async {
            if self.queue.write().await.is_enabled(queue_config.id) {
                let waiting = self
                    .queue
                    .write()
                    .await
                    .get_queue_wait(queue_config.id, index) as i64
                    - DEFAULT_WAIT_PART as i64;

                if waiting <= 0 {
                    self.queue.write().await.set_queue_wait(
                        queue_config.id,
                        queue_config.retry_wait,
                        index,
                        RetryMode::Normal,
                    );

                    None
                } else {
                    self.queue.write().await.set_queue_wait(
                        queue_config.id,
                        waiting as u64,
                        index,
                        RetryMode::Forced,
                    );

                    Some(utils::wait(DEFAULT_WAIT_PART).await)
                }
            } else {
                None
            }
        }
        .await
        {}
    }
}
