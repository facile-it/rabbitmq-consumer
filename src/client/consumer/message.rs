use lapin::message::Delivery;
use lapin::{Channel, Error as LapinError};

use crate::logger;

#[derive(Debug)]
pub enum MessageError {
    LapinError(LapinError),
}

pub struct Message {}

impl Message {
    pub async fn handle_message(channel: &Channel, delivery: Delivery) -> Result<(), MessageError> {
        // info!("received message: {:?}", delivery);
        // if let Ok((channel, delivery)) = delivery {
        //     channel
        //         .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
        //         .await
        //         .expect("basic_ack");
        // }

        Ok(())
    }
}
