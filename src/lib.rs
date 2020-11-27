#![feature(static_nobundle)]
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;

pub mod client;
pub mod config;
mod utils;
