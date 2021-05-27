pub mod channel;
pub mod connection;
mod message;

use async_std::sync::{Arc, RwLock};

use log::{error, info};

use tokio::signal::unix::{signal, SignalKind};
use tokio_stream::StreamExt;

use futures::future::select_all;
use futures::future::FutureExt;

use lapin::options::{BasicCancelOptions, BasicConsumeOptions, BasicRecoverOptions};
use lapin::{types::FieldTable, Channel as LapinChannel, Error as LapinError, Queue as LapinQueue};

use crate::client::consumer::channel::Channel;
use crate::client::consumer::connection::{Connection, ConnectionError};
use crate::client::consumer::message::{Message, MessageError};
use crate::client::executor::events::{Events, EventsHandler};
use crate::config::database::Database;
use crate::config::file::File;
use crate::config::queue::config::QueueConfig;
use crate::config::queue::Queue;
use crate::config::Config;
use crate::utils;

const CONSUMER_WAIT: u64 = 60000;
const DEFAULT_WAIT_PART: u64 = 1000;

#[derive(Debug, PartialEq)]
pub enum ConsumerStatus {
    ConsumerChanged,
    CountChanged,
    GenericOk,
    Killed,
}

#[derive(Debug)]
pub enum ConsumerError {
    LapinError(LapinError),
    IoError(std::io::Error),
    ConnectionError(ConnectionError),
    MessageError(MessageError),
}

type ConsumerResult<T> = Result<T, ConsumerError>;

pub struct Consumer {
    queue: Arc<RwLock<Queue>>,
    connection: Connection,
    message: Message,
    hooks: Vec<Arc<RwLock<dyn Events>>>,
    config: Config,
}

impl Consumer {
    pub fn new(config: Config) -> Self {
        let queue = Arc::new(RwLock::new(Queue::new({
            if config.database.enabled {
                Box::new(Database::new(config.database.clone()))
            } else {
                Box::new(File::new(config.rabbit.queues.clone()))
            }
        })));

        Self {
            queue: queue.clone(),
            connection: Connection::new(config.rabbit.clone()),
            message: Message::new(queue),
            hooks: Vec::new(),
            config,
        }
    }

    pub async fn run(&mut self) -> ConsumerResult<ConsumerStatus> {
        match self.connection.get_connection().await {
            Ok(connection) => {
                for hook in &self.hooks {
                    hook.write()
                        .await
                        .on_connect(&self.config.rabbit.host, self.config.rabbit.port);
                }

                let mut sigint = signal(SignalKind::interrupt()).map_err(ConsumerError::IoError)?;
                let sigint = sigint.recv().map(|_| Ok(ConsumerStatus::Killed));
                let mut sigquit = signal(SignalKind::quit()).map_err(ConsumerError::IoError)?;
                let sigquit = sigquit.recv().map(|_| Ok(ConsumerStatus::Killed));
                let mut sigterm =
                    signal(SignalKind::terminate()).map_err(ConsumerError::IoError)?;
                let sigterm = sigterm.recv().map(|_| Ok(ConsumerStatus::Killed));

                let mut futures = vec![sigint.boxed(), sigquit.boxed(), sigterm.boxed()];

                info!("Managing queues...");

                let queues = self.queue.write().await.get_queues();
                if queues.is_empty() {
                    panic!("Can't load consumers due to empty queues");
                }

                for queue in queues {
                    for index in 0..queue.count {
                        futures.push(
                            match Channel::get_queue(
                                connection.clone(),
                                queue.clone(),
                                self.config.rabbit.queue_prefix.clone(),
                            )
                            .await
                            {
                                Ok((c, q)) => {
                                    info!("[{}] Queue created", queue.queue_name);

                                    self.consume(index, queue.clone(), c, q).boxed()
                                }
                                Err(e) => async { Err(ConsumerError::LapinError(e)) }.boxed(),
                            },
                        );
                    }
                }

                let (res, _, _) = select_all(futures).await;

                res
            }
            Err(e) => Err(ConsumerError::ConnectionError(e)),
        }
    }

    pub async fn consume(
        &self,
        index: i32,
        queue_config: QueueConfig,
        channel: LapinChannel,
        queue: LapinQueue,
    ) -> ConsumerResult<ConsumerStatus> {
        let consumer_name = format!("{}_consumer_{}", queue_config.consumer_name, index);

        if !self.queue.write().await.is_enabled(queue_config.id) {
            info!(
                "[{}] Consumer #{} with \"{}\" not enabled, waiting...",
                queue_config.queue_name, index, consumer_name
            );
        }

        self.check_consumer(&queue_config).await;

        let consumer = channel
            .basic_consume(
                queue.name().as_str(),
                &consumer_name,
                BasicConsumeOptions {
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await;

        match consumer {
            Ok(mut consumer) => {
                info!(
                    "[{}] Consumer #{} declared \"{}\"",
                    queue_config.queue_name, index, consumer_name
                );

                while let Some(delivery) = consumer.next().await {
                    match delivery {
                        Ok((channel, delivery)) => {
                            let is_changed = self
                                .queue
                                .write()
                                .await
                                .is_changed(queue_config.id, queue_config.count);
                            let is_enabled = self.queue.write().await.is_enabled(queue_config.id);

                            if !is_changed && is_enabled {
                                self.message
                                    .handle_message(index, &queue_config, &channel, delivery)
                                    .await
                                    .map_err(ConsumerError::MessageError)?;
                            }

                            if !is_enabled {
                                if !is_changed {
                                    if channel
                                        .basic_cancel(
                                            &consumer_name,
                                            BasicCancelOptions { nowait: false },
                                        )
                                        .await
                                        .is_err()
                                    {
                                        error!(
                                            "[{}] Error canceling the consumer #{}, returning...",
                                            queue_config.queue_name, index
                                        );
                                    } else {
                                        utils::wait(DEFAULT_WAIT_PART).await;
                                    }
                                }

                                if channel
                                    .basic_recover(BasicRecoverOptions { requeue: true })
                                    .await
                                    .is_err()
                                {
                                    error!(
                                            "[{}] Error recovering message for consumer #{}, message is not ackable...",
                                            queue_config.queue_name,
                                            index
                                        );

                                    return Ok(ConsumerStatus::GenericOk);
                                }

                                info!(
                                        "[{}] Consumer #{} not active, messages recovered and consumer canceled...",
                                        queue_config.queue_name,
                                        index
                                    );

                                return Ok(ConsumerStatus::ConsumerChanged);
                            } else if is_changed {
                                info!(
                                    "[{}] Consumers count changed, messages recovered...",
                                    queue_config.queue_name
                                );

                                return Ok(ConsumerStatus::CountChanged);
                            }
                        }
                        Err(e) => {
                            error!("[{}] Error getting messages.", queue_config.queue_name);

                            return Err(ConsumerError::LapinError(e));
                        }
                    }
                }

                info!("Messages have been processed.");

                Ok(ConsumerStatus::GenericOk)
            }
            Err(e) => Err(ConsumerError::LapinError(e)),
        }
    }

    async fn check_consumer(&self, queue_config: &QueueConfig) {
        while async {
            if !self.queue.write().await.is_enabled(queue_config.id) {
                utils::wait(CONSUMER_WAIT).await;

                Some(())
            } else {
                None
            }
        }
        .await
        .is_some()
        {}
    }
}

impl EventsHandler for Consumer {
    fn add_events_hook<E: Events + 'static>(mut self, hook: Arc<RwLock<E>>) -> Self {
        self.hooks.push(hook);

        self
    }
}
