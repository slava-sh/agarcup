extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod config;
mod strategy;
mod interactor;

fn main() {
    interactor::Interactor::new().run();
}
