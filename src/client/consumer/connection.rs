use async_std::sync::Arc;

use log::info;

use lapin::{
    uri::AMQPAuthority, uri::AMQPUri, uri::AMQPUserInfo, Connection as LapinConnection,
    ConnectionProperties, Error,
};

use crate::config::RabbitConfig;

#[derive(Debug, Clone)]
pub enum ConnectionError {
    NotConnected,
    LapinError(Error),
}

type ConnectionResult = Result<Arc<LapinConnection>, ConnectionError>;

pub struct Connection {
    config: RabbitConfig,
    lapin: ConnectionResult,
}

impl Connection {
    pub fn new(config: RabbitConfig) -> Self {
        Self {
            config,
            lapin: Err(ConnectionError::NotConnected),
        }
    }

    pub async fn get_connection(&mut self) -> ConnectionResult {
        if let Ok(ref lapin) = self.lapin {
            if lapin.status().errored() {
                self.lapin = Err(ConnectionError::NotConnected);
            }
        }

        if self.lapin.is_err() {
            info!(
                "Connecting to RabbitMQ at {}:{}...",
                self.config.host, self.config.port
            );

            match LapinConnection::connect_uri(
                AMQPUri {
                    scheme: Default::default(),
                    authority: AMQPAuthority {
                        userinfo: AMQPUserInfo {
                            username: self.config.username.clone(),
                            password: self.config.password.clone(),
                        },
                        host: self.config.host.clone(),
                        port: self.config.port,
                    },
                    vhost: self.config.vhost.clone(),
                    query: Default::default(),
                },
                ConnectionProperties::default(),
            )
            .await
            {
                Ok(connection) => {
                    self.lapin = Ok(Arc::new(connection));
                }
                Err(e) => {
                    self.lapin = Err(ConnectionError::LapinError(e));
                }
            }
        }

        self.lapin.clone()
    }
}
