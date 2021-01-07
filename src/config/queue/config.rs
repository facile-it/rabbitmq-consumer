use chrono::{self, NaiveTime};

use crate::utils::{
    bool_or_string, i32_or_string, option_i32_or_string, option_u64_or_string, u64_or_string,
};

#[derive(Queryable, Deserialize, Debug, Clone)]
pub struct QueueConfig {
    #[serde(deserialize_with = "i32_or_string")]
    pub id: i32,
    pub queue_name: String,
    pub consumer_name: String,
    pub command: String,
    #[serde(deserialize_with = "option_u64_or_string", default)]
    pub command_timeout: Option<u64>,
    #[serde(deserialize_with = "bool_or_string")]
    pub base64: bool,
    pub start_hour: Option<NaiveTime>,
    pub end_hour: Option<NaiveTime>,
    #[serde(deserialize_with = "i32_or_string")]
    pub count: i32,
    #[serde(deserialize_with = "option_i32_or_string", default)]
    pub nack_code: Option<i32>,
    #[serde(deserialize_with = "u64_or_string")]
    pub retry_wait: u64,
    pub retry_mode: String,
    #[serde(deserialize_with = "bool_or_string")]
    pub enabled: bool,
}
