use async_std::sync::{Arc, RwLock};

use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use lapin::Connection as LapinConnection;

use crate::client::consumer::channel::Channel;
use crate::client::consumer::connection::{Connection, ConnectionError};
use crate::client::consumer::{Consumer, ConsumerResult};
use crate::config::file::File;
use crate::config::queue::config::QueueConfig;
use crate::config::queue::Queue;
use crate::config::{Config, DatabaseConfig, RabbitConfig};

fn get_queues() -> Vec<QueueConfig> {
    vec![
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
    ]
}

fn get_cfg() -> Config {
    Config {
        rabbit: RabbitConfig {
            host: "127.0.0.1".into(),
            port: 5672,
            username: "guest".into(),
            password: "guest".into(),
            vhost: "/".into(),
            queues: get_queues(),
            queue_prefix: "sample_".into(),
            reconnections: Some(0),
        },
        database: DatabaseConfig {
            enabled: false,
            host: "".into(),
            port: None,
            user: "".into(),
            password: "".into(),
            db_name: "".into(),
            retries: None,
        },
    }
}

fn create_config() -> Config {
    let config = get_cfg();

    assert_eq!(config.database.enabled, false);

    config
}

async fn create_data() -> Arc<RwLock<Queue>> {
    let config = create_config();
    let data = Arc::new(RwLock::new(Queue::new({
        Box::new(File::new(config.rabbit.queues.clone()))
    })));

    assert_eq!(data.write().await.get_queues().is_empty(), false);

    data
}

async fn connect(config: Config) -> Result<Arc<LapinConnection>, ConnectionError> {
    Connection::new(config.rabbit).get_connection().await
}

#[tokio::test]
async fn connection() {
    let connection = connect(create_config()).await;

    assert!(connection.is_ok());
}

#[tokio::test]
async fn channel_queue() {
    let config = create_config();
    let data = create_data().await;

    let connection = connect(create_config()).await.unwrap();

    let result = Channel::get_queue(
        connection,
        data.write().await.get_queues().get(0).unwrap().to_owned(),
        config.rabbit.queue_prefix.clone(),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn consumer_changed() {
    let config = create_config();
    let data = create_data().await;
    let mut queue_config: QueueConfig = data.write().await.get_queues().get(0).unwrap().to_owned();

    let connection = connect(create_config()).await.unwrap();

    let (channel, queue) = Channel::get_queue(
        connection,
        queue_config.clone(),
        config.rabbit.queue_prefix.clone(),
    )
    .await
    .unwrap();

    assert!(channel
        .basic_publish(
            "",
            &format!("{}{}", config.rabbit.queue_prefix, queue_config.queue_name),
            BasicPublishOptions::default(),
            b"This is a test!".to_vec(),
            BasicProperties::default()
        )
        .await
        .is_ok());

    queue_config.count = 10;

    assert!(data
        .write()
        .await
        .is_changed(queue_config.id, queue_config.count));

    let consumer = Consumer::new(config);

    let result = consumer.consume(0, queue_config, channel, queue).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConsumerResult::CountChanged);
}
