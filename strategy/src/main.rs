extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
mod config;
mod models;
mod strategy;
mod interactor;

fn main() {
    interactor::Interactor::new().run();
}
