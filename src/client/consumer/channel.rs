use async_std::sync::Arc;

use lapin::options::{BasicQosOptions, QueueDeclareOptions};
use lapin::{types::FieldTable, Channel as LapinChannel, Connection, Error as LapinError, Queue};

use crate::config::queue::config::QueueConfig;
use crate::logger;

#[derive(Debug)]
pub enum ChannelError {
    LapinError(LapinError),
}

pub struct Channel {}

impl Channel {
    pub async fn get_queue<S: AsRef<str>>(
        connection: Arc<Connection>,
        queue: QueueConfig,
        prefix: S,
    ) -> Result<(LapinChannel, Queue), ChannelError> {
        let channel = connection
            .create_channel()
            .await
            .map_err(ChannelError::LapinError)?;
        channel
            .basic_qos(
                1,
                BasicQosOptions {
                    ..Default::default()
                },
            )
            .await
            .map_err(ChannelError::LapinError)?;

        logger::log(format!(
            "[{}] Created channel with id: {}",
            queue.queue_name,
            channel.id()
        ));

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
            .await
            .map_err(ChannelError::LapinError)?;

        Ok((channel, queue))
    }
}
