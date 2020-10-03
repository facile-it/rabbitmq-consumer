mod message;
#[cfg(test)]
mod tests;

use std::net::ToSocketAddrs;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_std::sync::RwLock;

use tokio::signal;

use futures::future::select_all;
use futures::future::FutureExt;
use futures::stream::StreamExt;

use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, uri::AMQPAuthority,
    uri::AMQPUri, uri::AMQPUserInfo, BasicProperties, Connection, ConnectionProperties, Error,
    Result,
};

use self::message::Message;
use crate::data::{
    config::Config, database::Database, models::QueueSetting, plain::Plain, DatabasePlain,
};
use crate::logger;

const CONSUMER_WAIT: u64 = 60000;
const DEFAULT_WAIT_PART: u64 = 1000;

#[allow(unused_variables)]
pub trait Events {
    fn on_connect(&mut self, host: &str, port: i32) {}
    fn on_error(&mut self, error: &str) {}
}

#[derive(Debug, PartialEq)]
pub enum ConsumerResult {
    CountChanged,
    GenericOk,
}

pub struct Consumer {
    config: Config,
    data: Arc<RwLock<DatabasePlain>>,
    hooks: Arc<RwLock<Vec<dyn Events>>>,
}

impl Consumer {
    pub fn new(config: Config) -> Self {
        Consumer {
            data: Arc::new(RwLock::new(DatabasePlain::new({
                if config.database.enabled {
                    Box::new(Database::new(config.database.clone()))
                } else {
                    Box::new(Plain::new(config.rabbit.queues.clone()))
                }
            }))),
            config,
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_events_hook<E: Events + 'static>(&mut self, hook: Arc<RwLock<E>>) {
        self.hooks.write().await.push(hook);
    }

    pub async fn run(&mut self) -> Result<ConsumerResult> {
        let data = self.data.clone();
        let queue_prefix = self.config.rabbit.queue_prefix.clone();

        match Self::connect(
            self.config.rabbit.host.clone(),
            self.config.rabbit.port,
            self.config.rabbit.username.clone(),
            self.config.rabbit.password.clone(),
            self.config.rabbit.vhost.clone(),
        )
        .await
        {
            Ok(connection) => {
                for hook in &self.hooks {
                    hook.borrow_mut()
                        .on_connect(&self.config.rabbit.host, self.config.rabbit.port);
                }

                Ok(Self::queues(data, client, heartbeat, queue_prefix).await)
            }
            Err(e) => Err(e),
        }
    }

    async fn connect<S: AsRef<str>>(
        host: S,
        port: u16,
        username: S,
        password: S,
        vhost: S,
    ) -> Result<Connection> {
        let addr = format!("{}:{}", host.as_ref(), port)
            .to_socket_addrs()
            .expect("Not valid address")
            .next()
            .expect("Error getting the right address");

        logger::log(&format!("Connection to: {:?}", addr));

        Connection::connect_uri(
            AMQPUri {
                scheme: Default::default(),
                authority: AMQPAuthority {
                    userinfo: AMQPUserInfo {
                        username: username.to_owned(),
                        password: password.to_owned(),
                    },
                    host: host.to_owned(),
                    port,
                },
                vhost: vhost.to_owned(),
                query: Default::default(),
            },
            ConnectionProperties::default(),
        )
        .await
    }

    async fn queues(
        data: Arc<RwLock<DatabasePlain>>,
        client: Client<TcpStream>,
        queue_prefix: String,
    ) -> ConsumerResult {
        let sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?.recv();
        let sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?.recv();

        logger::log("Managing queues...");

        let queues = data.write().await.get_queues();
        if queues.is_empty() {
            panic!("Can't load consumers due to empty queues");
        }

        let mut futures = Vec::new();
        for queue in queues {
            for index in 0..queue.count {
                let data2 = data.clone();
                let queue2 = queue.clone();

                futures.push(
                    Self::channel(client.clone(), queue.clone(), queue_prefix.clone())
                        .and_then(move |(c, q)| Self::consumer(data2, c, q, queue2, index)),
                );
            }

            logger::log(format!("[{}] Queue created", queue.queue_name));
        }

        select_all(futures)
            .map(|r| r.0)
            .map_err(|t| t.0)
            .select(sigterm.map(|_| ConsumerResult::GenericOk).map_err(|t| t.0))
            .map(|r| r.0)
            .map_err(|t| t.0)
            .select(sigint.map(|_| ConsumerResult::GenericOk).map_err(|t| t.0))
            .map(|r| r.0)
            .map_err(|t| t.0)
            .select(heartbeat.map(|_| ConsumerResult::GenericOk))
            .map(|r| r.0)
            .map_err(|t| t.0)
    }

    async fn channel(
        client: Client<TcpStream>,
        queue_setting: QueueSetting,
        prefix: String,
    ) -> impl Future<Item = (Channel<TcpStream>, Queue), Error = io::Error> + 'static {
        client.create_channel().and_then(move |channel| {
            channel
                .basic_qos(BasicQosOptions {
                    prefetch_count: 1,
                    ..Default::default()
                })
                .then(move |_| {
                    logger::log(format!(
                        "[{}] Created channel with id: {}",
                        queue_setting.queue_name, channel.id
                    ));

                    channel
                        .queue_declare(
                            &format!("{}{}", &prefix, queue_setting.queue_name),
                            QueueDeclareOptions {
                                durable: true,
                                auto_delete: false,
                                ..Default::default()
                            },
                            FieldTable::new(),
                        )
                        .map(|queue| (channel, queue))
                })
        })
    }

