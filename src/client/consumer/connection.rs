use async_std::sync::Arc;

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

pub struct Connection {
    config: RabbitConfig,
    lapin: Result<Arc<LapinConnection>, ConnectionError>,
}

impl Connection {
    pub fn new(config: RabbitConfig) -> Self {
        Self {
            config,
            lapin: Err(ConnectionError::NotConnected),
        }
    }

    pub async fn get_connection(&mut self) -> Result<Arc<LapinConnection>, ConnectionError> {
        if self.lapin.is_err() {
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
