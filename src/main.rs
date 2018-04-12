#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "debug")]
#[macro_use]
extern crate log;
#[cfg(feature = "debug")]
extern crate fern;
#[cfg(feature = "debug")]
extern crate chrono;
#[cfg(feature = "debug")]
extern crate time;

mod command;
mod config;
mod interactor;
mod models;
mod strategy;
mod version;

fn main() {
    #[cfg(feature = "debug")] init_logging();
    interactor::run();
}

#[cfg(feature = "debug")]
fn init_logging() {
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
                .truncate(true)
                .open("/tmp/strategy.log")
                .expect("opening log file failed"),
        )
        .apply()
        .expect("logging initialization failed");
    debug!("hello");
}
