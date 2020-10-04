mod channel;
mod connection;

use async_std::sync::{Arc, RwLock};

use tokio::signal;

use futures::future::select_all;
use futures::future::FutureExt;

use lapin::{Channel as LapinChannel, Queue as LapinQueue};

use crate::client::consumer::channel::{Channel, ChannelError};
use crate::client::consumer::connection::{Connection, ConnectionError};
use crate::client::executor::events::{Events, EventsHandler};
use crate::config::database::Database;
use crate::config::file::File;
use crate::config::queue::Queue;
use crate::config::Config;
use crate::logger;

#[derive(Debug, PartialEq)]
pub enum ConsumerResult {
    CountChanged,
    GenericOk,
}

#[derive(Debug)]
pub enum ConsumerError {
    GenericError,
    ConnectionError(ConnectionError),
    IoError(std::io::Error),
    ChannelError(ChannelError),
    QueuesError,
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

                let mut signint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .map_err(|e| ConsumerError::IoError(e))?;
                let sigint = signint.recv().map(|_| Ok(ConsumerResult::GenericOk));
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .map_err(|e| ConsumerError::IoError(e))?;
                let sigterm = sigterm.recv().map(|_| Ok(ConsumerResult::GenericOk));

                logger::log("Managing queues...");

                let queues = self.queue.write().await.get_queues();
                if queues.is_empty() {
                    panic!("Can't load consumers due to empty queues");
                }

                let mut futures = Vec::new();
                for queue in queues.into_iter() {
                    for index in 0..queue.count {
                        let future = match Channel::get_queue(
                            connection.clone(),
                            queue.clone(),
                            self.config.rabbit.queue_prefix.clone(),
                        )
                        .await
                        {
                            Ok((c, q)) => {
                                logger::log(format!("[{}] Queue created", queue.queue_name));

                                self.consume(index, c, q).await
                            }
                            Err(e) => Err(ConsumerError::ChannelError(e)),
                        };

                        futures.push(async { future }.boxed());
                    }
                }

                futures.push(sigint.boxed());
                futures.push(sigterm.boxed());

                let (res, _idx, _remaining_futures) = select_all(futures).await;

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
        channel: LapinChannel,
        queue: LapinQueue,
    ) -> Result<ConsumerResult, ConsumerError> {
        Ok(ConsumerResult::GenericOk)
    }

    // async fn check_consumer(
    //     data: Arc<RwLock<DatabasePlain>>,
    //     queue_setting: QueueSetting,
    // ) -> Result<()> {
    //     loop_fn((), move |_| {
    //         let wait_ms = if data.write().await.is_enabled(queue_setting.id) {
    //             0
    //         } else {
    //             // Sleep 2 minutes (60000 milliseconds) each DB check
    //             CONSUMER_WAIT
    //         };
    //
    //         Self::wait(wait_ms).and_then(move |_| {
    //             if wait_ms == 0 {
    //                 Ok(Loop::Break(()))
    //             } else {
    //                 Ok(Loop::Continue(()))
    //             }
    //         })
    //     })
    // }
    //
    // async fn wait(millis: u64) -> Result<()> {
    //     Delay::new(Instant::now().add(Duration::from_millis(millis)))
    //         .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    // }
}

impl EventsHandler for Consumer {
    fn add_events_hook<E: Events + 'static>(mut self, hook: Arc<RwLock<E>>) -> Self {
        self.hooks.push(hook);

        self
    }
}
