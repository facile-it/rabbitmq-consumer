use async_std::sync::Arc;

use log::info;

use lapin::options::{BasicQosOptions, QueueDeclareOptions};
use lapin::{types::FieldTable, Channel as LapinChannel, Connection, Error as LapinError, Queue};

use crate::config::queue::config::QueueConfig;

type ChannelResult = Result<(LapinChannel, Queue), LapinError>;

pub struct Channel {}

impl Channel {
    pub async fn get_queue<S: AsRef<str>>(
        connection: Arc<Connection>,
        queue: QueueConfig,
        prefix: S,
    ) -> ChannelResult {
        let channel = connection.create_channel().await?;
        channel
            .basic_qos(
                queue.prefetch_count.unwrap_or(1) as u16,
                BasicQosOptions {
                    ..Default::default()
                },
            )
            .await?;

        info!(
            "[{}] Created channel with id: {}",
            queue.queue_name,
            channel.id()
        );

        let queue = channel
            .queue_declare(
                &format!("{}{}", prefix.as_ref(), queue.queue_name),
                QueueDeclareOptions {
                    durable: true,
                    auto_delete: false,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        Ok((channel, queue))
    }
}
