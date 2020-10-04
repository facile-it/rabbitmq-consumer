use std::error::Error;

use crate::config::Config;

pub enum ClientResult {
    Ok(()),
    Error(Box<dyn Error>),
}

pub struct Client {
    config: Config,
}

impl Client {
    pub fn new<S: AsRef<str>>(environment: S, path: S) -> Self {
        Client {
            config: Config::new(environment, path),
        }
    }

    pub async fn run(&self) -> ClientResult {
        // let mut executor = Executor::new(data::config::config_loader(
        //     matches.value_of("env"),
        //     matches.value_of("path"),
        // ))
        // .await;
        //
        // loop {
        //     match executor.execute().await {
        //         ExecutorResult::Restart => logger::log("Consumer count changed, restarting..."),
        //         ExecutorResult::Wait(error, waiting) => {
        //             logger::log(&format!(
        //                 "Error ({:?}), waiting {} seconds...",
        //                 error,
        //                 waiting / 1000
        //             ));
        //
        //             thread::sleep(Duration::from_millis(waiting));
        //         }
        //         ExecutorResult::Exit => {
        //             logger::log("Process finished, exiting...");
        //
        //             break;
        //         }
        //         ExecutorResult::Error(e) => {
        //             logger::log(&format!("Error ({:?}), exiting...", e));
        //
        //             break;
        //         }
        //     }
        // }

        ClientResult::Ok(())
    }
}
