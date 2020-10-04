use crate::config::queue::config::QueueConfig;
use crate::config::queue::model::QueueModel;

pub struct File {
    queues: Vec<QueueConfig>,
}

impl File {
    pub fn new(queues: Vec<QueueConfig>) -> Self {
        File { queues }
    }
}

impl QueueModel for File {
    fn get_queues(&mut self) -> Vec<QueueConfig> {
        self.queues.clone()
    }

    fn get_queue(&mut self, id: i32) -> Option<QueueConfig> {
        if let Some(index) = self.queues.iter().position(|ref rs| rs.id == id) {
            Some(self.queues[index].clone())
        } else {
            None
        }
    }
}
