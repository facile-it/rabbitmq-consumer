use std::sync::Arc;

use async_std::sync::RwLock;

#[allow(unused_variables)]
pub trait Events: Send + Sync {
    fn on_connect(&mut self, host: &str, port: u16) {}
    fn on_error(&mut self, error: &str) {}
}

pub trait EventsHandler {
    fn add_events_hook<E: Events + 'static>(self, hook: Arc<RwLock<E>>) -> Self;
}
