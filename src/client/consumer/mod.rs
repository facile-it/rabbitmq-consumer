mod channel;
mod connection;
mod message;

use async_std::sync::{Arc, RwLock};

use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{delay_for, Duration};

use futures::future::select_all;
use futures::future::FutureExt;

use lapin::options::BasicConsumeOptions;
use lapin::{types::FieldTable, Channel as LapinChannel, Error as LapinError, Queue as LapinQueue};

use crate::client::consumer::channel::{Channel, ChannelError};
use crate::client::consumer::connection::{Connection, ConnectionError};
use crate::client::consumer::message::{Message, MessageError};
use crate::client::executor::events::{Events, EventsHandler};
use crate::config::database::Database;
use crate::config::file::File;
use crate::config::queue::config::QueueConfig;
use crate::config::queue::Queue;
use crate::config::Config;
use crate::logger;
use diesel::QueryDsl;
use tokio::stream::StreamExt;

const CONSUMER_WAIT: u64 = 60000;
const DEFAULT_WAIT_PART: u64 = 1000;

#[derive(Debug, PartialEq)]
pub enum ConsumerResult {
    CountChanged,
    GenericOk,
    Killed,
}

#[derive(Debug)]
pub enum ConsumerError {
    GenericError,
    ConsumerChanged,
    LapinError(LapinError),
    IoError(std::io::Error),
    ConnectionError(ConnectionError),
    ChannelError(ChannelError),
    MessageError(MessageError),
}

pub struct Consumer {
    queue: Arc<RwLock<Queue>>,
    connection: Connection,
    hooks: Vec<Arc<RwLock<dyn Events>>>,
    config: Config,
}

impl Consumer {
    pub fn new(config: Config) -> Self {
        logger::log(&format!(
            "Connection to {}:{}",
            config.rabbit.host, config.rabbit.port
        ));

        Self {
            queue: Arc::new(RwLock::new(Queue::new({
                if config.database.enabled {
                    Box::new(Database::new(config.database.clone()))
                } else {
                    Box::new(File::new(config.rabbit.queues.clone()))
                }
            }))),
            connection: Connection::new(config.rabbit.clone()),
            hooks: Vec::new(),
            config,
        }
    }

    pub async fn run(&mut self) -> Result<ConsumerResult, ConsumerError> {
        match self.connection.get_connection().await {
            Ok(connection) => {
                for hook in &self.hooks {
                    hook.write()
                        .await
                        .on_connect(&self.config.rabbit.host, self.config.rabbit.port);
                }

                let mut sigint =
                    signal(SignalKind::interrupt()).map_err(|e| ConsumerError::IoError(e))?;
                let sigint = sigint.recv().map(|_| Ok(ConsumerResult::Killed));
                let mut sigquit =
                    signal(SignalKind::quit()).map_err(|e| ConsumerError::IoError(e))?;
                let sigquit = sigquit.recv().map(|_| Ok(ConsumerResult::Killed));
                let mut sigterm =
                    signal(SignalKind::terminate()).map_err(|e| ConsumerError::IoError(e))?;
                let sigterm = sigterm.recv().map(|_| Ok(ConsumerResult::Killed));

                let mut futures = Vec::new();
                futures.push(sigint.boxed());
                futures.push(sigquit.boxed());
                futures.push(sigterm.boxed());

                logger::log("Managing queues...");

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
                                    logger::log(format!("[{}] Queue created", queue.queue_name));

                                    self.consume(index, queue.clone(), c, q).boxed()
                                }
                                Err(e) => async { Err(ConsumerError::ChannelError(e)) }.boxed(),
                            },
                        );
                    }
                }

                let (res, _, _) = select_all(futures).await;

                res
            }
            Err(e) => {
                for hook in &self.hooks {
                    hook.write().await.on_error(&format!("{:?}", e));
                }

                Err(ConsumerError::ConnectionError(e))
            }
        }
    }

    pub async fn consume(
        &self,
        index: i32,
        queue_config: QueueConfig,
        channel: LapinChannel,
        queue: LapinQueue,
    ) -> Result<ConsumerResult, ConsumerError> {
        let consumer_name = format!("{}_consumer_{}", queue_config.consumer_name, index);

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
                logger::log(format!(
                    "[{}] Consumer #{} declared \"{}\"",
                    queue_config.queue_name, index, consumer_name
                ));

                while let Some(delivery) = (consumer.next()).await {
                    match delivery {
                        Ok((channel, delivery)) => {
                            let is_changed = self
                                .queue
                                .write()
                                .await
                                .is_changed(queue_config.id, queue_config.count);
                            let is_enabled = self.queue.write().await.is_enabled(queue_config.id);

                            if !is_changed && is_enabled {
                                Message::handle_message(channel, delivery)
                                    .await
                                    .map_err(|e| ConsumerError::MessageError(e))?;
                            } else {
                                return Err(ConsumerError::ConsumerChanged);
                            }
                        }
                        Err(e) => {
                            return Err(ConsumerError::LapinError(e));
                        }
                    }
                }

                Ok(ConsumerResult::GenericOk)
            }
            Err(e) => Err(ConsumerError::LapinError(e)),
        }
    }

    async fn check_consumer(&self, queue_config: &QueueConfig) {
        loop {
            let wait_ms = if self.queue.write().await.is_enabled(queue_config.id) {
                0
            } else {
                // Sleep 2 minutes (60000 milliseconds) each DB check
                CONSUMER_WAIT
            };

            Self::wait(wait_ms).await;

            if wait_ms == 0 {
                break;
            }
        }
    }

    async fn wait(millis: u64) {
        delay_for(Duration::from_millis(millis)).await
    }
}

impl EventsHandler for Consumer {
    fn add_events_hook<E: Events + 'static>(mut self, hook: Arc<RwLock<E>>) -> Self {
        self.hooks.push(hook);

        self
    }
}
