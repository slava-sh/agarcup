#[cfg(feature = "debug")]
#[macro_use]
extern crate log;
#[cfg(feature = "debug")]
extern crate fern;
#[cfg(feature = "debug")]
extern crate chrono;

extern crate my_strategy;

fn main() {
    #[cfg(feature = "debug")] init_logging();
    my_strategy::interactor::run();
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
