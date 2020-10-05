use std::error::Error;

use clap::{App, Arg};

use rabbitmq_consumer_lib::client::{Client, ClientResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let name = "RabbitMQ Consumer";
    let description = "A configurable RabbitMQ consumer made in Rust, useful for a stable and reliable CLI commands processor.";

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .name(name)
        .author("Dario Cancelliere <dario.cancelliere@facile.it>")
        .about(description)
        .arg(
            Arg::with_name("env")
                .short("e")
                .long("env")
                .required(false)
                .takes_value(true)
                .default_value("local")
                .help("Environment for configuration file loading"),
        )
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .required(false)
                .takes_value(true)
                .default_value("config")
                .help("Base config file path"),
        )
        .get_matches();

    logger::log(format!(
        "{} v{} by Dario Cancelliere",
        name,
        env!("CARGO_PKG_VERSION")
    ));
    logger::log(description);
    logger::log("");

    match Client::new(
        matches.value_of("env").unwrap(),
        matches.value_of("path").unwrap(),
    )
    .run()
    .await
    {
        ClientResult::Ok => Ok(()),
        ClientResult::ConsumerError(_) => Ok(()),
    }
}
