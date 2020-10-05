# RabbitMQ consumer
[![Build Status](https://travis-ci.org/facile-it/rabbitmq-consumer.svg?branch=master)](https://travis-ci.org/facile-it/rabbitmq-consumer)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A configurable RabbitMQ consumer made in Rust, useful for a stable and reliable CLI commands processor.

## Installation

You can either compile it yourself, or download a precompiled binary from [here](https://github.com/facile-it/rabbitmq-consumer/releases).

### Compiling

You need the Rust environment, and you should be familiar with the language, in order to build a binary.

#### Installing Rust

In order to install the Rust toolchain with  `rustc` (compiler), `rustup` and `Cargo`, execute this command:  
`curl https://sh.rustup.rs -sSf | sh`

#### Introducing Cargo
Cargo is a dependency manager for Rust. You can create new projects, require new crates (libraries) or update existing crates.  

#### Building
Just run `cargo build` inside the project folder to build a debug binary.

You can even build a release binary (slower but optimized build) using `cargo build --release`

NB: You need the `libmysqlclient-dev` package (for Ubuntu) or `mysql-client` (for macOS) in order to build and statically link the binary.

## Usage

Run without arguments to start with default configuration or with `--help` to show the help summary:

```
$ rabbitmq-consumer
rabbitmq-consumer 1.0.0

A configurable RabbitMQ consumer made in Rust, useful for a stable and reliable CLI commands processor.

USAGE:
    rabbitmq-consumer [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --env <env>      Environment for configuration file loading
    -p, --path <path>    Base config file path
```

## Process shutdown

The consumer handles the SIGTERM and SIGINT signals, so when a signal is received, the consumer will be gracefully canceled.

# Configuration 
The consumer loads the standard configuration file, located at `config/config.toml`, at runtime. Pass the `--env` parameter in order to load a custom configuration file, for example:

```
$ rabbitmq-consumer --env dev
```

This will load the file located at `config/config_dev.toml`, if exists, otherwise the consumer will fallback to the default `config/config.toml` file.

You can also specify the configuration file base path with the `--path` option.

## config.toml
### [rabbit] section
This section describes the connection to the AMQP server.

> `host = "localhost"`
>> This defines the host domain or IP of the AMQP server: if the consumer fails to connect to this host, it will throw an error, and retry automatically.

> `port = 5672`
>> This is the port of the AMQP server.
  
> `username = "guest"`
>> This is the username of the AMQP server.
 
> `password = "guest"`
>> This is the password of the AMQP server.

> `vhost = "/"`
>> This is the vhost to use for the current user and session.

> `queue_prefix = "queue_"`
>> This is the internal prefix that the application will use to configure queues.

> `reconnections = 0`
>> By default the consumer will try to reconnect to AMQP server automatically and indefinitely (default 0 value), change this value to limit reconnection retries.

### [[rabbit.queues]] section
This section (a TOML array) defines all queues and consumers.

> `id = 1`
>> The virtual ID of this queue: must be different for each queue.

> `queue_name = "example"`
>> The internal name of the queue, also used in combination with the prefix to generate the channel and queue name.

> `consumer_name = "example"`
>> The internal name of the consumer: "_consumer_NUMBER" string will be attached after the name.

> `command = "php ../../bin/console example:command"`
>> The command executed for each received message; for example if the message contains `--id 1234`, the final executed command will be:
`php ../../bin/console example:command --id 1234`, you can attach any parameter using the content of the message, or pass a base64 encoded string for serialized data.

> `command_timeout = 30`
>> If specified, the command will be executed with a custom timeout: default is 30 (value is in minutes).

> `base64 = false`
>> If enabled, the consumer will send a base64 encoded message with a single "--body" parameter with the encoded data.

> `start_hour = "00:00:00"`
>> The start hour of consumer activity.

> `end_hour = "23:59:59"`
>> The end hour of consumer activity: the consumer will be stopped automatically until the start hour is reached again.

> `count = 1`
>> Specify how many consumers to run for this queue. WARNING: use multiple counts only on a shared instance, for a container based utilization, is recommended to split the load between replicas or more containers. 

> `retry_wait = 120`
>> The waiting time for the retry mode: default is 120 (value is in seconds).

> `retry_mode = "incremental"`
>> The retry mode can be "incremental", "static" or "ignored": in "incremental" the waiting time is multiplied each time by 2; in "static" the waiting time is fixed; in "ignored" no retry will be attempted.

> `enabled = true`
>> Enable or disable the queue.

### [database] section
This section defines the MySQL configuration.

> `enabled = true`
>> Enable or disable MySQL connection: if disabled, the "Static configuration" will be used.

> `host = "localhost"`
>> The MySQL server host.

> `port = 3306`
>> The MySQL server port.

> `user = "user"`
>> The MySQL server user.

> `password = "password"`
>> The MySQL server password.

> `db_name = "database"`
>> The MySQL server database name to use.

> `retries = 3`
>> Specify how many retries of MySQL if connection is lost before ending the process

## Environment variables
You can use environment variables for every configuration of the TOML file just using the `"$VARIABLE_NAME"` syntax as parameter value.
It's mandatory to use quotes in order to avoid spaced parameters and other issues.

## Queues and consumers
There are two ways for configuring queues:

* Database configuration (using MySQL integration)
* Static configuration

In __Database configuration__, you can enable or disable a queue at runtime, change its operation time and consumers count.

You can enable the Database configuration by switching the `enabled` option to `true` in the `[database]` section.

WARNING: If you use the MySQL connection, the consumer can fails more frequently due to the dependency of the MySQL server connection.

When a database configuration is enabled, the consumer will fetch a table named `queues` and will load all the configuration values from there. This is the query for creating the needed table:

```sql
CREATE TABLE queues
(
  id              INT AUTO_INCREMENT
    PRIMARY KEY,
  queue_name      VARCHAR(255)                      NOT NULL,
  consumer_name   VARCHAR(255)                      NOT NULL,
  command         VARCHAR(250)                      NOT NULL,
  command_timeout BIGINT UNSIGNED                   NULL,
  base64          TINYINT(1) DEFAULT 0              NOT NULL,
  start_hour      TIME                              NOT NULL,
  end_hour        TIME                              NOT NULL,
  count           INT(11) DEFAULT 1                 NOT NULL,
  retry_wait      BIGINT UNSIGNED DEFAULT 120       NOT NULL,
  retry_mode      VARCHAR(50) DEFAULT 'incremental' NOT NULL,
  enabled         TINYINT(1)                        NOT NULL
)
  ENGINE = InnoDB;
```

You can skip this if you don't care about changing the values at runtime. Just set the `enabled` flag to `false` on the database section and use the __Static configuration__ as follows:

```toml
[[rabbit.queues]]
    id = 1
    queue_name = "example"
    consumer_name = "example"
    command = "command_to_execute"
    command_timeout = 30
    base64 = false
    start_hour = "00:00:00"
    end_hour = "23:59:59"
    count = 1
    retry_wait = 120
    retry_mode = "incremental"
    enabled = true

[[rabbit.queues]]
    id = 2
    queue_name = "example2"
    consumer_name = "example2"
    command = "command_to_execute2"
    command_timeout = 15
    base64 = false
    start_hour = "00:00:00"
    end_hour = "23:59:59"
    count = 1
    retry_wait = 120
    retry_mode = "incremental"
    enabled = true

[[rabbit.queues]]
    id = 3
    queue_name = "example3"
    consumer_name = "example3"
    command = "command_to_execute3"
    command_timeout = 20
    base64 = false
    start_hour = "00:00:00"
    end_hour = "23:59:59"
    count = 1
    retry_wait = 120
    retry_mode = "incremental"
    enabled = true
```

You can set as many consumer as you need: these consumers will run on the same process asynchronously using one thread.

## Retry logic with exit codes
The consumer will handle specific OS process exit codes in order to apply a retry logic. This only applies with "incremental" or "static" retry modes.

**List of exit codes**

| Exit Code | Action                                |
|:---------:|---------------------------------------|
| 0         | Acknowledgement                       |
| 1         | Negative acknowledgement and re-queue |
| 2         | Negative acknowledgement              |

# Thanks to
* [Tokio](https://github.com/tokio-rs/tokio)
* [Lapin](https://github.com/CleverCloud/lapin)
* [Diesel](https://github.com/diesel-rs/diesel)
* [TOML](https://github.com/alexcrichton/toml-rs)
* [Chrono](https://github.com/chronotope/chrono)
* [Base64](https://github.com/marshallpierce/rust-base64)