    async fn consumer(
        data: Arc<RwLock<DatabasePlain>>,
        channel: Channel<TcpStream>,
        queue: Queue,
        queue_setting: QueueSetting,
        consumer_index: i32,
    ) -> Result<ConsumerResult> {
        let data = data.clone();
        let data2 = data.clone();
        let channel = channel.clone();
        let channel2 = channel.clone();
        let queue = queue.clone();
        let queue_setting = queue_setting.clone();
        let queue_setting2 = queue_setting.clone();

        let consumer_name = format!(
            "{}_consumer_{}",
            queue_setting.consumer_name, consumer_index
        );

        Self::check_consumer(data.clone(), queue_setting.clone())
            .and_then(move |_| {
                let data = data.clone();
                let channel = channel.clone();
                let queue_setting = queue_setting.clone();

                channel
                    .basic_consume(
                        &queue,
                        &consumer_name,
                        BasicConsumeOptions {
                            ..Default::default()
                        },
                        FieldTable::new(),
                    )
                    .and_then(move |stream| {
                        logger::log(format!(
                            "[{}] Consumer #{} declared \"{}\"",
                            queue_setting.queue_name, consumer_index, consumer_name
                        ));

                        let data2 = data.clone();
                        let queue_setting2 = queue_setting.clone();

                        stream.take_while(move |_message| {
                            let is_changed = data2.borrow_mut().is_changed(
                                queue_setting2.id,
                                queue_setting2.count
                            );
                            let is_enabled = data2.borrow_mut().is_enabled(queue_setting2.id);

                            future::ok(!is_changed && is_enabled)
                        }).for_each(move |message| {
                            Box::new(
                                Message::handle_message(
                                    data.clone(),
                                    channel.clone(),
                                    queue_setting.clone(),
                                    consumer_index,
                                    message
                                )
                            ).then(|_| Ok(()))
                        })
                    })
            })
            .and_then(move |_| {
                let is_changed = data2.borrow_mut().is_changed(
                    queue_setting2.id,
                    queue_setting2.count
                );
                let is_enabled = data2.borrow_mut().is_enabled(queue_setting2.id);

                let queue_name = queue_setting2.queue_name.clone();

                Self::wait(
                    if is_changed {
                        0
                    } else if !is_enabled {
                        if let Ok(mut transport) = channel2.transport.try_lock() {
                            if let Err(_) = transport.conn.basic_cancel(
                                channel2.id,
                                format!(
                                    "{}_consumer_{}",
                                    queue_setting2.consumer_name,
                                    consumer_index
                                ),
                                false
                            ) {
                                logger::log(
                                    &format!(
                                        "[{}] Error canceling the consumer #{}, returning...",
                                        queue_setting2.queue_name,
                                        consumer_index
                                    )
                                );

                                0
                            } else {
                                DEFAULT_WAIT_PART
                            }
                        } else {
                            logger::log(
                                &format!(
                                    "[{}] Error locking transport for consumer #{}, returning...",
                                    queue_setting2.queue_name,
                                    consumer_index
                                )
                            );

                            0
                        }
                    } else {
                        0
                    }
                ).and_then(move |_| {
                    if !is_enabled {
                        match channel2.transport.try_lock() {
                            Ok(mut transport) => {
                                if let Err(_) = transport.conn.basic_recover(
                                    channel2.id,
                                    true
                                ) {
                                    logger::log(
                                        &format!(
                                            "[{}] Error recovering message for consumer #{}, message is not ackable...",
                                            queue_name,
                                            consumer_index
                                        )
                                    );

                                    return Ok(Loop::Break(ConsumerResult::GenericOk));
                                }
                            }
                            Err(_) => {
                                logger::log(
                                    &format!(
                                        "[{}] Error locking transport for consumer #{}, message is not ackable...",
                                        queue_name,
                                        consumer_index
                                    )
                                );

                                return Ok(Loop::Break(ConsumerResult::GenericOk));
                            }
                        }

                        logger::log(
                            &format!(
                                "[{}] Consumer #{} not active, messages recovered and consumer canceled...",
                                queue_name,
                                consumer_index
                            )
                        );

                        Ok(Loop::Continue(()))
                    } else if is_changed {
                        logger::log(
                            &format!(
                                "[{}] Consumers count changed, messages recovered...",
                                queue_name
                            )
                        );

                        Ok(Loop::Break(ConsumerResult::CountChanged))
                    } else {
                        logger::log("Messages has been processed.");

                        Ok(Loop::Break(ConsumerResult::GenericOk))
                    }
                })
            }).await
    }

    async fn check_consumer(
        data: Arc<RwLock<DatabasePlain>>,
        queue_setting: QueueSetting,
    ) -> Result<()> {
        loop_fn((), move |_| {
            let wait_ms = if data.write().await.is_enabled(queue_setting.id) {
                0
            } else {
                // Sleep 2 minutes (60000 milliseconds) each DB check
                CONSUMER_WAIT
            };

            Self::wait(wait_ms).and_then(move |_| {
                if wait_ms == 0 {
                    Ok(Loop::Break(()))
                } else {
                    Ok(Loop::Continue(()))
                }
            })
        })
    }

    async fn wait(millis: u64) -> Result<()> {
        Delay::new(Instant::now().add(Duration::from_millis(millis)))
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}
