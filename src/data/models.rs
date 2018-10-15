use chrono::NaiveTime;

#[derive(Queryable, Deserialize, Debug, Clone)]
pub struct QueueSetting {
    pub id: i32,
    pub queue_name: String,
    pub consumer_name: String,
    pub command: String,
    pub command_timeout: Option<u64>,
    pub base64: bool,
    pub start_hour: Option<NaiveTime>,
    pub end_hour: Option<NaiveTime>,
    pub count: i32,
    pub retry_wait: u64,
    pub retry_mode: String,
    pub enabled: bool,
}
