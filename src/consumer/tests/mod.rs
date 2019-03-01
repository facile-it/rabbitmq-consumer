use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use futures::Future;

use tokio::net::TcpStream;
use tokio::runtime::current_thread::Runtime;

use lapin::{
    channel::{BasicProperties, BasicPublishOptions},
    client::Client,
    client::Heartbeat,
};

use crate::consumer::{Consumer, ConsumerResult};
use crate::data::config::{Config, DatabaseConfig, RabbitConfig};
use crate::data::models::QueueSetting;
use crate::data::plain::Plain;
use crate::data::DatabasePlain;

fn get_queues() -> Vec<QueueSetting> {
    vec![
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
    ]
}

fn get_cfg() -> Config {
    Config {
        rabbit: RabbitConfig {
            host: "localhost".into(),
            port: 5672,
            username: "guest".into(),
            password: "guest".into(),
            vhost: "/".into(),
            queues: get_queues(),
            queue_prefix: "sample_".into(),
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

fn create_data() -> Rc<RefCell<DatabasePlain>> {
    let config = create_config();
    let data = Rc::new(RefCell::new(DatabasePlain::new({
        Box::new(Plain::new(config.rabbit.queues.clone()))
    })));

    assert_eq!(data.borrow_mut().get_queues().is_empty(), false);

    data
}

fn connect(
    config: Config,
) -> impl Future<
    Item = (
        Client<TcpStream>,
        Heartbeat<impl Future<Item = (), Error = io::Error> + Send + 'static>,
    ),
    Error = io::Error,
> + 'static {
    Consumer::connect(
        config.rabbit.host,
        config.rabbit.port,
        config.rabbit.username,
        config.rabbit.password,
        config.rabbit.vhost,
    )
}

#[test]
fn connection() {
    let future = connect(create_config());
    let result = Runtime::new().unwrap().block_on(future);

    assert!(result.is_ok());
}

#[test]
fn channel_queue() {
    let mut runtime = Runtime::new().unwrap();

    let config = create_config();
    let data = create_data();

    let result = runtime.block_on(connect(create_config())).unwrap();

    let future = Consumer::channel(
        result.0,
        data.borrow_mut().get_queues().get(0).unwrap().to_owned(),
        config.rabbit.queue_prefix,
    );
    let result = runtime.block_on(future);

    assert!(result.is_ok());
}

#[test]
fn consumer_changed() {
    let mut runtime = Runtime::new().unwrap();

    let config = create_config();
    let data = create_data();
    let mut queue_setting: QueueSetting = data.borrow_mut().get_queues().get(0).unwrap().to_owned();

    let result = runtime.block_on(connect(create_config())).unwrap();
    let client = result.0;
    let heartbeat = result.1;

    let result = runtime
        .block_on(Consumer::channel(
            client,
            queue_setting.clone(),
            config.rabbit.queue_prefix.clone(),
        ))
        .unwrap();

    assert!(runtime
        .block_on(result.0.basic_publish(
            "",
            &format!("{}{}", config.rabbit.queue_prefix, queue_setting.queue_name),
            b"This is a test!".to_vec(),
            BasicPublishOptions::default(),
            BasicProperties::default(),
        ))
        .is_ok());

    queue_setting.count = 10;

    assert!(data
        .borrow_mut()
        .is_changed(queue_setting.id, queue_setting.count));

    let future = Consumer::consumer(data, result.0, result.1, queue_setting, 0);
    let result = runtime.block_on(
        future
            .select(heartbeat.map(|_| ConsumerResult::GenericOk))
            .map(|r| r.0)
            .map_err(|t| t.0),
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConsumerResult::CountChanged);
}
