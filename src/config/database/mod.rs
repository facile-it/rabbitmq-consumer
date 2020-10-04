mod schema;

use diesel::mysql::MysqlConnection;
use diesel::prelude::*;

use crate::config::database::schema::queues;
use crate::config::queue::config::QueueConfig;
use crate::config::queue::model::QueueModel;
use crate::config::DatabaseConfig;

pub struct Database {
    pub connection: MysqlConnection,
    config: DatabaseConfig,
}

impl Database {
    const DEFAULT_PORT: i32 = 3306;
    const DEFAULT_RETRIES: i32 = 3;

    pub fn new(config: DatabaseConfig) -> Self {
        Database {
            connection: Database::connection(config.clone()),
            config,
        }
    }

    pub fn connection(config: DatabaseConfig) -> MysqlConnection {
        let database_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.user,
            config.password,
            config.host,
            config.port.unwrap_or(Self::DEFAULT_PORT),
            config.db_name
        );

        MysqlConnection::establish(&database_url).expect(&format!(
            "Error connecting to host {} with db name {}",
            config.host, config.db_name
        ))
    }

    pub fn reconnect(&mut self) {
        self.connection = Database::connection(self.config.clone());
    }
}

impl QueueModel for Database {
    fn get_queues(&mut self) -> Vec<QueueConfig> {
        for i in 1..self.config.retries.unwrap_or(Self::DEFAULT_RETRIES) {
            match queues::dsl::queues.load::<QueueConfig>(&self.connection) {
                Ok(rs) => return rs,
                Err(e) => {
                    if i == 1 {
                        self.reconnect();
                    } else {
                        panic!("Error checking Database: {:?}", e);
                    }
                }
            }
        }

        vec![]
    }

    fn get_queue(&mut self, id: i32) -> Option<QueueConfig> {
        for i in 1..self.config.retries.unwrap_or(Self::DEFAULT_RETRIES) {
            match queues::dsl::queues
                .filter(queues::dsl::id.eq(id))
                .limit(1)
                .get_result::<QueueConfig>(&self.connection)
            {
                Ok(c) => {
                    return Some(c);
                }
                Err(e) => {
                    if i == 1 {
                        self.reconnect();
                    } else {
                        panic!("Error checking Database: {:?}", e);
                    }
                }
            }
        }

        None
    }
}
