use crate::config::queue::config::QueueConfig;

pub trait QueueModel {
    fn get_queues(&mut self) -> Vec<QueueConfig>;
    fn get_queue(&mut self, id: i32) -> Option<QueueConfig>;

    fn get_command(&mut self, id: i32) -> String {
        match self.get_queue(id) {
            Some(queue) => queue.command,
            None => String::new(),
        }
    }

    fn get_command_timeout(&mut self, id: i32) -> u64 {
        (match self.get_queue(id) {
            Some(queue) => queue.command_timeout.unwrap_or(super::DEFAULT_TIMEOUT),
            None => super::DEFAULT_TIMEOUT,
        } * super::TIME_S_MULTIPLIER)
            * super::TIME_MS_MULTIPLIER
    }

    fn is_enabled(&mut self, id: i32) -> bool {
        match self.get_queue(id) {
            Some(queue) => {
                let now = Some(chrono::Local::now().naive_local().time());

                queue.enabled && {
                    if queue.start_hour.is_some() && queue.end_hour.is_some() {
                        queue.start_hour.le(&now) && queue.end_hour.ge(&now)
                    } else {
                        true
                    }
                }
            }
            None => false,
        }
    }

    fn is_changed(&mut self, id: i32, current_count: i32) -> bool {
        match self.get_queue(id) {
            Some(p) => p.count != current_count,
            None => false,
        }
    }
}
