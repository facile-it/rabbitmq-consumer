use diesel::mysql::MysqlConnection;
use diesel::prelude::*;

use crate::data::models::QueueSetting;
use crate::data::schema::queues;
use crate::data::{config::DatabaseConfig, Data};

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

        MysqlConnection::establish(&database_url)
            .expect(&format!("Error connecting to {}", database_url))
    }

    pub fn reconnect(&mut self) {
        self.connection = Database::connection(self.config.clone());
    }
}

impl Data for Database {
    fn get_queues(&mut self) -> Vec<QueueSetting> {
        for i in 1..self.config.retries.unwrap_or(Self::DEFAULT_RETRIES) {
            match queues::dsl::queues.load::<QueueSetting>(&self.connection) {
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

    fn get_queue(&mut self, id: i32) -> Option<QueueSetting> {
        for i in 1..self.config.retries.unwrap_or(Self::DEFAULT_RETRIES) {
            match queues::dsl::queues
                .filter(queues::dsl::id.eq(id))
                .limit(1)
                .get_result::<QueueSetting>(&self.connection)
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
