extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate fern;
extern crate chrono;
extern crate time;

#[macro_use]
mod utils;
mod models;
mod config;
mod strategy;
mod command;
mod interactor;

fn main() {
    if cfg!(feature = "debug") {
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} {} {} {}",
                    chrono::Local::now().format("%H:%M:%S%.6f"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .level(log::LevelFilter::Debug)
            .chain(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open("/tmp/strategy.log")
                    .expect("opening log file failed"),
            )
            .apply()
            .expect("logging initialization failed");
        debug!("hello");
    }
    interactor::run();
}
