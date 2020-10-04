use crate::client::executor::events::Events;

pub struct Waiter {
    connections: i32,
    pub waiting: u64,
    waiting_times: i32,
}

impl Waiter {
    const LOOP_WAIT: u64 = 500;

    pub fn new(connections: i32) -> Self {
        Waiter {
            connections,
            waiting: Self::LOOP_WAIT,
            waiting_times: 0,
        }
    }

    pub fn is_to_close(&self) -> bool {
        if self.connections > 0 {
            if self.waiting_times < self.connections {
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}

impl Events for Waiter {
    fn on_connect(&mut self, _host: &str, _port: u16) {
        self.waiting = Self::LOOP_WAIT;
        self.waiting_times = 0;
    }

    fn on_error(&mut self, _error: &str) {
        if self.connections > 0 {
            if self.waiting_times < self.connections {
                self.waiting_times += 1;
            }
        }

        if self.waiting < (u64::max_value() / 2) {
            self.waiting *= 2;
        }
    }
}
