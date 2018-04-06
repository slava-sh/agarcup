extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod utils;
mod models;
mod config;
mod strategy;
mod interactor;

fn main() {
    interactor::run();
}
