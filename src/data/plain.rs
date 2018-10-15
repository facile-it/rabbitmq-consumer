use data::models::QueueSetting;
use data::Data;

pub struct Plain {
    queues: Vec<QueueSetting>,
}

impl Plain {
    pub fn new(queues: Vec<QueueSetting>) -> Self {
        Plain { queues }
    }
}

impl Data for Plain {
    fn get_queues(&mut self) -> Vec<QueueSetting> {
        self.queues.clone()
    }

    fn get_queue(&mut self, id: i32) -> Option<QueueSetting> {
        if let Some(index) = self.queues.iter().position(|ref rs| rs.id == id) {
            Some(self.queues[index].clone())
        } else {
            None
        }
    }
}
