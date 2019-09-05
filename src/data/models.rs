use chrono::NaiveTime;

use crate::utils::bool_or_string;
use crate::utils::i32_or_string;
use crate::utils::u64_or_string;
use crate::utils::option_u64_or_string;

#[derive(Queryable, Deserialize, Debug, Clone)]
pub struct QueueSetting {
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
    #[serde(deserialize_with = "u64_or_string")]
    pub retry_wait: u64,
    pub retry_mode: String,
    #[serde(deserialize_with = "bool_or_string")]
    pub enabled: bool,
}
